mod pixel_art;
mod renderer;

pub use renderer::{SampleStrat, Transform};

use crate::render::{Renderer, Texture};
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
    Image(Arc<DynamicImage>),
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

enum CacheLoc {
    Atlas {
        // Index of the atlas this is cached in
        atlas_idx: usize,
        // Where in the cache texture this is
        aabr: Aabr<u16>,
    },
    Texture {
        index: usize,
    },
}
struct CachedDetails {
    location: CacheLoc,
    valid: bool,
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
        self.graphic_map.insert(id, graphic);

        // Remove from caches
        // Maybe make this more efficient if replace graphic is used more often
        let uses = self
            .cache_map
            .keys()
            .filter(|k| k.0 == id)
            .copied()
            .collect::<Vec<_>>();
        for p in uses {
            if let Some(details) = self.cache_map.get_mut(&p) {
                // Reuse allocation
                details.valid = false;
            }
        }
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

        let details = match self.cache_map.entry(key) {
            Entry::Occupied(details) => {
                let details = details.get();
                let (idx, aabr) = match details.location {
                    CacheLoc::Atlas {
                        atlas_idx, aabr, ..
                    } => (self.atlases[atlas_idx].1, aabr),
                    CacheLoc::Texture { index } => {
                        (index, Aabr {
                            min: Vec2::new(0, 0),
                            // Note texture should always match the cached dimensions
                            max: dims,
                        })
                    },
                };

                // Check if the cached version has been invalidated by replacing the underlying
                // graphic
                if !details.valid {
                    // Create image
                    let image = draw_graphic(&self.graphic_map, graphic_id, dims)?;
                    // Transfer to the gpu
                    upload_image(renderer, aabr, &self.textures[idx], &image);
                }

                return Some((transformed_aabr(aabr.map(|e| e as f64)), TexId(idx)));
            },
            Entry::Vacant(details) => details,
        };

        // Create image
        let image = draw_graphic(&self.graphic_map, graphic_id, dims)?;

        // Allocate space on the gpu
        // Check size of graphic
        // Graphics over a particular size are sent to their own textures
        let location = if Vec2::<i32>::from(self.atlases[0].0.size().to_tuple())
            .map(|e| e as u16)
            .map2(dims, |a, d| a as f32 * ATLAS_CUTTOFF_FRAC >= d as f32)
            .reduce_and()
        {
            // Fit into an atlas
            let mut loc = None;
            for (atlas_idx, (ref mut atlas, _)) in self.atlases.iter_mut().enumerate() {
                if let Some(rectangle) = atlas.allocate(size2(i32::from(dims.x), i32::from(dims.y)))
                {
                    let aabr = aabr_from_alloc_rect(rectangle);
                    loc = Some(CacheLoc::Atlas { atlas_idx, aabr });
                    break;
                }
            }

            match loc {
                Some(loc) => loc,
                // Create a new atlas
                None => {
                    let (mut atlas, texture) = create_atlas_texture(renderer);
                    let aabr = atlas
                        .allocate(size2(i32::from(dims.x), i32::from(dims.y)))
                        .map(aabr_from_alloc_rect)
                        .unwrap();
                    let tex_idx = self.textures.len();
                    let atlas_idx = self.atlases.len();
                    self.textures.push(texture);
                    self.atlases.push((atlas, tex_idx));
                    CacheLoc::Atlas { atlas_idx, aabr }
                },
            }
        } else {
            // Create a texture just for this
            let texture = renderer.create_dynamic_texture(dims).unwrap();
            let index = self.textures.len();
            self.textures.push(texture);
            CacheLoc::Texture { index }
        };

        let (idx, aabr) = match location {
            CacheLoc::Atlas {
                atlas_idx, aabr, ..
            } => (self.atlases[atlas_idx].1, aabr),
            CacheLoc::Texture { index } => {
                (index, Aabr {
                    min: Vec2::new(0, 0),
                    // Note texture should always match the cached dimensions
                    max: dims,
                })
            },
        };
        // Upload
        upload_image(renderer, aabr, &self.textures[idx], &image);
        // Insert into cached map
        details.insert(CachedDetails {
            location,
            valid: true,
        });

        Some((transformed_aabr(aabr.map(|e| e as f64)), TexId(idx)))
    }
}

// Draw a graphic at the specified dimensions
fn draw_graphic(graphic_map: &GraphicMap, graphic_id: Id, dims: Vec2<u16>) -> Option<RgbaImage> {
    match graphic_map.get(&graphic_id) {
        Some(Graphic::Blank) => None,
        // Render image at requested resolution
        // TODO: Use source aabr.
        Some(Graphic::Image(ref image)) => Some(resize_pixel_art(
            &image.to_rgba(),
            u32::from(dims.x),
            u32::from(dims.y),
        )),
        Some(Graphic::Voxel(ref segment, trans, sample_strat)) => Some(renderer::draw_vox(
            &segment,
            dims,
            trans.clone(),
            *sample_strat,
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

fn create_atlas_texture(renderer: &mut Renderer) -> (SimpleAtlasAllocator, Texture) {
    let (w, h) = renderer.get_resolution().into_tuple();

    let max_texture_size = renderer.max_texture_size();

    let size = Vec2::new(w, h).map(|e| {
        (e * GRAPHIC_CACHE_RELATIVE_SIZE)
            .max(512)
            .min(max_texture_size as u16)
    });

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
        &image.pixels().map(|p| p.0).collect::<Vec<[u8; 4]>>(),
    ) {
        warn!(?e, "Failed to update texture");
    }
}
