mod pixel_art;
pub mod renderer;

pub use renderer::{SampleStrat, Transform};

use crate::{
    render::{Renderer, Texture, UiTextureBindGroup},
    ui::KeyedJobs,
};
use common::{figure::Segment, slowjob::SlowJobPool};
use guillotiere::{size2, SimpleAtlasAllocator};
use hashbrown::{hash_map::Entry, HashMap};
use image::{DynamicImage, RgbaImage};
use slab::Slab;
use std::{hash::Hash, sync::Arc};
use tracing::{error, warn};
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
    // TODO: probably convert this type to `RgbaImage`.
    Image(Arc<DynamicImage>, Option<Rgba<f32>>),
    // Note: none of the users keep this Arc currently
    Voxel(Arc<Segment>, Transform, SampleStrat),
    Blank,
}

#[derive(Clone, Copy, Debug)]
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
const ATLAS_CUTOFF_FRAC: f32 = 0.2;
/// Multiplied by current window size
const GRAPHIC_CACHE_RELATIVE_SIZE: u32 = 1;

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub struct Id(u32);

// TODO these can become invalid when clearing the cache
#[derive(PartialEq, Eq, Hash, Copy, Clone)]
pub struct TexId(usize);

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
        textures: &Slab<(Texture, UiTextureBindGroup)>,
    ) -> (usize, bool, Aabr<u16>) {
        // NOTE: We don't accept images larger than u16::MAX (rejected in `cache_res`)
        // (and probably would not be able to create a texture this large).
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
                    max: textures[index].0.get_dimensions().xy().map(|e| e as u16),
                })
            },
            CachedDetails::Immutable { index } => {
                (index, true, Aabr {
                    min: Vec2::zero(),
                    // Note texture should always match the cached dimensions
                    max: textures[index].0.get_dimensions().xy().map(|e| e as u16),
                })
            },
        }
    }

    /// Attempt to invalidate this cache entry.
    /// If invalidation is not possible this returns the index of the texture to
    /// deallocate
    fn invalidate(&mut self) -> Result<(), usize> {
        match self {
            Self::Atlas { ref mut valid, .. } => {
                *valid = false;
                Ok(())
            },
            Self::Texture { ref mut valid, .. } => {
                *valid = false;
                Ok(())
            },
            Self::Immutable { index } => Err(*index),
        }
    }
}

// Caches graphics, only deallocates when changing screen resolution (completely
// cleared)
pub struct GraphicCache {
    // TODO replace with slotmap
    graphic_map: HashMap<Id, Graphic>,
    /// Next id to use when a new graphic is added
    next_id: u32,

    /// Atlases with the index of their texture in the textures vec
    atlases: Vec<(SimpleAtlasAllocator, usize)>,
    textures: Slab<(Texture, UiTextureBindGroup)>,
    /// The location and details of graphics cached on the GPU.
    ///
    /// Graphic::Voxel images include the dimensions they were rasterized at in
    /// the key. Other images are scaled as part of sampling them on the
    /// GPU.
    cache_map: HashMap<(Id, Option<Vec2<u16>>), CachedDetails>,

