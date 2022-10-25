mod pixel_art;
pub mod renderer;

pub use renderer::{SampleStrat, Transform};

use crate::{
    render::{Renderer, Texture, UiPremultiplyUpload, UiTextureBindGroup},
    ui::KeyedJobs,
};
use common::{figure::Segment, slowjob::SlowJobPool};
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
        textures: &Slab<(Texture, UiTextureBindGroup, Vec<UiPremultiplyUpload>)>,
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

    // TODO: what if requested size is 0? Do we currently panic on this case and
    // expect caller not to ask for 0 size? (if so document that)
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
    textures: Slab<(Texture, UiTextureBindGroup, Vec<UiPremultiplyUpload>)>,
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
        let tex_id = textures.insert((tex, bind, Vec::new()));

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
        let new_requirements = TextureRequirements::from_graphic(&new);
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
                        // TODO: if replace_graphic is used continously for small images (i.e.
                        // images placed into an atlas) of different sizes, that can use up our
                        // atlas space since spots in the atlas can't be reused. (this scenario is
                        // now possible with scaling being done during sampling rather than placing
                        // resized version into the atlas)
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
        let (tex, bind, _uploads) = self.textures.get(id.0).expect("Invalid TexId used");
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
        let tex_id = textures.insert((tex, bind, Vec::new()));
        self.atlases = vec![(atlas, tex_id)];
        self.textures = textures;
    }

    /// Source rectangle should be from 0 to 1, and represents a bounding box
    /// for the source image of the graphic.
    ///
    /// [`complete_premultiply_uploads`](Self::complete_premultiply_uploads)
    /// needs to be called to finalize updates on the GPU that are initiated
    /// here. Thus, ideally that would be called before drawing UI elements
    /// using the images cached here.
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
                // S-TODO div by zero potential? If so, is NaN an issue in that case?
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

        let requirements = TextureRequirements::from_graphic(&graphic)?;
        let (key, texture_parameters) =
            requirements.to_key_and_tex_parameters(graphic_id, requested_dims_upright);

        let details = match cache_map.entry(key) {
            Entry::Occupied(details) => {
                let details = details.get();
                let (idx, valid, aabr) = details.info(atlases, textures);

                // Check if the cached version has been invalidated by replacing the underlying
                // graphic
                if !valid {
                    // Create image
                    let image = prepare_graphic(
                        graphic,
                        key,
                        requested_dims_upright,
                        false,
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
                    upload_image(renderer, aabr, &mut textures[idx].2, &image);
                }

                return Some((transformed_aabr(aabr.map(|e| e as f64)), TexId(idx)));
            },
            Entry::Vacant(details) => details,
        };

        // Construct image in an optional threadpool.
        let image = prepare_graphic(
            graphic,
            key,
            requested_dims_upright,
            false,
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
                    upload_image(renderer, aabr, &mut textures[texture_idx].2, &image);
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
                    let tex_idx = textures.insert((tex, bind, Vec::new()));
                    let atlas_idx = atlases.len();
                    atlases.push((atlas, tex_idx));
                    upload_image(renderer, aabr, &mut textures[tex_idx].2, &image);
                    CachedDetails::Atlas {
                        atlas_idx,
                        valid: true,
                        aabr,
                    }
                },
            }
        } else {
            // Create a texture just for this
            let (tex, bind, uploads) = create_image(renderer, &image, texture_parameters);
            // NOTE: All mutations happen only after the texture creation and upload
            // initiation succeeds! (completing the upload does not have any failure cases
            // afaik)
            let index = textures.insert((tex, bind, uploads));
            CachedDetails::Texture { index, valid: true }
        };

        // Extract information from cache entry.
        let (idx, _, aabr) = location.info(atlases, textures);

        // Insert into cached map
        details.insert(location);

        Some((transformed_aabr(aabr.map(|e| e as f64)), TexId(idx)))
    }

    /// Runs render passes with alpha premultiplication pipeline to complete any
    /// pending uploads.
    ///
    /// This should be called before starting the pass where the ui is rendered.
    pub fn complete_premultiply_uploads(&mut self, drawer: &mut crate::render::Drawer<'_>) {
        drawer.run_ui_premultiply_passes(
            self.textures
                .iter_mut()
                .map(|(_tex_id, (texture, _, uploads))| (&*texture, core::mem::take(uploads))),
        );
    }
}

