mod pixel_art;
mod renderer;

pub use renderer::{SampleStrat, Transform};

use crate::render::{RenderError, Renderer, Texture};
use common::figure::Segment;
use guillotiere::{size2, SimpleAtlasAllocator};
use hashbrown::{hash_map::Entry, HashMap};
use image::{DynamicImage, RgbaImage};
use pixel_art::resize_pixel_art;
use std::sync::Arc;
use tracing::warn;
use vek::*;

#[derive(Clone)]
pub enum Graphic {
    /// NOTE: The second argument is an optional border color.  If this is set,
    /// we force the image into its own texture and use the border color
    /// whenever we sample beyond the image extent. This can be useful, for
    /// example, for the map and minimap, which both rotate and may be
    /// non-square (meaning if we want to display the whole map and render to a
    /// square, we may render out of bounds unless we perform proper
    /// clipping).
    Image(Arc<DynamicImage>, Option<Rgba<f32>>),
    // Note: none of the users keep this Arc currently
    Voxel(Arc<Segment>, Transform, SampleStrat),
    Blank,
}

#[derive(Clone, Copy)]
pub enum Rotation {
    None,
    Cw90,
    Cw180,
    Cw270,
    /// Orientation of source rectangle that always faces true north.
    /// Simple hack to get around Conrod not having space for proper
    /// rotation data (though it should be possible to add in other ways).
    SourceNorth,
    /// Orientation of target rectangle that always faces true north.
    /// Simple hack to get around Conrod not having space for proper
    /// rotation data (though it should be possible to add in other ways).
    TargetNorth,
}

/// Images larger than this are stored in individual textures
/// Fraction of the total graphic cache size
const ATLAS_CUTTOFF_FRAC: f32 = 0.2;
/// Multiplied by current window size
const GRAPHIC_CACHE_RELATIVE_SIZE: u16 = 1;

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub struct Id(u32);

// TODO these can become invalid when clearing the cache
#[derive(PartialEq, Eq, Hash, Copy, Clone)]
pub struct TexId(usize);

type Parameters = (Id, Vec2<u16>);
type GraphicMap = HashMap<Id, Graphic>;

enum CachedDetails {
    Atlas {
        // Index of the atlas this is cached in
        atlas_idx: usize,
        // Whether this texture is valid.
        valid: bool,
        // Where in the cache texture this is
        aabr: Aabr<u16>,
    },
    Texture {
        // Index of the (unique, non-atlas) texture this is cached in.
        index: usize,
        // Whether this texture is valid.
        valid: bool,
    },
    Immutable {
        // Index of the (unique, immutable, non-atlas) texture this is cached in.
        index: usize,
    },
}

impl CachedDetails {
    /// Get information about this cache entry: texture index,
    /// whether the entry is valid, and its bounding box in the referenced
    /// texture.
    fn info(
        &self,
        atlases: &[(SimpleAtlasAllocator, usize)],
        dims: Vec2<u16>,
    ) -> (usize, bool, Aabr<u16>) {
        match *self {
            CachedDetails::Atlas {
                atlas_idx,
                valid,
                aabr,
            } => (atlases[atlas_idx].1, valid, aabr),
            CachedDetails::Texture { index, valid } => {
                (index, valid, Aabr {
                    min: Vec2::zero(),
                    // Note texture should always match the cached dimensions
                    max: dims,
                })
            },
            CachedDetails::Immutable { index } => {
                (index, true, Aabr {
                    min: Vec2::zero(),
                    // Note texture should always match the cached dimensions
                    max: dims,
                })
            },
        }
    }

    /// Attempt to invalidate this cache entry.
    pub fn invalidate(&mut self) -> Result<(), ()> {
        match self {
            Self::Atlas { ref mut valid, .. } => {
                *valid = false;
                Ok(())
            },
            Self::Texture { ref mut valid, .. } => {
                *valid = false;
                Ok(())
            },
            Self::Immutable { .. } => Err(()),
        }
    }
}

// Caches graphics, only deallocates when changing screen resolution (completely
// cleared)
pub struct GraphicCache {
    graphic_map: GraphicMap,
    // Next id to use when a new graphic is added
    next_id: u32,

    // Atlases with the index of their texture in the textures vec
    atlases: Vec<(SimpleAtlasAllocator, usize)>,
    textures: Vec<Texture>,
    // Stores the location of graphics rendered at a particular resolution and cached on the cpu
    cache_map: HashMap<Parameters, CachedDetails>,
}
impl GraphicCache {
    pub fn new(renderer: &mut Renderer) -> Self {
        let (atlas, texture) = create_atlas_texture(renderer);

        Self {
            graphic_map: HashMap::default(),
            next_id: 0,
            atlases: vec![(atlas, 0)],
            textures: vec![texture],
            cache_map: HashMap::default(),
        }
    }