    keyed_jobs: KeyedJobs<(Id, Option<Vec2<u16>>), (RgbaImage, Option<Rgba<f32>>)>,
}
impl GraphicCache {
    pub fn new(renderer: &mut Renderer) -> Self {
        let (atlas, texture) = create_atlas_texture(renderer);

        Self {
            graphic_map: HashMap::default(),
            next_id: 0,
            atlases: vec![(atlas, 0)],
            textures: core::iter::once((0, texture)).collect(),
            cache_map: HashMap::default(),
            keyed_jobs: KeyedJobs::new("IMAGE_PROCESSING"),
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
        self.cache_map.retain(|&(key_id, _), details| {
            // If the entry does not reference id, or it does but we can successfully
            // invalidate, retain the entry; otherwise, discard this entry completely.
            key_id != id
                || details
                    .invalidate()
                    .map_err(|index| self.textures.remove(index))
                    .is_ok()
        });
    }

    pub fn get_graphic(&self, id: Id) -> Option<&Graphic> { self.graphic_map.get(&id) }

    /// Used to acquire textures for rendering
    pub fn get_tex(&self, id: TexId) -> &(Texture, UiTextureBindGroup) {
        self.textures.get(id.0).expect("Invalid TexId used")
    }

    pub fn get_graphic_dims(&self, (id, rot): (Id, Rotation)) -> Option<(u32, u32)> {
        use image::GenericImageView;
        self.get_graphic(id)
            .and_then(|graphic| match graphic {
                Graphic::Image(image, _) => Some(image.dimensions()),
                Graphic::Voxel(segment, _, _) => {
                    use common::vol::SizedVol;
                    let size = segment.size();
                    // TODO: HACK because they can be rotated arbitrarily, remove
                    // (and they can be rasterized at arbitrary resolution)
                    // (might need to return None here?)
                    Some((size.x, size.z))
                },
                Graphic::Blank => None,
            })
            .and_then(|(w, h)| match rot {
                Rotation::None | Rotation::Cw180 => Some((w, h)),
                Rotation::Cw90 | Rotation::Cw270 => Some((h, w)),
                // TODO: need dims for these?
                Rotation::SourceNorth | Rotation::TargetNorth => None,
            })
    }

    pub fn clear_cache(&mut self, renderer: &mut Renderer) {
        self.cache_map.clear();

        let (atlas, texture) = create_atlas_texture(renderer);
        self.atlases = vec![(atlas, 0)];
        self.textures = core::iter::once((0, texture)).collect();
    }

    /// Source rectangle should be from 0 to 1, and represents a bounding box
    /// for the source image of the graphic.
    pub fn cache_res(
        &mut self,
        renderer: &mut Renderer,
        pool: Option<&SlowJobPool>,
        graphic_id: Id,
        // TODO: if we aren't resizing here we can upload image earlier... (as long as this doesn't
        // lead to uploading too much unused stuff).
        requested_dims: Vec2<u16>,
        source: Aabr<f64>,
        rotation: Rotation,
    ) -> Option<((Aabr<f64>, Vec2<f32>), TexId)> {
        let requested_dims_upright = match rotation {
            // The image is stored on the GPU with no rotation, so we need to swap the dimensions
            // here to get the resolution that the image will be displayed at but re-oriented into
            // the "upright" space that the image is stored in and sampled from (this can be bit
            // confusing initially / hard to explain).
            Rotation::Cw90 | Rotation::Cw270 => requested_dims.yx(),
            Rotation::None | Rotation::Cw180 => requested_dims,
            Rotation::SourceNorth => requested_dims,
            Rotation::TargetNorth => requested_dims,
        };

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
            let size: Vec2<f64> = aabr.size().into();
            Aabr {
                min: size.mul_add(source.min, aabr.min),
                max: size.mul_add(source.max, aabr.min),
            }
        };
        // Apply all transformations.
        // TODO: Verify rotation is being applied correctly.
        let transformed_aabr = |aabr| {
            let scaled = scaled_aabr(aabr);
            // Calculate how many displayed pixels there are for each pixel in the source
            // image. We need this to calculate where to sample in the shader to
            // retain crisp pixel borders when scaling the image.
            // S-TODO: A bit hacky inserting this here, just to get things working initially
            let scale = requested_dims_upright.map2(
                Vec2::from(scaled.size()),
                |screen_pixels, sample_pixels: f64| screen_pixels as f32 / sample_pixels as f32,
            );
            let transformed = rotated_aabr(scaled);
            (transformed, scale)
        };

        let Self {
            textures,
            atlases,
            cache_map,
            graphic_map,
            ..
        } = self;

        let graphic = match graphic_map.get(&graphic_id) {
            Some(g) => g,
            None => {
                warn!(
                    ?graphic_id,
                    "A graphic was requested via an id which is not in use"
                );
                return None;
            },
        };

        let key = (
            graphic_id,
            // Dimensions only included in the key for voxel graphics which we rasterize at the
            // size that they will be displayed at (other images are scaled when sampling them on
            // the GPU).
            matches!(graphic, Graphic::Voxel { .. }).then(|| requested_dims_upright),
        );

        let details = match cache_map.entry(key) {
            Entry::Occupied(details) => {
                let details = details.get();
                let (idx, valid, aabr) = details.info(atlases, textures);

                // Check if the cached version has been invalidated by replacing the underlying
                // graphic
                if !valid {
                    // Create image
                    let (image, border) = prepare_graphic(
                        graphic,
                        graphic_id,
                        requested_dims_upright,
                        &mut self.keyed_jobs,
                        pool,
                    )?;
                    // If the cache location is invalid, we know the underlying texture is mutable,
                    // so we should be able to replace the graphic.  However, we still want to make
                    // sure that we are not reusing textures for images that specify a border
                    // color.
                    assert!(border.is_none());
                    // Transfer to the gpu
                    upload_image(renderer, aabr, &textures[idx].0, &image);
                }

                return Some((transformed_aabr(aabr.map(|e| e as f64)), TexId(idx)));
            },
            Entry::Vacant(details) => details,
        };

        // Construct image in an optional threadpool.
        let (image, border_color) = prepare_graphic(
            graphic,
            graphic_id,
            requested_dims_upright,
            &mut self.keyed_jobs,
            pool,
        )?;

        // Image sizes over u16::MAX are not supported (and we would probably not be
        // able to create a texture large enough to hold them on the GPU anyway)!
        let image_dims = match {
            let (x, y) = image.dimensions();
            (u16::try_from(x), u16::try_from(y))
        } {
            (Ok(x), Ok(y)) => Vec2::new(x, y),
            _ => {
                error!(
                    "Image dimensions greater than u16::MAX are not supported! Supplied image \
                     size: {:?}.",
                    image.dimensions()
                );
                return None;
            },
        };

        // Upload
        let atlas_size = atlas_size(renderer);

        // Allocate space on the gpu.
        //
        // Graphics with a border color.
        let location = if let Some(border_color) = border_color {
            // Create a new immutable texture.
            let texture = create_image(renderer, image, border_color);
            // NOTE: All mutations happen only after the upload succeeds!
            let index = textures.insert(texture);
            CachedDetails::Immutable { index }
        // Graphics over a particular size compared to the atlas size are sent
        // to their own textures. Here we check for ones under that
        // size.
        } else if atlas_size
            .map2(image_dims, |a, d| a as f32 * ATLAS_CUTOFF_FRAC >= d as f32)
            .reduce_and()
        {
            // Fit into an atlas
            let mut loc = None;
            for (atlas_idx, &mut (ref mut atlas, texture_idx)) in atlases.iter_mut().enumerate() {
                let clamped_dims = image_dims.map(|e| i32::from(e.max(1)));
                if let Some(rectangle) = atlas.allocate(size2(clamped_dims.x, clamped_dims.y)) {
                    let aabr = aabr_from_alloc_rect(rectangle);
                    loc = Some(CachedDetails::Atlas {
                        atlas_idx,
                        valid: true,
                        aabr,
                    });
                    upload_image(renderer, aabr, &textures[texture_idx].0, &image);
                    break;
                }
            }

            match loc {
                Some(loc) => loc,
                // Create a new atlas
                None => {
                    let (mut atlas, texture) = create_atlas_texture(renderer);
                    let clamped_dims = image_dims.map(|e| i32::from(e.max(1)));
                    let aabr = atlas
                        .allocate(size2(clamped_dims.x, clamped_dims.y))
                        .map(aabr_from_alloc_rect)
                        .unwrap();
                    // NOTE: All mutations happen only after the texture creation succeeds!
                    let tex_idx = textures.insert(texture);
                    let atlas_idx = atlases.len();
                    atlases.push((atlas, tex_idx));
                    upload_image(renderer, aabr, &textures[tex_idx].0, &image);
                    CachedDetails::Atlas {
                        atlas_idx,
                        valid: true,
                        aabr,
                    }
                },
            }
        } else {
            // Create a texture just for this
            let texture = {
                let tex = renderer.create_dynamic_texture(image_dims.map(u32::from));
                let bind = renderer.ui_bind_texture(&tex);
                (tex, bind)
            };
            // NOTE: All mutations happen only after the texture creation succeeds!
            let index = textures.insert(texture);
            upload_image(
                renderer,
                Aabr {
                    min: Vec2::zero(),
                    // Note texture should always match the cached dimensions
                    max: image_dims,
                },
                &textures[index].0,
                &image,
            );
            CachedDetails::Texture { index, valid: true }
        };

        // Extract information from cache entry.
        let (idx, _, aabr) = location.info(atlases, textures);

        // Insert into cached map
        details.insert(location);

        Some((transformed_aabr(aabr.map(|e| e as f64)), TexId(idx)))
    }
}

