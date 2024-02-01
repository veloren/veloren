mod pixel_art;
pub mod renderer;

pub use renderer::{SampleStrat, Transform};

use crate::{
    render::{Renderer, Texture, UiTextureBindGroup, UiUploadBatchId},
    ui::KeyedJobs,
};
use common::{figure::Segment, slowjob::SlowJobPool};
use common_base::prof_span;
use guillotiere::{size2, SimpleAtlasAllocator};
use hashbrown::{hash_map::Entry, HashMap};
use image::{DynamicImage, RgbaImage};
use slab::Slab;
use std::{borrow::Cow, hash::Hash, sync::Arc};
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
    // TODO: Re-evaluate whether we need this (especially outside conrod context)
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
        // Index of the atlas this is cached in.
        atlas_idx: usize,
        // Whether this texture is valid.
        valid: bool,
        // Where in the cache texture this is.
        aabr: Aabr<u16>,
    },
    Texture {
        // Index of the (unique, non-atlas) texture this is cached in.
        index: usize,
        // Whether this texture is valid.
        valid: bool,
    },
}

impl CachedDetails {
    /// Get information about this cache entry: texture index,
    /// whether the entry is valid, and its bounding box in the referenced
    /// texture.
    fn info(
        &self,
        atlases: &[(SimpleAtlasAllocator, usize)],
        textures: &Slab<(Arc<Texture>, UiTextureBindGroup, UiUploadBatchId)>,
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
                    // NOTE (as cast): We don't accept images larger than u16::MAX (rejected in
                    // `cache_res`) (and probably would not be able to create a texture this
                    // large).
                    //
                    // Note texture should always match the cached dimensions.
                    max: textures[index].0.get_dimensions().xy().map(|e| e as u16),
                })
            },
        }
    }

    /// Invalidate this cache entry.
    fn invalidate(&mut self) {
        match self {
            Self::Atlas { ref mut valid, .. } => {
                *valid = false;
            },
            Self::Texture { ref mut valid, .. } => {
                *valid = false;
            },
        }
    }

    fn set_valid(&mut self) {
        match self {
            Self::Atlas { ref mut valid, .. } => {
                *valid = true;
            },
            Self::Texture { ref mut valid, .. } => {
                *valid = true;
            },
        }
    }
}

/// Requirements that a particular graphic has with respect to the atlas
/// allocation or independent texture it will be stored in.
///
/// If this matches between an old graphic and a new one which is replacing it,
/// we can reuse any of the corresponding locations where it is cached in
/// textures on the GPU. That is we can invalidate such textures and upload the
/// new graphic there, rather than needing to allocate a new texture (or new
/// location in an atlas).
#[derive(PartialEq)]
enum TextureRequirements {
    /// These are uploaded to the GPU in the original resolution of the image
    /// supplied by the `Graphic` and any scaling is done during sampling in
    /// the UI fragment shader.
    Fixed {
        size: Vec2<u16>,
        /// Graphics with a border color specified are placed into their own
        /// individual textures so that the border color can be set
        /// there. (Note: this is partially a theoretical description as
        /// border color options are limited in the current graphics API).
        border_color: Option<Rgba<f32>>,
    },
    /// These are rasterized to the exact resolution that they will be displayed
    /// at and then uploaded to the GPU. This corresponds to
    /// `Graphic::Voxel`. There may be multiple copies on the GPU if
    /// different resolutions are requested.
    ///
    /// It is expected that the requested sizes will generally not differ when
    /// switching out a graphic. Thus, dependent cached depdendent should
    /// always be invalidated since those cached locations will be reusable
    /// if the requested size is the same.
    Dependent,
}

/// These solely determine how a place in an atlas will be found or how a
/// texture will be created to place the image for a graphic.
struct TextureParameters {
    size: Vec2<u16>,
    border_color: Option<Rgba<f32>>,
}

/// Key used to refer to an instance of a graphic that has been uploaded to the
/// GPU.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct CacheKey {
    graphic_id: Id,
    /// This is `Some` for `TextureRequirements::Dependent`.
    size: Option<Vec2<u16>>,
}