    pub fn add_graphic(&mut self, graphic: Graphic) -> Id {
        let id = self.next_id;
        self.next_id = id.wrapping_add(1);

        let id = Id(id);
        self.graphic_map.insert(id, graphic);

        id
    }

    pub fn replace_graphic(&mut self, id: Id, graphic: Graphic) {
        if self.graphic_map.insert(id, graphic).is_none() {
            // This was not an update, so no need to search for keys.
            return;
        }

        // Remove from caches
        // Maybe make this more efficient if replace graphic is used more often
        self.cache_map.retain(|&(key_id, _key_dims), details| {
            // If the entry does not reference id, or it does but we can successfully
            // invalidate, retain the entry; otherwise, discard this entry completely.
            key_id != id || details.invalidate().is_ok()
        });
    }

    pub fn get_graphic(&self, id: Id) -> Option<&Graphic> { self.graphic_map.get(&id) }

    /// Used to aquire textures for rendering
    pub fn get_tex(&self, id: TexId) -> &Texture {
        self.textures.get(id.0).expect("Invalid TexId used")
    }

    pub fn clear_cache(&mut self, renderer: &mut Renderer) {
        self.cache_map.clear();

        let (atlas, texture) = create_atlas_texture(renderer);
        self.atlases = vec![(atlas, 0)];
        self.textures = vec![texture];
    }

    /// Source rectangle should be from 0 to 1, and represents a bounding box
    /// for the source image of the graphic.
    pub fn cache_res(
        &mut self,
        renderer: &mut Renderer,
        graphic_id: Id,
        dims: Vec2<u16>,
        source: Aabr<f64>,
        rotation: Rotation,
    ) -> Option<(Aabr<f64>, TexId)> {
        let dims = match rotation {
            Rotation::Cw90 | Rotation::Cw270 => Vec2::new(dims.y, dims.x),
            Rotation::None | Rotation::Cw180 => dims,
            Rotation::SourceNorth => dims,
            Rotation::TargetNorth => dims,
        };
        let key = (graphic_id, dims);

        // Rotate aabr according to requested rotation.
        let rotated_aabr = |Aabr { min, max }| match rotation {
            Rotation::None | Rotation::SourceNorth | Rotation::TargetNorth => Aabr { min, max },
            Rotation::Cw90 => Aabr {
                min: Vec2::new(min.x, max.y),
                max: Vec2::new(max.x, min.y),
            },
            Rotation::Cw180 => Aabr { min: max, max: min },
            Rotation::Cw270 => Aabr {
                min: Vec2::new(max.x, min.y),
                max: Vec2::new(min.x, max.y),
            },
        };
        // Scale aabr according to provided source rectangle.
        let scaled_aabr = |aabr: Aabr<_>| {
            let size: Vec2<_> = aabr.size().into();
            Aabr {
                min: size.mul_add(source.min, aabr.min),
                max: size.mul_add(source.max, aabr.min),
            }
        };
        // Apply all transformations.
        // TODO: Verify rotation is being applied correctly.
        let transformed_aabr = |aabr| rotated_aabr(scaled_aabr(aabr));

        let Self {
            textures,
            atlases,
            cache_map,
            graphic_map,
            ..
        } = self;

        let details = match cache_map.entry(key) {
            Entry::Occupied(details) => {
                let details = details.get();
                let (idx, valid, aabr) = details.info(atlases, dims);

                // Check if the cached version has been invalidated by replacing the underlying
                // graphic
                if !valid {
                    // Create image
                    let (image, border) = draw_graphic(graphic_map, graphic_id, dims)?;
                    // If the cache location is invalid, we know the underlying texture is mutable,
                    // so we should be able to replace the graphic.  However, we still want to make
                    // sure that we are not reusing textures for images that specify a border
                    // color.
                    assert!(border.is_none());
                    // Transfer to the gpu
                    upload_image(renderer, aabr, &textures[idx], &image);
                }

                return Some((transformed_aabr(aabr.map(|e| e as f64)), TexId(idx)));
            },
            Entry::Vacant(details) => details,
        };

        // Construct image
        let (image, border_color) = draw_graphic(graphic_map, graphic_id, dims)?;

        // Upload
        let atlas_size = atlas_size(renderer);

        // Allocate space on the gpu
        // Check size of graphic
        // Graphics over a particular size are sent to their own textures
        let location = if let Some(border_color) = border_color {
            // Create a new immutable texture.
            let texture = create_image(renderer, image, border_color).unwrap();
            // NOTE: All mutations happen only after the upload succeeds!
            let index = textures.len();
            textures.push(texture);
            CachedDetails::Immutable { index }
        } else if atlas_size
            .map2(dims, |a, d| a as f32 * ATLAS_CUTTOFF_FRAC >= d as f32)
            .reduce_and()
        {
            // Fit into an atlas
            let mut loc = None;
            for (atlas_idx, &mut (ref mut atlas, texture_idx)) in atlases.iter_mut().enumerate() {
                let dims = dims.map(|e| e.max(1));
                if let Some(rectangle) = atlas.allocate(size2(i32::from(dims.x), i32::from(dims.y)))
                {
                    let aabr = aabr_from_alloc_rect(rectangle);
                    loc = Some(CachedDetails::Atlas {
                        atlas_idx,
                        valid: true,
                        aabr,
                    });
                    upload_image(renderer, aabr, &textures[texture_idx], &image);
                    break;
                }
            }

            match loc {
                Some(loc) => loc,
                // Create a new atlas
                None => {
                    let (mut atlas, texture) = create_atlas_texture(renderer);
                    let dims = dims.map(|e| e.max(1));
                    let aabr = atlas
                        .allocate(size2(i32::from(dims.x), i32::from(dims.y)))
                        .map(aabr_from_alloc_rect)
                        .unwrap();
                    // NOTE: All mutations happen only after the texture creation succeeds!
                    let tex_idx = textures.len();
                    let atlas_idx = atlases.len();
                    textures.push(texture);
                    atlases.push((atlas, tex_idx));
                    upload_image(renderer, aabr, &textures[tex_idx], &image);
                    CachedDetails::Atlas {
                        atlas_idx,
                        valid: true,
                        aabr,
                    }
                },
            }
        } else {
            // Create a texture just for this
            let texture = renderer.create_dynamic_texture(dims).unwrap();
            // NOTE: All mutations happen only after the texture creation succeeds!
            let index = textures.len();
            textures.push(texture);
            upload_image(
                renderer,
                Aabr {
                    min: Vec2::zero(),
                    // Note texture should always match the cached dimensions
                    max: dims,
                },
                &textures[index],
                &image,
            );
            CachedDetails::Texture { index, valid: true }
        };

        // Extract information from cache entry.
        let (idx, _, aabr) = location.info(atlases, dims);

        // Insert into cached map
        details.insert(location);

        Some((transformed_aabr(aabr.map(|e| e as f64)), TexId(idx)))
    }
}