/// Prepare the graphic into the form that will be uploaded to the GPU.
///
/// For voxel graphics, draws the graphic at the specified dimensions.
///
/// Also pre-multiplies alpha in images so they can be linearly filtered on the
/// GPU.
fn prepare_graphic(
    graphic: &Graphic,
    graphic_id: Id,
    dims: Vec2<u16>,
    keyed_jobs: &mut KeyedJobs<(Id, Option<Vec2<u16>>), (RgbaImage, Option<Rgba<f32>>)>,
    pool: Option<&SlowJobPool>,
) -> Option<(RgbaImage, Option<Rgba<f32>>)> {
    match graphic {
        // Short-circuit spawning a job on the threadpool for blank graphics
        Graphic::Blank => None,
        // Dimensions are only included in the key for Graphic::Voxel since otherwise we will
        // resize on the GPU.
        Graphic::Image(image, border_color) => keyed_jobs
            .spawn(pool, (graphic_id, None), || {
                let image = Arc::clone(image);
                let border_color = *border_color;
                move |_| {
                    // Image will be rescaled when sampling from it on the GPU so we don't
                    // need to resize it here.
                    let mut image = image.to_rgba8();
                    // TODO: could potentially do this when loading the image and for voxel
                    // images maybe at some point in the `draw_vox` processing. Or we could
                    // push it in the other direction and do conversion on the GPU.
                    premultiply_alpha(&mut image);
                    (image, border_color)
                }
            })
            .map(|(_, v)| v),
        Graphic::Voxel(segment, trans, sample_strat) => keyed_jobs
            .spawn(pool, (graphic_id, Some(dims)), || {
                let segment = Arc::clone(segment);
                let (trans, sample_strat) = (*trans, *sample_strat);
                move |_| {
                    // Render voxel model at requested resolution
                    let mut image = renderer::draw_vox(&segment, dims, trans, sample_strat);
                    premultiply_alpha(&mut image);
                    (image, None)
                }
            })
            .map(|(_, v)| v),
    }
}