impl TextureRequirements {
    fn from_graphic(graphic: &Graphic) -> Option<Self> {
        match graphic {
            Graphic::Image(image, border_color) => {
                // Image sizes over u16::MAX are not supported (and we would probably not be
                // able to create a texture large enough to hold them on the GPU anyway)!
                let image_dims = match (u16::try_from(image.width()), u16::try_from(image.height()))
                {
                    (Ok(x), Ok(y)) if x != 0 && y != 0 => Vec2::new(x, y),
                    _ => {
                        error!(
                            "Image dimensions greater than u16::MAX are not supported! Supplied \
                             image size: ({}, {}).",
                            image.width(),
                            image.height(),
                        );
                        // TODO: reasonable to return None on this error case? We could potentially
                        // validate images sizes on add_graphic/replace_graphic?
                        return None;
                    },
                };

                Some(Self::Fixed {
                    size: image_dims,
                    border_color: *border_color,
                })
            },
            Graphic::Voxel(_, _, _) => Some(Self::Dependent),
            Graphic::Blank => None,
        }
    }

    #[allow(clippy::wrong_self_convention)] // type is spiritually Copy
    fn to_key_and_tex_parameters(
        self,
        graphic_id: Id,
        requested_size: Vec2<u16>,
    ) -> (CacheKey, TextureParameters) {
        // NOTE: Any external parameters which influence the value of the returned
        // `TextureParameters` must be included in the `CacheKey`. Otherwise,
        // invalidation and subsequent re-use of cache locations based on the
        // value of `self` would be wrong.
        let (size, border_color, key_size) = match self {
            Self::Fixed { size, border_color } => (size, border_color, None),
            Self::Dependent => (requested_size, None, Some(requested_size)),
        };
        (
            CacheKey {
                graphic_id,
                size: key_size,
            },
            TextureParameters { size, border_color },
        )
    }
}

// Caches graphics, only deallocates when changing screen resolution (completely
// cleared)
pub struct GraphicCache {
    // TODO replace with slotmap
    graphic_map: HashMap<Id, Graphic>,
    /// Next id to use when a new graphic is added
    next_id: u32,

    /// Atlases with the index of their texture in the textures slab.
    atlases: Vec<(SimpleAtlasAllocator, usize)>,
    /// Third tuple element is a list of pending premultiply + upload operations
    /// for this frame. The purpose of this is to collect all the operations
    /// together so that a single renderpass is performed for each target
    /// texture.
    textures: Slab<(Arc<Texture>, UiTextureBindGroup, UiUploadBatchId)>,
    /// The location and details of graphics cached on the GPU.
    ///
    /// Graphic::Voxel images include the dimensions they were rasterized at in
    /// the key. Other images are scaled as part of sampling them on the
    /// GPU.
    cache_map: HashMap<CacheKey, CachedDetails>,

    keyed_jobs: KeyedJobs<CacheKey, RgbaImage>,
}