/// Prepare the graphic into the form that will be uploaded to the GPU.
///
/// For voxel graphics, draws the graphic at the specified dimensions.
///
/// Also can pre-multiplies alpha in images so they can be linearly filtered on
/// the GPU (this is optional since we also have a path to do this
/// premultiplication on the GPU).
fn prepare_graphic<'graphic>(
    graphic: &'graphic Graphic,
    cache_key: CacheKey,
    dims: Vec2<u16>,
    premultiply_on_cpu: bool, // TODO: currently unused
    keyed_jobs: &mut KeyedJobs<CacheKey, RgbaImage>,
    pool: Option<&SlowJobPool>,
) -> Option<Cow<'graphic, RgbaImage>> {
    match graphic {
        // Short-circuit spawning a job on the threadpool for blank graphics
        Graphic::Blank => None,
        Graphic::Image(image, _border_color) => {
            if premultiply_on_cpu {
                keyed_jobs
                    .spawn(pool, cache_key, || {
                        let image = Arc::clone(image);
                        move |_| {
                            // Image will be rescaled when sampling from it on the GPU so we don't
                            // need to resize it here.
                            let mut image = image.to_rgba8();
                            // TODO: could potentially do this when loading the image and for voxel
                            // images maybe at some point in the `draw_vox` processing. Or we could
                            // push it in the other direction and do conversion on the GPU.
                            premultiply_alpha(&mut image);
                            image
                        }
                    })
                    .map(|(_, v)| Cow::Owned(v))
            } else if let Some(rgba) = image.as_rgba8() {
                Some(Cow::Borrowed(rgba))
            } else {
                // TODO: we should require rgba8 format
                warn!("Non-rgba8 image in UI used this may be deprecated.");
                Some(Cow::Owned(image.to_rgba8()))
            }
        },
        Graphic::Voxel(segment, trans, sample_strat) => keyed_jobs
            .spawn(pool, cache_key, || {
                let segment = Arc::clone(segment);
                let (trans, sample_strat) = (*trans, *sample_strat);
                move |_| {
                    // Render voxel model at requested resolution
                    let mut image = renderer::draw_vox(&segment, dims, trans, sample_strat);
                    if premultiply_on_cpu {
                        premultiply_alpha(&mut image);
                    }
                    image
                }
            })
            .map(|(_, v)| Cow::Owned(v)),
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
) -> (Texture, UiTextureBindGroup) {
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
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
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
    (tex, bind)
}

fn create_atlas_texture(
    renderer: &mut Renderer,
) -> (SimpleAtlasAllocator, (Texture, UiTextureBindGroup)) {
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
    aabr: Aabr<u16>,
    target_texture_uploads: &mut Vec<UiPremultiplyUpload>,
    image: &RgbaImage,
) {
    let aabr = aabr.map(u32::from);
    // Check that this image and the target aabr are the same size (otherwise there
    // is a bug in this module).
    debug_assert_eq!(aabr.size().into_tuple(), image.dimensions());
    let offset = aabr.min.into_array();

    // TODO: can we transparently have cpu based version behind this (actually this
    // would introduce more complexity to be able to do it in the background,
    // but we could to it not in the background here especially for smaller
    // things this would work well)
    let upload = UiPremultiplyUpload::prepare(renderer, image, offset);
    target_texture_uploads.push(upload);
    //todo!()
}

// This is used for border_color.is_some() images (ie the map image).
fn create_image(
    renderer: &mut Renderer,
    image: &RgbaImage,
    texture_parameters: TextureParameters,
) -> (Texture, UiTextureBindGroup, Vec<UiPremultiplyUpload>) {
    let (tex, bind) = create_image_texture(
        renderer,
        texture_parameters.size.map(u32::from),
        texture_parameters
            .border_color
            // TODO: either use the desktop only border color or just emulate this
            //.map(|c| c.into_array().into()),
            .map(|_| wgpu::AddressMode::ClampToBorder),
    );
    let mut uploads = Vec::new();
    let aabr = Aabr {
        min: Vec2::zero(),
        max: texture_parameters.size,
    };
    upload_image(renderer, aabr, &mut uploads, image);
    (tex, bind, uploads)
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
    let (arrays, end) = (&mut **image)[..image_buffer_len].as_chunks_mut::<{ 4 * 4 }>();
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

// Next step: Handling invalidation / removal of old textures when
// replace_graphic is used under new resizing scheme.
//
// TODO: does screenshot texture have COPY_DST? I don't think it needs this.