fn atlas_size(renderer: &Renderer) -> Vec2<u32> {
    let max_texture_size = renderer.max_texture_size();

    renderer
        .resolution()
        .map(|e| (e * GRAPHIC_CACHE_RELATIVE_SIZE).clamp(512, max_texture_size))
}

fn create_atlas_texture(
    renderer: &mut Renderer,
) -> (SimpleAtlasAllocator, (Texture, UiTextureBindGroup)) {
    let size = atlas_size(renderer);
    // Note: here we assume the max texture size is under i32::MAX.
    let atlas = SimpleAtlasAllocator::new(size2(size.x as i32, size.y as i32));
    let texture = {
        let tex = renderer.create_dynamic_texture(size);
        let bind = renderer.ui_bind_texture(&tex);
        (tex, bind)
    };

    (atlas, texture)
}

fn aabr_from_alloc_rect(rect: guillotiere::Rectangle) -> Aabr<u16> {
    let (min, max) = (rect.min, rect.max);
    // Note: here we assume the max texture size (and thus the maximum size of the
    // atlas) is under `u16::MAX`.
    Aabr {
        min: Vec2::new(min.x as u16, min.y as u16),
        max: Vec2::new(max.x as u16, max.y as u16),
    }
}

fn upload_image(renderer: &mut Renderer, aabr: Aabr<u16>, tex: &Texture, image: &RgbaImage) {
    let aabr = aabr.map(u32::from);
    let offset = aabr.min.into_array();
    let size = aabr.size().into_array();
    renderer.update_texture(
        tex,
        offset,
        size,
        // NOTE: Rgba texture, so each pixel is 4 bytes, ergo this cannot fail.
        // We make the cast parameters explicit for clarity.
        bytemuck::cast_slice::<u8, [u8; 4]>(image),
    );
}