impl GraphicCache {
    pub fn new(renderer: &mut Renderer) -> Self {
        let (atlas, (tex, bind)) = create_atlas_texture(renderer);

        let mut textures = Slab::new();
        let tex_id = textures.insert((tex, bind, UiUploadBatchId::default()));

        Self {
            graphic_map: HashMap::default(),
            next_id: 0,
            atlases: vec![(atlas, tex_id)],
            textures,
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
        let (old, new) = match self.graphic_map.entry(id) {
            Entry::Occupied(o) => {
                let slot_mut = o.into_mut();
                let old = core::mem::replace(slot_mut, graphic);
                (old, slot_mut)
            },
            Entry::Vacant(v) => {
                // This was not an update, so no need to cleanup caches.
                v.insert(graphic);
                return;
            },
        };

        let old_requirements = TextureRequirements::from_graphic(&old);
        let new_requirements = TextureRequirements::from_graphic(new);
        let should_invalidate = old_requirements == new_requirements && old_requirements.is_some();

        // Invalidate if possible or remove from caches.
        // Maybe make this more efficient if replace graphic is used more often
        // (especially since we should know the exact key for non-voxel
        // graphics).
        //
        // NOTE: at the time of writing, replace_graphic is only used for voxel minimap
        // updates and item image reloading.
        if should_invalidate {
            self.cache_map.iter_mut().for_each(|(key, details)| {
                if key.graphic_id == id {
                    details.invalidate();
                }
            });
        } else {
            self.cache_map.drain_filter(|key, details| {
                if key.graphic_id == id {
                    match details {
                        // NOTE: if replace_graphic is used continously for small images (i.e.
                        // images placed into an atlas) of different sizes, that can use up our
                        // atlas space since spots in the atlas can't be reused. (this scenario is
                        // now possible with scaling being done during sampling rather than placing
                        // resized version into the atlas). This is expected to not occur in all
                        // pratical cases we plan to support here (i.e. the size of the replacement
                        // image will always be the same).
                        CachedDetails::Atlas { .. } => {},
                        CachedDetails::Texture { index, .. } => {
                            self.textures.remove(*index);
                        },
                    };
                    true
                } else {
                    false
                }
            });
        }
    }

    pub fn get_graphic(&self, id: Id) -> Option<&Graphic> { self.graphic_map.get(&id) }

    /// Used to acquire textures for rendering
    pub fn get_tex(&self, id: TexId) -> (&Texture, &UiTextureBindGroup) {
        let (tex, bind, _upload_batch) = self.textures.get(id.0).expect("Invalid TexId used");
        (tex, bind)
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

        let (atlas, (tex, bind)) = create_atlas_texture(renderer);
        let mut textures = Slab::new();
        let tex_id = textures.insert((tex, bind, UiUploadBatchId::default()));
        self.atlases = vec![(atlas, tex_id)];
        self.textures = textures;
    }

    /// Source rectangle should be from 0 to 1, and represents a bounding box
    /// for the source image of the graphic.
    ///
    /// # Panics
    ///
    /// Panics if one of the lengths in requested_dims is zero.
    pub fn cache_res(
        &mut self,
        renderer: &mut Renderer,
        pool: Option<&SlowJobPool>,
        graphic_id: Id,
        // TODO: if we aren't resizing here we can potentially upload the image earlier... (as long
        // as this doesn't lead to uploading too much unused stuff). (currently not sure whether it
        // would be an overall gain to pursue this.)
        requested_dims: Vec2<u16>,
        source: Aabr<f64>,
        rotation: Rotation,
    ) -> Option<((Aabr<f64>, Vec2<f32>), TexId)> {
        assert!(requested_dims.map(|e| e != 0).reduce_and());
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
        let transformed_aabr_and_scale = |aabr| {
            let scaled = scaled_aabr(aabr);
            // Calculate how many displayed pixels there are for each pixel in the source
            // image. We need this to calculate where to sample in the shader to
            // retain crisp pixel borders when scaling the image.
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

        let requirements = TextureRequirements::from_graphic(graphic)?;
        let (key, texture_parameters) =
            requirements.to_key_and_tex_parameters(graphic_id, requested_dims_upright);

        let details = match cache_map.entry(key) {
            Entry::Occupied(mut details) => {
                let details = details.get_mut();
                let (idx, valid, aabr) = details.info(atlases, textures);

                // Check if the cached version has been invalidated by replacing the underlying
                // graphic
                if !valid {
                    // Create image
                    let (image, gpu_premul) = prepare_graphic(
                        graphic,
                        key,
                        requested_dims_upright,
                        &mut self.keyed_jobs,
                        pool,
                    )?;
                    // Ensure we don't have any bugs causing the size used to determine if the
                    // cached version is reusable to not match the size of the image produced by
                    // prepare_graphic.
                    assert_eq!(
                        image.dimensions(),
                        texture_parameters.size.map(u32::from).into_tuple()
                    );
                    // Transfer to the gpu
                    let (ref texture, _, ref mut upload_batch) = &mut textures[idx];
                    upload_image(renderer, texture, upload_batch, &image, aabr, gpu_premul);
                    details.set_valid();
                }

                return Some((
                    transformed_aabr_and_scale(aabr.map(|e| e as f64)),
                    TexId(idx),
                ));
            },
            Entry::Vacant(details) => details,
        };

        // Construct image in an optional threadpool.
        let (image, gpu_premul) = prepare_graphic(
            graphic,
            key,
            requested_dims_upright,
            &mut self.keyed_jobs,
            pool,
        )?;
        // Assert dimensions of image from `prepare_graphic` are as expected!
        assert_eq!(
            image.dimensions(),
            texture_parameters.size.map(u32::from).into_tuple()
        );
        // Image dimensions in the format used by the allocator crate.
        let image_dims_size2d = size2(
            i32::from(texture_parameters.size.x),
            i32::from(texture_parameters.size.y),
        );

        // Now we allocate space on the gpu (either in an atlas or an independent
        // texture) and upload the image to that location.

        let atlas_size = atlas_size(renderer);
        // Graphics that request a border color or which are over a particular size
        // compared to the atlas size are sent to their own textures.
        let can_place_in_atlas = texture_parameters.border_color.is_none()
            && atlas_size
                .map2(texture_parameters.size, |a, d| {
                    a as f32 * ATLAS_CUTOFF_FRAC >= d as f32
                })
                .reduce_and();
        let location = if can_place_in_atlas {
            // Fit into an atlas
            let mut loc = None;
            for (atlas_idx, &mut (ref mut atlas, texture_idx)) in atlases.iter_mut().enumerate() {
                if let Some(rectangle) = atlas.allocate(image_dims_size2d) {
                    let aabr = aabr_from_alloc_rect(rectangle);
                    loc = Some(CachedDetails::Atlas {
                        atlas_idx,
                        valid: true,
                        aabr,
                    });
                    let (ref texture, _, ref mut upload_batch) = &mut textures[texture_idx];
                    upload_image(renderer, texture, upload_batch, &image, aabr, gpu_premul);
                    break;
                }
            }

            match loc {
                Some(loc) => loc,
                // Create a new atlas
                None => {
                    let (mut atlas, (tex, bind)) = create_atlas_texture(renderer);
                    let aabr = atlas
                        .allocate(image_dims_size2d)
                        .map(aabr_from_alloc_rect)
                        .unwrap();
                    // NOTE: All mutations happen only after the texture creation succeeds!
                    let tex_idx = textures.insert((tex, bind, UiUploadBatchId::default()));
                    let atlas_idx = atlases.len();
                    atlases.push((atlas, tex_idx));
                    let (ref texture, _, ref mut upload_batch) = &mut textures[tex_idx];
                    upload_image(renderer, texture, upload_batch, &image, aabr, gpu_premul);
                    CachedDetails::Atlas {
                        atlas_idx,
                        valid: true,
                        aabr,
                    }
                },
            }
        } else {
            // Create a texture just for this
            let (tex, bind, upload_batch) =
                create_image(renderer, &image, texture_parameters, gpu_premul);
            // NOTE: All mutations happen only after the texture creation and upload
            // initiation succeeds! (completing the upload does not have any
            // failure cases afaik)
            let index = textures.insert((tex, bind, upload_batch));
            CachedDetails::Texture { index, valid: true }
        };

        // Extract information from cache entry.
        let (idx, _, aabr) = location.info(atlases, textures);

        // Insert into cached map
        details.insert(location);

        Some((
            transformed_aabr_and_scale(aabr.map(|e| e as f64)),
            TexId(idx),
        ))
    }
}

/// Prepare the graphic into the form that will be uploaded to the GPU.
///
/// For voxel graphics, draws the graphic at the specified dimensions.
///
/// Alpha premultiplication is necessary so that  images so they can be linearly
/// filtered on the GPU. Premultiplication can either occur here or on the GPU
/// depending on the size of the image and other factors. If premultiplication
/// on the GPU is needed the returned bool will be `true`.
fn prepare_graphic<'graphic>(
    graphic: &'graphic Graphic,
    cache_key: CacheKey,
    dims: Vec2<u16>,
    keyed_jobs: &mut KeyedJobs<CacheKey, RgbaImage>,
    pool: Option<&SlowJobPool>,
) -> Option<(Cow<'graphic, RgbaImage>, bool)> {
    prof_span!("prepare_graphic");
    match graphic {
        Graphic::Blank => None,
        Graphic::Image(image, _border_color) => {
            // Image will be rescaled when sampling from it on the GPU so we don't
            // need to resize it here.
            //
            // TODO: We could potentially push premultiplication even earlier (e.g. to the
            // time of loading images or packaging veloren for distribution).
            let mut rgba_cow = image.as_rgba8().map_or_else(
                || {
                    // TODO: we may want to require loading in as the rgba8 format so we don't have
                    // to perform conversion here. On the other hand, we can take advantage of
                    // certain formats to know that alpha premultiplication doesn't need to be
                    // performed (but we would probably just want to store that with the loaded
                    // rgba8 format).
                    Cow::Owned(image.to_rgba8())
                },
                Cow::Borrowed,
            );
            // NOTE: We do premultiplication on the main thread since if it would be
            // expensive enough to do in the background we would just do it on
            // the GPU. Could still use `rayon` to parallelize this work, if
            // needed.
            let premultiply_strategy = PremultiplyStrategy::determine(&rgba_cow);
            let needs_gpu_premultiply = match premultiply_strategy {
                PremultiplyStrategy::UseGpu => true,
                PremultiplyStrategy::NotNeeded => false,
                PremultiplyStrategy::UseCpu => {
                    // NOTE: to_mut will clone the image if it was Cow::Borrowed
                    premultiply_alpha(rgba_cow.to_mut());
                    false
                },
            };

            Some((rgba_cow, needs_gpu_premultiply))
        },
        Graphic::Voxel(segment, trans, sample_strat) => keyed_jobs
            .spawn(pool, cache_key, || {
                let segment = Arc::clone(segment);
                let (trans, sample_strat) = (*trans, *sample_strat);
                move |_| {
                    // TODO: for now we always use CPU premultiplication for these, may want to
                    // re-evaluate this after zoomy worldgen branch is merged (and it is more clear
                    // when these jobs go to the background thread pool or not).

                    // Render voxel model at requested resolution
                    let mut image = renderer::draw_vox(&segment, dims, trans, sample_strat);
                    premultiply_alpha(&mut image);
                    image
                }
            })
            .map(|(_, v)| (Cow::Owned(v), false)),
    }
}

fn atlas_size(renderer: &Renderer) -> Vec2<u32> {
    let max_texture_size = renderer.max_texture_size();

    renderer
        .resolution()
        .map(|e| (e * GRAPHIC_CACHE_RELATIVE_SIZE).clamp(512, max_texture_size))
}

/// This creates a texture suitable for sampling from during the UI pass and
/// rendering too during alpha premultiplication upload passes.
fn create_image_texture(
    renderer: &mut Renderer,
    size: Vec2<u32>,
    address_mode: Option<wgpu::AddressMode>,
) -> (Arc<Texture>, UiTextureBindGroup) {
    // TODO: Right now we have to manually clear images to workaround AMD DX bug,
    // for this we use Queue::write_texture which needs this usage. I think this
    // may be fixed in newer wgpu versions that auto-clear the texture.
    let workaround_usage = wgpu::TextureUsages::COPY_DST;
    let tex_info = wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT // GPU premultiply
            | wgpu::TextureUsages::COPY_DST // CPU premultiply
            | wgpu::TextureUsages::TEXTURE_BINDING // using image in ui rendering
            | workaround_usage,
        view_formats: &[],
    };
    let view_info = wgpu::TextureViewDescriptor {
        format: Some(tex_info.format),
        dimension: Some(wgpu::TextureViewDimension::D2),
        ..Default::default()
    };
    let address_mode = address_mode.unwrap_or(wgpu::AddressMode::ClampToEdge);
    let sampler_info = wgpu::SamplerDescriptor {
        address_mode_u: address_mode,
        address_mode_v: address_mode,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    };
    let tex = renderer.create_texture_raw(&tex_info, &view_info, &sampler_info);
    let bind = renderer.ui_bind_texture(&tex);
    (Arc::new(tex), bind)
}