// Draw a graphic at the specified dimensions
fn draw_graphic(
    graphic_map: &GraphicMap,
    graphic_id: Id,
    dims: Vec2<u16>,
) -> Option<(RgbaImage, Option<Rgba<f32>>)> {
    match graphic_map.get(&graphic_id) {
        Some(Graphic::Blank) => None,
        // Render image at requested resolution
        // TODO: Use source aabr.
        Some(&Graphic::Image(ref image, border_color)) => Some((
            resize_pixel_art(&image.to_rgba(), u32::from(dims.x), u32::from(dims.y)),
            border_color,
        )),
        Some(Graphic::Voxel(ref segment, trans, sample_strat)) => Some((
            renderer::draw_vox(&segment, dims, trans.clone(), *sample_strat),
            None,
        )),
        None => {
            warn!(
                ?graphic_id,
                "A graphic was requested via an id which is not in use"
            );
            None
        },
    }
}

fn atlas_size(renderer: &Renderer) -> Vec2<u16> {
    let max_texture_size = renderer.max_texture_size();

    renderer.get_resolution().map(|e| {
        (e * GRAPHIC_CACHE_RELATIVE_SIZE)
            .max(512)
            .min(max_texture_size)
    })
}

fn create_atlas_texture(renderer: &mut Renderer) -> (SimpleAtlasAllocator, Texture) {
    let size = atlas_size(renderer);
    let atlas = SimpleAtlasAllocator::new(size2(i32::from(size.x), i32::from(size.y)));
    let texture = renderer.create_dynamic_texture(size).unwrap();
    (atlas, texture)
}

fn aabr_from_alloc_rect(rect: guillotiere::Rectangle) -> Aabr<u16> {
    let (min, max) = (rect.min, rect.max);
    Aabr {
        min: Vec2::new(min.x as u16, min.y as u16),
        max: Vec2::new(max.x as u16, max.y as u16),
    }
}

fn upload_image(renderer: &mut Renderer, aabr: Aabr<u16>, tex: &Texture, image: &RgbaImage) {
    let offset = aabr.min.into_array();
    let size = aabr.size().into_array();
    if let Err(e) = renderer.update_texture(
        tex,
        offset,
        size,
        // NOTE: Rgba texture, so each pixel is 4 bytes, ergo this cannot fail.
        // We make the cast parameters explicit for clarity.
        gfx::memory::cast_slice::<u8, [u8; 4]>(&image),
    ) {
        warn!(?e, "Failed to update texture");
    }
}

fn create_image(
    renderer: &mut Renderer,
    image: RgbaImage,
    border_color: Rgba<f32>,
) -> Result<Texture, RenderError> {
    renderer.create_texture(
        &DynamicImage::ImageRgba8(image),
        None,
        Some(gfx::texture::WrapMode::Border),
        Some(border_color.into_array().into()),
    )
}