fn create_image(
    renderer: &mut Renderer,
    image: RgbaImage,
    _border_color: Rgba<f32>, // See TODO below
) -> (Texture, UiTextureBindGroup) {
    let tex = renderer
        .create_texture(
            &DynamicImage::ImageRgba8(image),
            Some(wgpu::FilterMode::Linear),
            // TODO: either use the desktop only border color or just emulate this
            // Some(border_color.into_array().into()),
            Some(wgpu::AddressMode::ClampToBorder),
        )
        .expect("create_texture only panics if non ImageRbga8 is passed");
    let bind = renderer.ui_bind_texture(&tex);

    (tex, bind)
}

fn premultiply_alpha(image: &mut RgbaImage) {
    use fast_srgb8::{f32x4_to_srgb8, srgb8_to_f32};
    // TODO: Apparently it is possible for ImageBuffer raw vec to have more pixels
    // than the dimensions of the actual image (I don't think we actually have
    // this occuring but we should probably fix other spots that use the raw
    // buffer). See:
    // https://github.com/image-rs/image/blob/a1ce569afd476e881acafdf9e7a5bce294d0db9a/src/buffer.rs#L664
    let dims = image.dimensions();
    let image_buffer_len = dims.0 as usize * dims.1 as usize * 4;
    let (arrays, end) = image[..image_buffer_len].as_chunks_mut::<{ 4 * 4 }>();
    // Rgba8 has 4 bytes per pixel they should be no remainder when dividing by 4.
    let (end, _) = end.as_chunks_mut::<4>();
    end.iter_mut().for_each(|pixel| {
        let alpha = pixel[3];
        if alpha == 0 {
            *pixel = [0; 4];
        } else if alpha != 255 {
            let linear_alpha = alpha as f32 / 255.0;
            let [r, g, b] = core::array::from_fn(|i| srgb8_to_f32(pixel[i]) * linear_alpha);
            let srgb8 = f32x4_to_srgb8([r, g, b, 0.0]);
            (pixel[0], pixel[1], pixel[3]) = (srgb8[0], srgb8[1], srgb8[3]);
        }
    });
    arrays.iter_mut().for_each(|pixelx4| {
        use core::simd::{f32x4, u8x4, Simd};
        let alpha = Simd::from_array([pixelx4[3], pixelx4[7], pixelx4[11], pixelx4[15]]);
        if alpha == Simd::splat(0) {
            *pixelx4 = [0; 16];
        } else if alpha != Simd::splat(255) {
            let linear_simd = |array: [u8; 4]| Simd::from_array(array.map(srgb8_to_f32));
            // Pack rgb components from the 4th pixel into the the last position for each of
            // the other 3 pixels.
            let a = linear_simd([pixelx4[0], pixelx4[1], pixelx4[2], pixelx4[12]]);
            let b = linear_simd([pixelx4[4], pixelx4[5], pixelx4[6], pixelx4[13]]);
            let c = linear_simd([pixelx4[8], pixelx4[9], pixelx4[10], pixelx4[14]]);
            let linear_alpha = alpha.cast::<f32>() * Simd::splat(1.0 / 255.0);

            // Multiply by alpha and then convert back into srgb8.
            let premultiply = |x: f32x4, i| {
                let mut a = f32x4::splat(linear_alpha[i]);
                a[3] = linear_alpha[3];
                u8x4::from_array(f32x4_to_srgb8((x * a).to_array()))
            };
            let pa = premultiply(a, 0);
            let pb = premultiply(b, 1);
            let pc = premultiply(c, 2);

            (pixelx4[0], pixelx4[1], pixelx4[2]) = (pa[0], pa[1], pa[2]);
            (pixelx4[4], pixelx4[5], pixelx4[6]) = (pb[0], pb[1], pb[2]);
            (pixelx4[8], pixelx4[9], pixelx4[10]) = (pc[0], pc[1], pc[2]);
            (pixelx4[12], pixelx4[13], pixelx4[14]) = (pa[3], pb[3], pc[3]);
        }
    })
}