fn create_atlas_texture(
    renderer: &mut Renderer,
) -> (SimpleAtlasAllocator, (Arc<Texture>, UiTextureBindGroup)) {
    let size = atlas_size(renderer);
    // Note: here we assume the max texture size is under i32::MAX.
    let atlas = SimpleAtlasAllocator::new(size2(size.x as i32, size.y as i32));
    let (tex, bind) = create_image_texture(renderer, size, None);
    (atlas, (tex, bind))
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

fn upload_image(
    renderer: &mut Renderer,
    target_texture: &Arc<Texture>,
    upload_batch: &mut UiUploadBatchId,
    image: &RgbaImage,
    aabr: Aabr<u16>,
    premultiply_on_gpu: bool,
) {
    // Check that this image and the target aabr are the same size (otherwise there
    // is a bug in this module).
    debug_assert_eq!(aabr.map(u32::from).size().into_tuple(), image.dimensions());
    if premultiply_on_gpu {
        *upload_batch =
            renderer.ui_premultiply_upload(target_texture, *upload_batch, image, aabr.min);
    } else {
        let aabr = aabr.map(u32::from);
        let offset = aabr.min.into_array();
        let size = aabr.size().into_array();
        // upload directly
        renderer.update_texture(
            target_texture,
            offset,
            size,
            // NOTE: Rgba texture, so each pixel is 4 bytes, ergo this cannot fail.
            // We make the cast parameters explicit for clarity.
            bytemuck::cast_slice::<u8, [u8; 4]>(
                &(&**image)[..size[0] as usize * size[1] as usize * 4],
            ),
        )
    }
}

// This is used for border_color.is_some() images (ie the map image).
fn create_image(
    renderer: &mut Renderer,
    image: &RgbaImage,
    texture_parameters: TextureParameters,
    premultiply_on_gpu: bool,
) -> (Arc<Texture>, UiTextureBindGroup, UiUploadBatchId) {
    let (tex, bind) = create_image_texture(
        renderer,
        texture_parameters.size.map(u32::from),
        texture_parameters
            .border_color
            // TODO: either use the desktop only border color or just emulate this
            //.map(|c| c.into_array().into()),
            .map(|_| wgpu::AddressMode::ClampToBorder),
    );
    let mut upload_batch = UiUploadBatchId::default();
    let aabr = Aabr {
        min: Vec2::zero(),
        max: texture_parameters.size,
    };
    upload_image(
        renderer,
        &tex,
        &mut upload_batch,
        image,
        aabr,
        premultiply_on_gpu,
    );
    (tex, bind, upload_batch)
}

// CPU-side alpha premultiplication implementation.

pub struct PremultiplyLookupTable {
    alpha: [u16; 256],
    // This is for both colors that are always below the linear transform threshold (of the
    // transform between linear/non-linear srgb) and colors that start above the threshold when
    // transforming into linear srgb and then fall below it after being multiplied by alpha (before
    // being transformed out of linear srgb).
    color: [u16; 256],
}

impl Default for PremultiplyLookupTable {
    fn default() -> Self {
        #[rustfmt::skip]
        fn accurate_to_linear(c: u8) -> f32 {
            let c = c as f32 / 255.0;
            // https://en.wikipedia.org/wiki/SRGB#Transformation
            if c <= 0.04045 {
                c / 12.92
            } else {
                // 0.055 ~= 14
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }

        use core::array;
        let alpha = array::from_fn(|alpha| {
            // NOTE: u16::MAX + 1 here relies on the max alpha being short-circuited (and
            // not using this table). We multiply by this factor since it is a
            // power of 2, which means later demultiplying it will optimize to a
            // bitshift.
            (((alpha as f32 / 255.0).powf(1.0 / 2.4) * (u16::MAX as f32 + 1.0)) + 0.5) as u16
        });
        let color = array::from_fn(|color| {
            (if color <= 10 {
                //  <= 10 means the transform is linear!
                color as f32 / 255.0
            } else {
                // Here the transform into linear srgb isn't linear but the transform out of it is. 
                //
                // This is transform into and out of linear srgb with the theoretical alpha
                // multiplication factored out.
                accurate_to_linear(color as u8) * 12.92
            }
            // take advantage of the precision offered by u16
            * (1 << 13) as f32
            // round to the nearest integer when the cast truncates
            + 0.5) as u16
        });
        Self { alpha, color }
    }
}

fn premultiply_alpha(image: &mut RgbaImage) {
    lazy_static::lazy_static! {
        static ref LOOKUP: PremultiplyLookupTable = Default::default();
    }
    let lookup = &*LOOKUP;
    // TODO: Apparently it is possible for ImageBuffer raw vec to have more pixels
    // than the dimensions of the actual image (I don't think we actually have
    // this occuring but we should probably fix other spots that use the raw
    // buffer). See:
    // https://github.com/image-rs/image/blob/a1ce569afd476e881acafdf9e7a5bce294d0db9a/src/buffer.rs#L664
    let dims = image.dimensions();
    let image_buffer_len = dims.0 as usize * dims.1 as usize * 4;
    let (arrays, end) = (&mut **image)[..image_buffer_len].as_chunks_mut::<{ 4 * 4 }>();
    // Rgba8 has 4 bytes per pixel there should be no remainder when dividing by 4.
    let (end, _) = end.as_chunks_mut::<4>();
    end.iter_mut().for_each(|pixel| {
        let alpha = pixel[3];
        if alpha == 0 {
            *pixel = [0; 4];
            return;
        } else if alpha == 255 {
            return;
        };

        for color in &mut pixel[..3] {
            let predicted = ((lookup.alpha[alpha as usize] as u32) * (*color as u32 + 14) + 32433)
                / (u16::MAX as u32 + 1);
            let multiplied_color = (if predicted < 9 + 14 {
                (lookup.color[*color as usize] as u32 * alpha as u32 + 4096) >> 13
            } else {
                predicted - 14
            }) as u8;
            *color = multiplied_color;
        }
    });
    arrays.iter_mut().for_each(|pixelx4| {
        // Short-circuit for alpha == 0 or 255
        // This adds ~7 us (worst case) for a 256x256 image.
        // Best case is decreased to 20 us total time.
        if pixelx4[3] == pixelx4[7] && pixelx4[3] == pixelx4[11] && pixelx4[3] == pixelx4[15] {
            if pixelx4[3] == 0 {
                *pixelx4 = [0; 16];
                return;
            } else if pixelx4[3] == u8::MAX {
                return;
            }
        }

        // Lookup transformed alpha values for each pixel first.
        // Putting this here seems to make things slightly faster.
        let factors = [
            lookup.alpha[pixelx4[3] as usize],
            lookup.alpha[pixelx4[7] as usize],
            lookup.alpha[pixelx4[11] as usize],
            lookup.alpha[pixelx4[15] as usize],
        ];
        for pixel_index in 0..4 {
            let alpha_factor = factors[pixel_index];
            let alpha = pixelx4[pixel_index * 4 + 3];
            // Putting this code outside the loop makes things take ~25% less time.
            let color_factors = [
                lookup.color[pixelx4[pixel_index * 4 + 0] as usize] as u32 * alpha as u32 + 4096,
                lookup.color[pixelx4[pixel_index * 4 + 1] as usize] as u32 * alpha as u32 + 4096,
                lookup.color[pixelx4[pixel_index * 4 + 2] as usize] as u32 * alpha as u32 + 4096,
            ];
            for i in 0..3 {
                let color = &mut pixelx4[pixel_index * 4 + i];
                // Loosely based on transform to linear and back (above threshold) (this is
                // where use of 14 comes from).
                // `32433` selected via trial and error to reduce the number of mismatches.
                // `/ (u16::MAX as u32 + 1)` transforms back to `u8` precision (we add 1 so it
                // will be a division by a power of 2 which optimizes well).
                let predicted =
                    ((alpha_factor as u32) * (*color as u32 + 14) + 32328) / (u16::MAX as u32 + 1);
                let multiplied_color = (if predicted < 9 + 14 {
                    // Here we handle two cases:
                    // 1. When the transform starts and ends as linear.
                    // 2. When the color is over the linear threshold for the transform into linear
                    //    space but below this threshold when transforming back out (due to being
                    //    multiplied with a small alpha).
                    // (in both cases the result is linearly related to alpha and we can encode how
                    // it is related to the color in a lookup table)
                    // NOTE: 212 is the largest color value used here (when alpha isn't 0)
                    color_factors[i] >> 13
                } else {
                    predicted - 14
                }) as u8;
                *color = multiplied_color;
            }
        }
    });
}

/// Strategy for how alpha premultiplication will be applied to an image.
enum PremultiplyStrategy {
    UseCpu,
    UseGpu,
    // Image is fully opaque.
    NotNeeded,
}

impl PremultiplyStrategy {
    #[rustfmt::skip] // please don't format comment with 'ns/pixel' to a separate line from the value
    fn determine(image: &RgbaImage) -> Self {
        // TODO: Would be useful to re-time this after a wgpu update.
        //
        // Thresholds below are based on the timing measurements of the CPU based premultiplication
        // vs ovehead of interacting with the GPU API to perform premultiplication on the GPU.
        // These timings are quite circumstantial and could vary between machines, wgpu updates,
        // and changes to the structure of the GPU based path.  
        //
        // GPU path costs (For calculations I used `57.6 us` as a roughly reasonable estimate of
        // total time here but that can vary lower and higher. Everything is a bit imprecise here
        // so I won't list individual timings. The key takeaway is that this can be made more
        // efficient by avoidiing the create/drop of a texture, texture view, and bind group for
        // each image. Also, if we didn't need a separate render pass for each target image that
        // would be helpful as well. Using compute passes and passing data in as a raw buffer may
        // help with both of these but initial attempts with that ran into issues (e.g. when we get
        // the ability to have non-srgb views of srgb textures that will be useful)):
        // * create/drop texture
        // * create/drop texture view
        // * create/drop bind group
        // * run render pass (NOTE: if many images are processed at once with the same target
        //   texture this portion of the cost can be split between them)
        //
        // CPU path costs:
        // * clone image (0.17 ns/pixel (benchmark) - 0.73 ns/pixel (in voxygen))
        // * run premultiplication (0.305 ns/pixel (when shortcircuits are always hit) -
        //   3.81 ns/pixel (with random alpha))
        //
        // Shared costs include:
        // * write_texture
        // * (optional) check for fraction of shortcircuit blocks in image (0.223 ns/pixel)
        //
        // `ALWAYS_CPU_THRESHOLD` is roughly:
        // ("cost of GPU path" + "shortcircuit count cost") / "worst case cost of CPU path per pixel"
        //
        // `ALWAYS_GPU_THRESHOLD` is NOT: "cost of GPU path" / "best case cost of CPU path per pixel"
        // since the cost of checking for whether the CPU path is better at this quantity of pixels
        // becomes more than the on the amount of overhead we are willing to add to the worst case
        // scenario where we run the short-circuit count check and end up using the GPU path. The
        // currently selected value of 200x200 adds at most about ~20% of the cost of the GPU path.
        // (TODO: maybe we could have the check bail out early if the results aren't looking
        // favorable for the CPU path and/or sample a random subset of the pixels).
        //
        // `CHECKED_THRESHOLD` is roughly: "cost of GPU path / "best case cost of CPU path per pixel"
        const ALWAYS_CPU_THRESHOLD: usize = 120 * 120;
        const ALWAYS_GPU_THRESHOLD: usize = 200 * 200;
        const CHECKED_THRESHOLD: usize = 240 * 240;

        let dims = image.dimensions();
        let pixel_count = dims.0 as usize * dims.1 as usize;
        if pixel_count <= ALWAYS_CPU_THRESHOLD {
            Self::UseCpu
        } else if pixel_count > ALWAYS_GPU_THRESHOLD {
            Self::UseGpu
        } else if let Some(fraction) = fraction_shortcircuit_blocks(image) {
            // This seems correct...?
            // TODO: I think we technically can exit the fraction checking early if we know the
            // total fraction value will be over: (threshold - ALWAYS_CPU_THRESHOLD) /
            // (CHECKED_THRESHOLD - ALWAYS_CPU_THRESHOLD).
            let threshold = fraction * CHECKED_THRESHOLD as f32
                + (1.0 - fraction) * ALWAYS_CPU_THRESHOLD as f32;
            if pixel_count as f32 <= threshold {
                Self::UseCpu
            } else {
                Self::UseGpu
            }
        } else {
            Self::NotNeeded
        }
    }
}

/// Useful to estimates cost of premultiplying alpha in the provided image via
/// the CPU method.
///
/// Computes the fraction of 4 pixel chunks that are fully translucent or
/// opaque. Returns `None` if no premultiplication is needed (i.e. all alpha
/// values are 255).
#[allow(clippy::unusual_byte_groupings)]
fn fraction_shortcircuit_blocks(image: &RgbaImage) -> Option<f32> {
    let dims = image.dimensions();
    let pixel_count = dims.0 as usize * dims.1 as usize;
    let (arrays, end) = (&**image)[..pixel_count * 4].as_chunks::<{ 4 * 4 }>();

    // Rgba8 has 4 bytes per pixel there should be no remainder when dividing by 4.
    let (end, _) = end.as_chunks::<4>();
    let end_is_opaque = end.iter().all(|pixel| pixel[3] == 255);

    // 14.6 us for 256x256 image
    let num_chunks = arrays.len();
    let mut num_translucent = 0;
    let mut num_opaque = 0;
    arrays.iter().for_each(|pixelx4| {
        let v = u128::from_ne_bytes(*pixelx4);
        let alpha_mask = 0x000000FF_000000FF_000000FF_000000FF;
        let masked = v & alpha_mask;
        if masked == 0 {
            num_translucent += 1;
        } else if masked == alpha_mask {
            num_opaque += 1;
        }
    });

    if num_chunks == num_opaque && num_translucent == 0 && end_is_opaque {
        None
    } else {
        Some((num_translucent as f32 + num_opaque as f32) / num_chunks as f32)
    }
}
