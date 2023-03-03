mod watcher;

pub use self::watcher::{BlocksOfInterest, FireplaceType, Interaction};

use crate::{
    mesh::{
        greedy::{GreedyMesh, SpriteAtlasAllocator},
        segment::generate_mesh_base_vol_sprite,
        terrain::{generate_mesh, SUNLIGHT, SUNLIGHT_INV},
    },
    render::{
        pipelines::{self, ColLights},
        AltIndices, ColLightInfo, CullingMode, FirstPassDrawer, FluidVertex, GlobalModel,
        Instances, LodData, Mesh, Model, RenderError, Renderer, SpriteGlobalsBindGroup,
        SpriteInstance, SpriteVertex, SpriteVerts, TerrainLocals, TerrainShadowDrawer,
        TerrainVertex, SPRITE_VERT_PAGE_SIZE,
    },
};

use super::{
    camera::{self, Camera},
    math, SceneData, RAIN_THRESHOLD,
};
use common::{
    assets::{self, AssetExt, DotVoxAsset},
    figure::Segment,
    spiral::Spiral2d,
    terrain::{Block, SpriteKind, TerrainChunk},
    vol::{BaseVol, ReadVol, RectRasterableVol, SampleVol},
    volumes::vol_grid_2d::{VolGrid2d, VolGrid2dError},
};
use common_base::{prof_span, span};
use core::{f32, fmt::Debug, marker::PhantomData, time::Duration};
use crossbeam_channel as channel;
use guillotiere::AtlasAllocator;
use hashbrown::HashMap;
use serde::Deserialize;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use strum::IntoEnumIterator;
use tracing::warn;
use treeculler::{BVol, Frustum, AABB};
use vek::*;

const SPRITE_SCALE: Vec3<f32> = Vec3::new(1.0 / 11.0, 1.0 / 11.0, 1.0 / 11.0);
const SPRITE_LOD_LEVELS: usize = 5;

// For rain occlusion we only need to render the closest chunks.
/// How many chunks are maximally rendered for rain occlusion.
pub const RAIN_OCCLUSION_CHUNKS: usize = 25;

#[derive(Clone, Copy, Debug)]
struct Visibility {
    in_range: bool,
    in_frustum: bool,
}

impl Visibility {
    /// Should the chunk actually get rendered?
    fn is_visible(&self) -> bool {
        // Currently, we don't take into account in_range to allow all chunks to do
        // pop-in. This isn't really a problem because we no longer have VD mist
        // or anything like that. Also, we don't load chunks outside of the VD
        // anyway so this literally just controls which chunks get actually
        // rendered.
        /* self.in_range && */
        self.in_frustum
    }
}

/// Type of closure used for light mapping.
type LightMapFn = Arc<dyn Fn(Vec3<i32>) -> f32 + Send + Sync>;

pub struct TerrainChunkData {
    // GPU data
    load_time: f32,
    opaque_model: Option<Model<TerrainVertex>>,
    fluid_model: Option<Model<FluidVertex>>,
    /// If this is `None`, this texture is not allocated in the current atlas,
    /// and therefore there is no need to free its allocation.
    col_lights_alloc: Option<guillotiere::AllocId>,
    /// The actual backing texture for this chunk.  Use this for rendering
    /// purposes.  The texture is reference-counted, so it will be
    /// automatically freed when no chunks are left that need it (though
    /// shadow chunks will still keep it alive; we could deal with this by
    /// making this an `Option`, but it probably isn't worth it since they
    /// shouldn't be that much more nonlocal than regular chunks).
    col_lights: Arc<ColLights<pipelines::terrain::Locals>>,
    light_map: LightMapFn,
    glow_map: LightMapFn,
    sprite_instances: [(Instances<SpriteInstance>, AltIndices); SPRITE_LOD_LEVELS],
    locals: pipelines::terrain::BoundLocals,
    pub blocks_of_interest: BlocksOfInterest,

    visible: Visibility,
    can_shadow_point: bool,
    can_shadow_sun: bool,
    z_bounds: (f32, f32),
    sun_occluder_z_bounds: (f32, f32),
    frustum_last_plane_index: u8,

    alt_indices: AltIndices,
}

/// The depth at which the intermediate zone between underground and surface
/// begins
pub const SHALLOW_ALT: f32 = 24.0;
/// The depth at which the intermediate zone between underground and surface
/// ends
pub const DEEP_ALT: f32 = 96.0;
/// The depth below the surface altitude at which the camera switches from
/// displaying surface elements to underground elements
pub const UNDERGROUND_ALT: f32 = (SHALLOW_ALT + DEEP_ALT) * 0.5;

// The distance (in chunks) within which all levels of the chunks will be drawn
// to minimise cull-related popping.
const NEVER_CULL_DIST: i32 = 3;

#[derive(Copy, Clone)]
struct ChunkMeshState {
    pos: Vec2<i32>,
    started_tick: u64,
    is_worker_active: bool,
    // If this is set, we skip the actual meshing part of the update.
    skip_remesh: bool,
}

/// Just the mesh part of a mesh worker response.
pub struct MeshWorkerResponseMesh {
    z_bounds: (f32, f32),
    sun_occluder_z_bounds: (f32, f32),
    opaque_mesh: Mesh<TerrainVertex>,
    fluid_mesh: Mesh<FluidVertex>,
    col_lights_info: ColLightInfo,
    light_map: LightMapFn,
    glow_map: LightMapFn,
    alt_indices: AltIndices,
}

/// A type produced by mesh worker threads corresponding to the position and
/// mesh of a chunk.
struct MeshWorkerResponse {
    pos: Vec2<i32>,
    sprite_instances: [(Vec<SpriteInstance>, AltIndices); SPRITE_LOD_LEVELS],
    /// If None, this update was requested without meshing.
    mesh: Option<MeshWorkerResponseMesh>,
    started_tick: u64,
    blocks_of_interest: BlocksOfInterest,
}

#[derive(Deserialize)]
/// Configuration data for an individual sprite model.
struct SpriteModelConfig<Model> {
    /// Data for the .vox model associated with this sprite.
    model: Model,
    /// Sprite model center (as an offset from 0 in the .vox file).
    offset: (f32, f32, f32),
    /// LOD axes (how LOD gets applied along each axis, when we switch
    /// to an LOD model).
    lod_axes: (f32, f32, f32),
}

#[derive(Deserialize)]
/// Configuration data for a group of sprites (currently associated with a
/// particular SpriteKind).
struct SpriteConfig<Model> {
    /// All possible model variations for this sprite.
    // NOTE: Could make constant per sprite type, but eliminating this indirection and
    // allocation is probably not that important considering how sprites are used.
    variations: Vec<SpriteModelConfig<Model>>,
    /// The extent to which the sprite sways in the window.
    ///
    /// 0.0 is normal.
    wind_sway: f32,
}

// TODO: reduce llvm IR lines from this
/// Configuration data for all sprite models.
///
/// NOTE: Model is an asset path to the appropriate sprite .vox model.
#[derive(Deserialize)]
#[serde(try_from = "HashMap<SpriteKind, Option<SpriteConfig<String>>>")]
struct SpriteSpec([Option<SpriteConfig<String>>; 256]);

impl SpriteSpec {
    fn get(&self, kind: SpriteKind) -> Option<&SpriteConfig<String>> {
        const _: () = assert!(core::mem::size_of::<SpriteKind>() == 1);
        // NOTE: This will never be out of bounds since `SpriteKind` is `repr(u8)`
        self.0[kind as usize].as_ref()
    }
}

/// Conversion of SpriteSpec from a hashmap failed because some sprites were
/// missing.
struct SpritesMissing(Vec<SpriteKind>);

use core::fmt;

impl fmt::Display for SpritesMissing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Missing entries in the sprite manifest for these sprites: {:?}",
            &self.0,
        )
    }
}

// Here we ensure all variants have an entry in the config.
impl TryFrom<HashMap<SpriteKind, Option<SpriteConfig<String>>>> for SpriteSpec {
    type Error = SpritesMissing;

    fn try_from(
        mut map: HashMap<SpriteKind, Option<SpriteConfig<String>>>,
    ) -> Result<Self, Self::Error> {
        let mut array = [(); 256].map(|()| None);
        let sprites_missing = SpriteKind::iter()
            .filter(|kind| match map.remove(kind) {
                Some(config) => {
                    array[*kind as usize] = config;
                    false
                },
                None => true,
            })
            .collect::<Vec<_>>();

        if sprites_missing.is_empty() {
            Ok(Self(array))
        } else {
            Err(SpritesMissing(sprites_missing))
        }
    }
}

impl assets::Asset for SpriteSpec {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

/// Function executed by worker threads dedicated to chunk meshing.

/// skip_remesh is either None (do the full remesh, including recomputing the
/// light map), or Some((light_map, glow_map)).
fn mesh_worker(
    pos: Vec2<i32>,
    z_bounds: (f32, f32),
    skip_remesh: Option<(LightMapFn, LightMapFn)>,
    started_tick: u64,
    volume: <VolGrid2d<TerrainChunk> as SampleVol<Aabr<i32>>>::Sample,
    max_texture_size: u16,
    chunk: Arc<TerrainChunk>,
    range: Aabb<i32>,
    sprite_data: &HashMap<(SpriteKind, usize), [SpriteData; SPRITE_LOD_LEVELS]>,
    sprite_config: &SpriteSpec,
) -> MeshWorkerResponse {
    span!(_guard, "mesh_worker");
    let blocks_of_interest = BlocksOfInterest::from_chunk(&chunk);

    let mesh;
    let (light_map, glow_map) = if let Some((light_map, glow_map)) = &skip_remesh {
        mesh = None;
        (&**light_map, &**glow_map)
    } else {
        let (
            opaque_mesh,
            fluid_mesh,
            _shadow_mesh,
            (bounds, col_lights_info, light_map, glow_map, alt_indices, sun_occluder_z_bounds),
        ) = generate_mesh(
            &volume,
            (
                range,
                Vec2::new(max_texture_size, max_texture_size),
                &blocks_of_interest,
            ),
        );
        mesh = Some(MeshWorkerResponseMesh {
            // TODO: Take sprite bounds into account somehow?
            z_bounds: (bounds.min.z, bounds.max.z),
            sun_occluder_z_bounds,
            opaque_mesh,
            fluid_mesh,
            col_lights_info,
            light_map,
            glow_map,
            alt_indices,
        });
        // Pointer juggling so borrows work out.
        let mesh = mesh.as_ref().unwrap();
        (&*mesh.light_map, &*mesh.glow_map)
    };

    MeshWorkerResponse {
        pos,
        // Extract sprite locations from volume
        sprite_instances: {
            prof_span!("extract sprite_instances");
            let mut instances = [(); SPRITE_LOD_LEVELS].map(|()| {
                (
                    Vec::new(), // Deep
                    Vec::new(), // Shallow
                    Vec::new(), // Surface
                )
            });

            let (underground_alt, deep_alt) = volume
                .get_key(volume.pos_key((range.min + range.max) / 2))
                .map_or((0.0, 0.0), |c| {
                    (c.meta().alt() - SHALLOW_ALT, c.meta().alt() - DEEP_ALT)
                });

            for x in 0..TerrainChunk::RECT_SIZE.x as i32 {
                for y in 0..TerrainChunk::RECT_SIZE.y as i32 {
                    for z in z_bounds.0 as i32..z_bounds.1 as i32 + 1 {
                        let rel_pos = Vec3::new(x, y, z);
                        let wpos = Vec3::from(pos * TerrainChunk::RECT_SIZE.map(|e: u32| e as i32))
                            + rel_pos;

                        let block = if let Ok(block) = volume.get(wpos) {
                            block
                        } else {
                            continue;
                        };
                        let sprite = if let Some(sprite) = block.get_sprite() {
                            sprite
                        } else {
                            continue;
                        };

                        if let Some(cfg) = sprite_config.get(sprite) {
                            let seed = wpos.x as u64 * 3
                                + wpos.y as u64 * 7
                                + wpos.x as u64 * wpos.y as u64; // Awful PRNG
                            let ori = (block.get_ori().unwrap_or((seed % 4) as u8 * 2)) & 0b111;
                            let variation = seed as usize % cfg.variations.len();
                            let key = (sprite, variation);
                            // NOTE: Safe because we called sprite_config_for already.
                            // NOTE: Safe because 0 ≤ ori < 8
                            let light = light_map(wpos);
                            let glow = glow_map(wpos);

                            for ((deep_level, shallow_level, surface_level), sprite_data) in
                                instances.iter_mut().zip(&sprite_data[&key])
                            {
                                let mat = Mat4::identity()
                                    // Scaling for different LOD resolutions
                                    .scaled_3d(sprite_data.scale)
                                    // Offset
                                    .translated_3d(sprite_data.offset)
                                    .scaled_3d(SPRITE_SCALE)
                                    .rotated_z(f32::consts::PI * 0.25 * ori as f32)
                                    .translated_3d(
                                        rel_pos.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0)
                                    );
                                // Add an instance for each page in the sprite model
                                for page in sprite_data.vert_pages.clone() {
                                    // TODO: could be more efficient to create once and clone while
                                    // modifying vert_page
                                    let instance = SpriteInstance::new(
                                        mat,
                                        cfg.wind_sway,
                                        sprite_data.scale.z,
                                        rel_pos,
                                        ori,
                                        light,
                                        glow,
                                        page,
                                        matches!(sprite, SpriteKind::Door | SpriteKind::DoorDark),
                                    );
                                    if (wpos.z as f32) < deep_alt {
                                        deep_level.push(instance);
                                    } else if wpos.z as f32 > underground_alt {
                                        surface_level.push(instance);
                                    } else {
                                        shallow_level.push(instance);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            instances.map(|(deep_level, shallow_level, surface_level)| {
                let deep_end = deep_level.len();
                let alt_indices = AltIndices {
                    deep_end,
                    underground_end: deep_end + shallow_level.len(),
                };
                (
                    deep_level
                        .into_iter()
                        .chain(shallow_level.into_iter())
                        .chain(surface_level.into_iter())
                        .collect(),
                    alt_indices,
                )
            })
        },
        mesh,
        blocks_of_interest,
        started_tick,
    }
}

struct SpriteData {
    // Sprite vert page ranges that need to be drawn
    vert_pages: core::ops::Range<u32>,
    // Scale
    scale: Vec3<f32>,
    // Offset
    offset: Vec3<f32>,
}

pub struct Terrain<V: RectRasterableVol = TerrainChunk> {
    /// This is always the *current* atlas into which data is being allocated.
    /// Once an atlas is too full to allocate the next texture, we always
    /// allocate a fresh texture and start allocating into that.  Trying to
    /// keep more than one texture available for allocation doesn't seem
    /// worth it, because our allocation patterns are heavily spatial (so all
    /// data allocated around the same time should have a very similar lifetime,
    /// even in pathological cases).  As a result, fragmentation effects
    /// should be minimal.
    ///
    /// TODO: Consider "moving GC" style allocation to deal with spatial
    /// fragmentation effects due to odd texture sizes, which in some cases
    /// might significantly reduce the number of textures we need for
    /// particularly difficult locations.
    atlas: AtlasAllocator,
    /// FIXME: This could possibly become an `AssetHandle<SpriteSpec>`, to get
    /// hot-reloading for free, but I am not sure if sudden changes of this
    /// value would break something
    sprite_config: Arc<SpriteSpec>,
    chunks: HashMap<Vec2<i32>, TerrainChunkData>,
    /// Temporary storage for dead chunks that might still be shadowing chunks
    /// in view.  We wait until either the chunk definitely cannot be
    /// shadowing anything the player can see, the chunk comes back into
    /// view, or for daylight to end, before removing it (whichever comes
    /// first).
    ///
    /// Note that these chunks are not complete; for example, they are missing
    /// texture data (they still currently hold onto a reference to their
    /// backing texture, but it generally can't be trusted for rendering
    /// purposes).
    shadow_chunks: Vec<(Vec2<i32>, TerrainChunkData)>,
    /* /// Secondary index into the terrain chunk table, used to sort through chunks by z index from
    /// the top down.
    z_index_down: BTreeSet<Vec3<i32>>,
    /// Secondary index into the terrain chunk table, used to sort through chunks by z index from
    /// the bottom up.
    z_index_up: BTreeSet<Vec3<i32>>, */
    // The mpsc sender and receiver used for talking to meshing worker threads.
    // We keep the sender component for no reason other than to clone it and send it to new
    // workers.
    mesh_send_tmp: channel::Sender<MeshWorkerResponse>,
    mesh_recv: channel::Receiver<MeshWorkerResponse>,
    mesh_todo: HashMap<Vec2<i32>, ChunkMeshState>,
    mesh_todos_active: Arc<AtomicU64>,
    mesh_recv_overflow: f32,

    // GPU data
    // Maps sprite kind + variant to data detailing how to render it
    sprite_data: Arc<HashMap<(SpriteKind, usize), [SpriteData; SPRITE_LOD_LEVELS]>>,
    sprite_globals: SpriteGlobalsBindGroup,
    sprite_col_lights: Arc<ColLights<pipelines::sprite::Locals>>,
    /// As stated previously, this is always the very latest texture into which
    /// we allocate.  Code cannot assume that this is the assigned texture
    /// for any particular chunk; look at the `texture` field in
    /// `TerrainChunkData` for that.
    col_lights: Arc<ColLights<pipelines::terrain::Locals>>,

    phantom: PhantomData<V>,
}

impl TerrainChunkData {
    pub fn can_shadow_sun(&self) -> bool { self.visible.is_visible() || self.can_shadow_sun }
}

#[derive(Clone)]
pub struct SpriteRenderContext {
    sprite_config: Arc<SpriteSpec>,
    // Maps sprite kind + variant to data detailing how to render it
    sprite_data: Arc<HashMap<(SpriteKind, usize), [SpriteData; SPRITE_LOD_LEVELS]>>,
    sprite_col_lights: Arc<ColLights<pipelines::sprite::Locals>>,
    sprite_verts_buffer: Arc<SpriteVerts>,
}

pub type SpriteRenderContextLazy = Box<dyn FnMut(&mut Renderer) -> SpriteRenderContext>;

impl SpriteRenderContext {
    pub fn new(renderer: &mut Renderer) -> SpriteRenderContextLazy {
        let max_texture_size = renderer.max_texture_size();

        struct SpriteWorkerResponse {
            sprite_config: Arc<SpriteSpec>,
            sprite_data: HashMap<(SpriteKind, usize), [SpriteData; SPRITE_LOD_LEVELS]>,
            sprite_col_lights: ColLightInfo,
            sprite_mesh: Mesh<SpriteVertex>,
        }

        let join_handle = std::thread::spawn(move || {
            prof_span!("mesh all sprites");
            // Load all the sprite config data.
            let sprite_config =
                Arc::<SpriteSpec>::load_expect("voxygen.voxel.sprite_manifest").cloned();

            let max_size = Vec2::from(u16::try_from(max_texture_size).unwrap_or(u16::MAX));
            let mut greedy = GreedyMesh::<SpriteAtlasAllocator>::new(
                max_size,
                crate::mesh::greedy::sprite_config(),
            );
            let mut sprite_mesh = Mesh::new();
            // NOTE: Tracks the start vertex of the next model to be meshed.
            let sprite_data: HashMap<(SpriteKind, usize), _> = SpriteKind::iter()
                .filter_map(|kind| Some((kind, sprite_config.get(kind)?)))
                .flat_map(|(kind, sprite_config)| {
                    sprite_config.variations.iter().enumerate().map(
                        move |(
                            variation,
                            SpriteModelConfig {
                                model,
                                offset,
                                lod_axes,
                            },
                        )| {
                            let scaled = [1.0, 0.8, 0.6, 0.4, 0.2];
                            let offset = Vec3::from(*offset);
                            let lod_axes = Vec3::from(*lod_axes);
                            let model = DotVoxAsset::load_expect(model);
                            let zero = Vec3::zero();
                            let model_size = model
                                .read()
                                .0
                                .models
                                .first()
                                .map(
                                    |&dot_vox::Model {
                                         size: dot_vox::Size { x, y, z },
                                         ..
                                     }| Vec3::new(x, y, z),
                                )
                                .unwrap_or(zero);
                            let max_model_size = Vec3::new(31.0, 31.0, 63.0);
                            let model_scale =
                                max_model_size.map2(model_size, |max_sz: f32, cur_sz| {
                                    let scale = max_sz / max_sz.max(cur_sz as f32);
                                    if scale < 1.0 && (cur_sz as f32 * scale).ceil() > max_sz {
                                        scale - 0.001
                                    } else {
                                        scale
                                    }
                                });
                            move |greedy: &mut GreedyMesh<SpriteAtlasAllocator>,
                                  sprite_mesh: &mut Mesh<SpriteVertex>| {
                                prof_span!("mesh sprite");
                                let lod_sprite_data = scaled.map(|lod_scale_orig| {
                                    let lod_scale = model_scale
                                        * if lod_scale_orig == 1.0 {
                                            Vec3::broadcast(1.0)
                                        } else {
                                            lod_axes * lod_scale_orig
                                                + lod_axes.map(|e| if e == 0.0 { 1.0 } else { 0.0 })
                                        };

                                    // Get starting page count of opaque mesh
                                    let start_page_num = sprite_mesh.vertices().len()
                                        / SPRITE_VERT_PAGE_SIZE as usize;
                                    // Mesh generation exclusively acts using side effects; it
                                    // has no interesting return value, but updates the mesh.
                                    generate_mesh_base_vol_sprite(
                                        Segment::from(&model.read().0).scaled_by(lod_scale),
                                        (greedy, sprite_mesh, false),
                                        offset.map(|e: f32| e.floor()) * lod_scale,
                                    );
                                    // Get the number of pages after the model was meshed
                                    let end_page_num = (sprite_mesh.vertices().len()
                                        + SPRITE_VERT_PAGE_SIZE as usize
                                        - 1)
                                        / SPRITE_VERT_PAGE_SIZE as usize;
                                    // Fill the current last page up with degenerate verts
                                    sprite_mesh.vertices_mut_vec().resize_with(
                                        end_page_num * SPRITE_VERT_PAGE_SIZE as usize,
                                        SpriteVertex::default,
                                    );

                                    let sprite_scale = Vec3::one() / lod_scale;

                                    SpriteData {
                                        vert_pages: start_page_num as u32..end_page_num as u32,
                                        scale: sprite_scale,
                                        offset: offset.map(|e| e.rem_euclid(1.0)),
                                    }
                                });

                                ((kind, variation), lod_sprite_data)
                            }
                        },
                    )
                })
                .map(|f| f(&mut greedy, &mut sprite_mesh))
                .collect();

            let sprite_col_lights = {
                prof_span!("finalize");
                greedy.finalize()
            };

            SpriteWorkerResponse {
                sprite_config,
                sprite_data,
                sprite_col_lights,
                sprite_mesh,
            }
        });

        let init = core::cell::OnceCell::new();
        let mut join_handle = Some(join_handle);
        let mut closure = move |renderer: &mut Renderer| {
            // The second unwrap can only fail if the sprite meshing thread panics, which
            // implies that our sprite assets either were not found or did not
            // satisfy the size requirements for meshing, both of which are
            // considered invariant violations.
            let SpriteWorkerResponse {
                sprite_config,
                sprite_data,
                sprite_col_lights,
                sprite_mesh,
            } = join_handle
                .take()
                .expect(
                    "Closure should only be called once (in a `OnceCell::get_or_init`) in the \
                     absence of caught panics!",
                )
                .join()
                .unwrap();

            let sprite_col_lights =
                pipelines::shadow::create_col_lights(renderer, &sprite_col_lights);
            let sprite_col_lights = renderer.sprite_bind_col_light(sprite_col_lights);

            // Write sprite model to a 1D texture
            let sprite_verts_buffer = renderer.create_sprite_verts(sprite_mesh);

            Self {
                // TODO: these are all Arcs, would it makes sense to factor out the Arc?
                sprite_config: Arc::clone(&sprite_config),
                sprite_data: Arc::new(sprite_data),
                sprite_col_lights: Arc::new(sprite_col_lights),
                sprite_verts_buffer: Arc::new(sprite_verts_buffer),
            }
        };
        Box::new(move |renderer| init.get_or_init(|| closure(renderer)).clone())
    }
}

impl<V: RectRasterableVol> Terrain<V> {
    pub fn new(
        renderer: &mut Renderer,
        global_model: &GlobalModel,
        lod_data: &LodData,
        sprite_render_context: SpriteRenderContext,
    ) -> Self {
        // Create a new mpsc (Multiple Produced, Single Consumer) pair for communicating
        // with worker threads that are meshing chunks.
        let (send, recv) = channel::unbounded();

        let (atlas, col_lights) =
            Self::make_atlas(renderer).expect("Failed to create atlas texture");

        Self {
            atlas,
            sprite_config: sprite_render_context.sprite_config,
            chunks: HashMap::default(),
            shadow_chunks: Vec::default(),
            mesh_send_tmp: send,
            mesh_recv: recv,
            mesh_todo: HashMap::default(),
            mesh_todos_active: Arc::new(AtomicU64::new(0)),
            mesh_recv_overflow: 0.0,
            sprite_data: sprite_render_context.sprite_data,
            sprite_col_lights: sprite_render_context.sprite_col_lights,
            sprite_globals: renderer.bind_sprite_globals(
                global_model,
                lod_data,
                &sprite_render_context.sprite_verts_buffer,
            ),
            col_lights: Arc::new(col_lights),
            phantom: PhantomData,
        }
    }

    fn make_atlas(
        renderer: &mut Renderer,
    ) -> Result<(AtlasAllocator, ColLights<pipelines::terrain::Locals>), RenderError> {
        span!(_guard, "make_atlas", "Terrain::make_atlas");
        let max_texture_size = renderer.max_texture_size();
        let atlas_size = guillotiere::Size::new(max_texture_size as i32, max_texture_size as i32);
        let atlas = AtlasAllocator::with_options(atlas_size, &guillotiere::AllocatorOptions {
            // TODO: Verify some good empirical constants.
            small_size_threshold: 128,
            large_size_threshold: 1024,
            ..guillotiere::AllocatorOptions::default()
        });
        let texture = renderer.create_texture_raw(
            &wgpu::TextureDescriptor {
                label: Some("Atlas texture"),
                size: wgpu::Extent3d {
                    width: max_texture_size,
                    height: max_texture_size,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::SAMPLED,
            },
            &wgpu::TextureViewDescriptor {
                label: Some("Atlas texture view"),
                format: Some(wgpu::TextureFormat::Rgba8Unorm),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            },
            &wgpu::SamplerDescriptor {
                label: Some("Atlas sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            },
        );
        let col_light = renderer.terrain_bind_col_light(texture);
        Ok((atlas, col_light))
    }

    fn remove_chunk_meta(&mut self, _pos: Vec2<i32>, chunk: &TerrainChunkData) {
        // No need to free the allocation if the chunk is not allocated in the current
        // atlas, since we don't bother tracking it at that point.
        if let Some(col_lights) = chunk.col_lights_alloc {
            self.atlas.deallocate(col_lights);
        }
        /* let (zmin, zmax) = chunk.z_bounds;
        self.z_index_up.remove(Vec3::from(zmin, pos.x, pos.y));
        self.z_index_down.remove(Vec3::from(zmax, pos.x, pos.y)); */
    }

    fn insert_chunk(&mut self, pos: Vec2<i32>, chunk: TerrainChunkData) {
        if let Some(old) = self.chunks.insert(pos, chunk) {
            self.remove_chunk_meta(pos, &old);
        }
        /* let (zmin, zmax) = chunk.z_bounds;
        self.z_index_up.insert(Vec3::from(zmin, pos.x, pos.y));
        self.z_index_down.insert(Vec3::from(zmax, pos.x, pos.y)); */
    }

    fn remove_chunk(&mut self, pos: Vec2<i32>) {
        if let Some(chunk) = self.chunks.remove(&pos) {
            self.remove_chunk_meta(pos, &chunk);
            // Temporarily remember dead chunks for shadowing purposes.
            self.shadow_chunks.push((pos, chunk));
        }

        if let Some(_todo) = self.mesh_todo.remove(&pos) {
            //Do nothing on todo mesh removal.
        }
    }

    /// Find the light level (sunlight) at the given world position.
    pub fn light_at_wpos(&self, wpos: Vec3<i32>) -> f32 {
        let chunk_pos = Vec2::from(wpos).map2(TerrainChunk::RECT_SIZE, |e: i32, sz| {
            e.div_euclid(sz as i32)
        });
        self.chunks
            .get(&chunk_pos)
            .map(|c| (c.light_map)(wpos))
            .unwrap_or(1.0)
    }

    /// Determine whether a given block change actually require remeshing.
    ///
    /// Returns (skip_color, skip_lights) where
    ///
    /// skip_color means no textures were recolored (i.e. this was a sprite only
    /// change).
    ///
    /// skip_lights means no remeshing or relighting was required
    /// (i.e. the block opacity / lighting info / block kind didn't change).
    fn skip_remesh(old_block: Block, new_block: Block) -> (bool, bool) {
        let same_mesh =
            // Both blocks are of the same opacity and same liquidity (since these are what we use
            // to determine mesh boundaries).
            new_block.is_liquid() == old_block.is_liquid() &&
            new_block.is_opaque() == old_block.is_opaque();
        let skip_lights = same_mesh &&
            // Block glow and sunlight handling are the same (so we don't have to redo
            // lighting).
            new_block.get_glow() == old_block.get_glow() &&
            new_block.get_max_sunlight() == old_block.get_max_sunlight();
        let skip_color = same_mesh &&
            // Both blocks are uncolored
            !new_block.has_color() && !old_block.has_color();
        (skip_color, skip_lights)
    }

    /// Find the glow level (light from lamps) at the given world position.
    pub fn glow_at_wpos(&self, wpos: Vec3<i32>) -> f32 {
        let chunk_pos = Vec2::from(wpos).map2(TerrainChunk::RECT_SIZE, |e: i32, sz| {
            e.div_euclid(sz as i32)
        });
        self.chunks
            .get(&chunk_pos)
            .map(|c| (c.glow_map)(wpos))
            .unwrap_or(0.0)
    }

    pub fn glow_normal_at_wpos(&self, wpos: Vec3<f32>) -> (Vec3<f32>, f32) {
        let wpos_chunk = wpos.xy().map2(TerrainChunk::RECT_SIZE, |e: f32, sz| {
            (e as i32).div_euclid(sz as i32)
        });

        const AMBIANCE: f32 = 0.15; // 0-1, the proportion of light that should illuminate the rear of an object

        let (bias, total) = Spiral2d::new()
            .take(9)
            .flat_map(|rpos| {
                let chunk_pos = wpos_chunk + rpos;
                self.chunks
                    .get(&chunk_pos)
                    .into_iter()
                    .flat_map(|c| c.blocks_of_interest.lights.iter())
                    .filter_map(move |(lpos, level)| {
                        if (*lpos - wpos_chunk).map(|e| e.abs()).reduce_min() < SUNLIGHT as i32 + 2
                        {
                            Some((
                                Vec3::<i32>::from(
                                    chunk_pos * TerrainChunk::RECT_SIZE.map(|e| e as i32),
                                ) + *lpos,
                                level,
                            ))
                        } else {
                            None
                        }
                    })
            })
            .fold(
                (Vec3::broadcast(0.001), 0.0),
                |(bias, total), (lpos, level)| {
                    let rpos = lpos.map(|e| e as f32 + 0.5) - wpos;
                    let level = (*level as f32 - rpos.magnitude()).max(0.0) * SUNLIGHT_INV;
                    (
                        bias + rpos.try_normalized().unwrap_or_else(Vec3::zero) * level,
                        total + level,
                    )
                },
            );

        let bias_factor = bias.magnitude() * (1.0 - AMBIANCE) / total.max(0.001);

        (
            bias.try_normalized().unwrap_or_else(Vec3::zero) * bias_factor.powf(0.5),
            self.glow_at_wpos(wpos.map(|e| e.floor() as i32)),
        )
    }

    /// Maintain terrain data. To be called once per tick.
    ///
    /// The returned visible bounding volumes take into account the current
    /// camera position (i.e: when underground, surface structures will be
    /// culled from the volume).
    pub fn maintain(
        &mut self,
        renderer: &mut Renderer,
        scene_data: &SceneData,
        focus_pos: Vec3<f32>,
        loaded_distance: f32,
        camera: &Camera,
    ) -> (
        Aabb<f32>,
        Vec<math::Vec3<f32>>,
        math::Aabr<f32>,
        Vec<math::Vec3<f32>>,
        math::Aabr<f32>,
    ) {
        let camera::Dependents {
            view_mat,
            proj_mat_treeculler,
            cam_pos,
            ..
        } = camera.dependents();

        // Remove any models for chunks that have been recently removed.
        // Note: Does this before adding to todo list just in case removed chunks were
        // replaced with new chunks (although this would probably be recorded as
        // modified chunks)
        for &pos in &scene_data.state.terrain_changes().removed_chunks {
            self.remove_chunk(pos);
            // Remove neighbors from meshing todo
            for i in -1..2 {
                for j in -1..2 {
                    if i != 0 || j != 0 {
                        self.mesh_todo.remove(&(pos + Vec2::new(i, j)));
                    }
                }
            }
        }

        span!(_guard, "maintain", "Terrain::maintain");
        let current_tick = scene_data.tick;
        let current_time = scene_data.state.get_time();
        // The visible bounding box of all chunks, not including culled regions
        let mut visible_bounding_box: Option<Aabb<f32>> = None;

        // Add any recently created or changed chunks to the list of chunks to be
        // meshed.
        span!(guard, "Add new/modified chunks to mesh todo list");
        for (modified, pos) in scene_data
            .state
            .terrain_changes()
            .modified_chunks
            .iter()
            .map(|c| (true, c))
            .chain(
                scene_data
                    .state
                    .terrain_changes()
                    .new_chunks
                    .iter()
                    .map(|c| (false, c)),
            )
        {
            // TODO: ANOTHER PROBLEM HERE!
            // What happens if the block on the edge of a chunk gets modified? We need to
            // spawn a mesh worker to remesh its neighbour(s) too since their
            // ambient occlusion and face elision information changes too!
            for i in -1..2 {
                for j in -1..2 {
                    let pos = pos + Vec2::new(i, j);

                    if !(self.chunks.contains_key(&pos) || self.mesh_todo.contains_key(&pos))
                        || modified
                    {
                        let mut neighbours = true;
                        for i in -1..2 {
                            for j in -1..2 {
                                neighbours &= scene_data
                                    .state
                                    .terrain()
                                    .contains_key_real(pos + Vec2::new(i, j));
                            }
                        }

                        if neighbours {
                            self.mesh_todo.insert(pos, ChunkMeshState {
                                pos,
                                started_tick: current_tick,
                                is_worker_active: false,
                                skip_remesh: false,
                            });
                        }
                    }
                }
            }
        }
        drop(guard);

        // Add the chunks belonging to recently changed blocks to the list of chunks to
        // be meshed
        span!(guard, "Add chunks with modified blocks to mesh todo list");
        // TODO: would be useful if modified blocks were grouped by chunk
        for (&pos, &old_block) in scene_data.state.terrain_changes().modified_blocks.iter() {
            // terrain_changes() are both set and applied during the same tick on the
            // client, so the current state is the new state and modified_blocks
            // stores the old state.
            let new_block = scene_data.state.get_block(pos);

            let (skip_color, skip_lights) = if let Some(new_block) = new_block {
                Self::skip_remesh(old_block, new_block)
            } else {
                // The block coordinates of a modified block should be in bounds, since they are
                // only retained if setting the block was successful during the state tick in
                // client.  So this is definitely a bug, but we can recover safely by just
                // conservatively doing a full remesh in this case, rather than crashing the
                // game.
                warn!(
                    "Invariant violation: pos={:?} should be a valid block position.  This is a \
                     bug; please contact the developers if you see this error message!",
                    pos
                );
                (false, false)
            };

            // Currently, we can only skip remeshing if both lights and
            // colors don't need to be reworked.
            let skip_remesh = skip_color && skip_lights;

            // TODO: Be cleverer about this to avoid remeshing all neighbours. There are a
            // few things that can create an 'effect at a distance'. These are
            // as follows:
            // - A glowing block is added or removed, thereby causing a lighting
            //   recalculation proportional to its glow radius.
            // - An opaque block that was blocking sunlight from entering a cavity is
            //   removed (or added) thereby
            // changing the way that sunlight propagates into the cavity.
            //
            // We can and should be cleverer about this, but it's non-trivial. For now, we
            // don't remesh if only a block color changed or a sprite was
            // altered in a way that doesn't affect its glow, but we make no
            // attempt to do smarter cavity checking (to see if altering the
            // block changed the sunlight neighbors could get).
            // let block_effect_radius = block.get_glow().unwrap_or(0).max(1);
            let block_effect_radius = crate::mesh::terrain::MAX_LIGHT_DIST;

            // Handle block changes on chunk borders
            // Remesh all neighbours because we have complex lighting now
            // TODO: if lighting is on the server this can be updated to only remesh when
            // lighting changes in that neighbouring chunk or if the block
            // change was on the border
            for x in -1..2 {
                for y in -1..2 {
                    let neighbour_pos = pos + Vec3::new(x, y, 0) * block_effect_radius;
                    let neighbour_chunk_pos = scene_data.state.terrain().pos_key(neighbour_pos);

                    if skip_lights && !(x == 0 && y == 0) {
                        // We don't need to remesh neighboring chunks if this block change doesn't
                        // require relighting.
                        continue;
                    }

                    // Only remesh if this chunk has all its neighbors
                    let mut neighbours = true;
                    for i in -1..2 {
                        for j in -1..2 {
                            neighbours &= scene_data
                                .state
                                .terrain()
                                .contains_key_real(neighbour_chunk_pos + Vec2::new(i, j));
                        }
                    }
                    if neighbours {
                        let todo =
                            self.mesh_todo
                                .entry(neighbour_chunk_pos)
                                .or_insert(ChunkMeshState {
                                    pos: neighbour_chunk_pos,
                                    started_tick: current_tick,
                                    is_worker_active: false,
                                    skip_remesh,
                                });

                        // Make sure not to skip remeshing a chunk if it already had to be
                        // fully meshed for other reasons.  Even if the mesh is currently active
                        // (so relighting would be redundant), we currently have to remesh
                        // everything unless the previous mesh was also able to skip remeshing,
                        // since otherwise the active remesh is computing new lighting values
                        // that we don't have yet.
                        todo.skip_remesh &= skip_remesh;
                        todo.is_worker_active = false;
                        todo.started_tick = current_tick;
                    }
                }
            }
        }
        drop(guard);

        // Limit ourselves to u16::MAX even if larger textures are supported.
        let max_texture_size = renderer.max_texture_size();
        let meshing_cores = match num_cpus::get() as u64 {
            n if n < 4 => 1,
            n if n < 8 => n - 3,
            n => n - 4,
        };

        span!(guard, "Queue meshing from todo list");
        let mesh_focus_pos = focus_pos.map(|e| e.trunc()).xy().as_::<i64>();
        //TODO: this is actually no loop, it just runs for a single entry because of
        // the `min_by_key`. Evaluate actually looping here
        while let Some((todo, chunk)) = self
            .mesh_todo
            .values_mut()
            .filter(|todo| !todo.is_worker_active)
            .min_by_key(|todo| ((todo.pos.as_::<i64>() * TerrainChunk::RECT_SIZE.as_::<i64>()).distance_squared(mesh_focus_pos), todo.started_tick))
            // Find a reference to the actual `TerrainChunk` we're meshing
            .and_then(|todo| {
                let pos = todo.pos;
                Some((todo, scene_data.state
                    .terrain()
                    .get_key_arc(pos)
                    .cloned()
                    .or_else(|| {
                        warn!("Invariant violation: a chunk whose neighbors have not been fetched was found in the todo list,
                              which could halt meshing entirely.");
                        None
                    })?))
            })
        {
            if self.mesh_todos_active.load(Ordering::Relaxed) > meshing_cores {
                break;
            }

            // like ambient occlusion and edge elision, we also need the borders
            // of the chunk's neighbours too (hence the `- 1` and `+ 1`).
            let aabr = Aabr {
                min: todo
                    .pos
                    .map2(VolGrid2d::<V>::chunk_size(), |e, sz| e * sz as i32 - 1),
                max: todo.pos.map2(VolGrid2d::<V>::chunk_size(), |e, sz| {
                    (e + 1) * sz as i32 + 1
                }),
            };

            // Copy out the chunk data we need to perform the meshing. We do this by taking
            // a sample of the terrain that includes both the chunk we want and
            // its neighbours.
            let volume = match scene_data.state.terrain().sample(aabr) {
                Ok(sample) => sample, /* TODO: Ensure that all of the chunk's neighbours still
                                        * exist to avoid buggy shadow borders */
                // Either this chunk or its neighbours doesn't yet exist, so we keep it in the
                // queue to be processed at a later date when we have its neighbours.
                Err(VolGrid2dError::NoSuchChunk) => {
                    continue;
                },
                _ => panic!("Unhandled edge case"),
            };

            // The region to actually mesh
            let min_z = volume
                .iter()
                .fold(i32::MAX, |min, (_, chunk)| chunk.get_min_z().min(min));
            let max_z = volume
                .iter()
                .fold(i32::MIN, |max, (_, chunk)| chunk.get_max_z().max(max));

            let aabb = Aabb {
                min: Vec3::from(aabr.min) + Vec3::unit_z() * (min_z - 2),
                max: Vec3::from(aabr.max) + Vec3::unit_z() * (max_z + 2),
            };

            // Clone various things so that they can be moved into the thread.
            let send = self.mesh_send_tmp.clone();
            let pos = todo.pos;

            let chunks = &self.chunks;
            let skip_remesh = todo
                .skip_remesh
                .then_some(())
                .and_then(|_| chunks.get(&pos))
                .map(|chunk| (Arc::clone(&chunk.light_map), Arc::clone(&chunk.glow_map)));

            // Queue the worker thread.
            let started_tick = todo.started_tick;
            let sprite_data = Arc::clone(&self.sprite_data);
            let sprite_config = Arc::clone(&self.sprite_config);
            let cnt = Arc::clone(&self.mesh_todos_active);
            cnt.fetch_add(1, Ordering::Relaxed);
            scene_data
                .state
                .slow_job_pool()
                .spawn("TERRAIN_MESHING", move || {
                    let sprite_data = sprite_data;
                    let _ = send.send(mesh_worker(
                        pos,
                        (min_z as f32, max_z as f32),
                        skip_remesh,
                        started_tick,
                        volume,
                        max_texture_size as u16,
                        chunk,
                        aabb,
                        &sprite_data,
                        &sprite_config,
                    ));
                    cnt.fetch_sub(1, Ordering::Relaxed);
                });
            todo.is_worker_active = true;
        }
        drop(guard);

        // Receive a chunk mesh from a worker thread and upload it to the GPU, then
        // store it. Vary the rate at which we pull items out to correlate with the
        // framerate, preventing tail latency.
        span!(guard, "Get/upload meshed chunk");
        const CHUNKS_PER_SECOND: f32 = 240.0;
        let recv_count =
            scene_data.state.get_delta_time() * CHUNKS_PER_SECOND + self.mesh_recv_overflow;
        self.mesh_recv_overflow = recv_count.fract();
        let incoming_chunks =
            std::iter::from_fn(|| self.mesh_recv.recv_timeout(Duration::new(0, 0)).ok())
                .take(recv_count.floor() as usize)
                .collect::<Vec<_>>(); // Avoid ownership issue
        for response in incoming_chunks {
            match self.mesh_todo.get(&response.pos) {
                // It's the mesh we want, insert the newly finished model into the terrain model
                // data structure (convert the mesh to a model first of course).
                Some(todo) if response.started_tick <= todo.started_tick => {
                    let started_tick = todo.started_tick;

                    let sprite_instances =
                        response.sprite_instances.map(|(instances, alt_indices)| {
                            (
                                renderer
                                    .create_instances(&instances)
                                    .expect("Failed to upload chunk sprite instances to the GPU!"),
                                alt_indices,
                            )
                        });

                    if let Some(mesh) = response.mesh {
                        // Full update, insert the whole chunk.

                        let load_time = self
                            .chunks
                            .get(&response.pos)
                            .map(|chunk| chunk.load_time)
                            .unwrap_or(current_time as f32);
                        // TODO: Allocate new atlas on allocation failure.
                        let (tex, tex_size) = mesh.col_lights_info;
                        let atlas = &mut self.atlas;
                        let chunks = &mut self.chunks;
                        let col_lights = &mut self.col_lights;
                        let alloc_size =
                            guillotiere::Size::new(i32::from(tex_size.x), i32::from(tex_size.y));

                        let allocation = atlas.allocate(alloc_size).unwrap_or_else(|| {
                            // Atlas allocation failure: try allocating a new texture and atlas.
                            let (new_atlas, new_col_lights) =
                                Self::make_atlas(renderer).expect("Failed to create atlas texture");

                            // We reset the atlas and clear allocations from existing chunks,
                            // even though we haven't yet
                            // checked whether the new allocation can fit in
                            // the texture.  This is reasonable because we don't have a fallback
                            // if a single chunk can't fit in an empty atlas of maximum size.
                            //
                            // TODO: Consider attempting defragmentation first rather than just
                            // always moving everything into the new chunk.
                            chunks.iter_mut().for_each(|(_, chunk)| {
                                chunk.col_lights_alloc = None;
                            });
                            *atlas = new_atlas;
                            *col_lights = Arc::new(new_col_lights);

                            atlas
                                .allocate(alloc_size)
                                .expect("Chunk data does not fit in a texture of maximum size.")
                        });

                        // NOTE: Cast is safe since the origin was a u16.
                        let atlas_offs = Vec2::new(
                            allocation.rectangle.min.x as u32,
                            allocation.rectangle.min.y as u32,
                        );
                        renderer.update_texture(
                            &col_lights.texture,
                            atlas_offs.into_array(),
                            tex_size.map(u32::from).into_array(),
                            &tex,
                        );

                        self.insert_chunk(response.pos, TerrainChunkData {
                            load_time,
                            opaque_model: renderer.create_model(&mesh.opaque_mesh),
                            fluid_model: renderer.create_model(&mesh.fluid_mesh),
                            col_lights_alloc: Some(allocation.id),
                            col_lights: Arc::clone(&self.col_lights),
                            light_map: mesh.light_map,
                            glow_map: mesh.glow_map,
                            sprite_instances,
                            locals: renderer.create_terrain_bound_locals(&[TerrainLocals::new(
                                Vec3::from(
                                    response.pos.map2(VolGrid2d::<V>::chunk_size(), |e, sz| {
                                        e as f32 * sz as f32
                                    }),
                                ),
                                atlas_offs,
                                load_time,
                            )]),
                            visible: Visibility {
                                in_range: false,
                                in_frustum: false,
                            },
                            can_shadow_point: false,
                            can_shadow_sun: false,
                            blocks_of_interest: response.blocks_of_interest,
                            z_bounds: mesh.z_bounds,
                            sun_occluder_z_bounds: mesh.sun_occluder_z_bounds,
                            frustum_last_plane_index: 0,
                            alt_indices: mesh.alt_indices,
                        });
                    } else if let Some(chunk) = self.chunks.get_mut(&response.pos) {
                        // There was an update that didn't require a remesh (probably related to
                        // non-glowing sprites) so we just update those.
                        chunk.sprite_instances = sprite_instances;
                        chunk.blocks_of_interest = response.blocks_of_interest;
                    }

                    if response.started_tick == started_tick {
                        self.mesh_todo.remove(&response.pos);
                    }
                },
                // Chunk must have been removed, or it was spawned on an old tick. Drop the mesh
                // since it's either out of date or no longer needed.
                Some(_todo) => {},
                None => {},
            }
        }
        drop(guard);

        // Construct view frustum
        span!(guard, "Construct view frustum");
        let focus_off = focus_pos.map(|e| e.trunc());
        let frustum = Frustum::from_modelview_projection(
            (proj_mat_treeculler * view_mat * Mat4::translation_3d(-focus_off)).into_col_arrays(),
        );
        drop(guard);

        // Update chunk visibility
        span!(guard, "Update chunk visibility");
        let chunk_sz = V::RECT_SIZE.x as f32;
        for (pos, chunk) in &mut self.chunks {
            let chunk_pos = pos.as_::<f32>() * chunk_sz;

            chunk.can_shadow_sun = false;

            // Limit focus_pos to chunk bounds and ensure the chunk is within the fog
            // boundary
            let nearest_in_chunk = Vec2::from(focus_pos).clamped(chunk_pos, chunk_pos + chunk_sz);
            let distance_2 = Vec2::<f32>::from(focus_pos).distance_squared(nearest_in_chunk);
            let in_range = distance_2 < loaded_distance.powi(2);

            chunk.visible.in_range = in_range;

            // Ensure the chunk is within the view frustum
            let chunk_min = [chunk_pos.x, chunk_pos.y, chunk.z_bounds.0];
            let chunk_max = [
                chunk_pos.x + chunk_sz,
                chunk_pos.y + chunk_sz,
                chunk.sun_occluder_z_bounds.1,
            ];

            let (in_frustum, last_plane_index) = AABB::new(chunk_min, chunk_max)
                .coherent_test_against_frustum(&frustum, chunk.frustum_last_plane_index);

            chunk.frustum_last_plane_index = last_plane_index;
            chunk.visible.in_frustum = in_frustum;
            let chunk_area = Aabr {
                min: chunk_pos,
                max: chunk_pos + chunk_sz,
            };

            if in_frustum {
                let visible_box = Aabb {
                    min: chunk_area.min.with_z(chunk.sun_occluder_z_bounds.0),
                    max: chunk_area.max.with_z(chunk.sun_occluder_z_bounds.1),
                };
                visible_bounding_box = visible_bounding_box
                    .map(|e| e.union(visible_box))
                    .or(Some(visible_box));
            }
            // FIXME: Hack that only works when only the lantern casts point shadows
            // (and hardcodes the shadow distance).  Should ideally exist per-light, too.
            chunk.can_shadow_point = distance_2 < (128.0 * 128.0);
        }
        drop(guard);

        span!(guard, "Shadow magic");
        // PSRs: potential shadow receivers
        let visible_bounding_box = visible_bounding_box.unwrap_or(Aabb {
            min: focus_pos - 2.0,
            max: focus_pos + 2.0,
        });
        let inv_proj_view =
            math::Mat4::from_col_arrays((proj_mat_treeculler * view_mat).into_col_arrays())
                .as_::<f64>()
                .inverted();

        // PSCs: Potential shadow casters
        let ray_direction = scene_data.get_sun_dir();
        let collides_with_aabr = |a: math::Aabb<f32>, b: math::Aabr<f32>| {
            let min = math::Vec4::new(a.min.x, a.min.y, b.min.x, b.min.y);
            let max = math::Vec4::new(b.max.x, b.max.y, a.max.x, a.max.y);
            #[cfg(feature = "simd")]
            return min.partial_cmple_simd(max).reduce_and();
            #[cfg(not(feature = "simd"))]
            return min.partial_cmple(&max).reduce_and();
        };

        let (visible_light_volume, visible_psr_bounds) = if ray_direction.z < 0.0
            && renderer.pipeline_modes().shadow.is_map()
        {
            let visible_bounding_box = math::Aabb::<f32> {
                min: math::Vec3::from(visible_bounding_box.min - focus_off),
                max: math::Vec3::from(visible_bounding_box.max - focus_off),
            };
            let focus_off = math::Vec3::from(focus_off);
            let visible_bounds_fine = visible_bounding_box.as_::<f64>();
            let ray_direction = math::Vec3::<f32>::from(ray_direction);
            // NOTE: We use proj_mat_treeculler here because
            // calc_focused_light_volume_points makes the assumption that the
            // near plane lies before the far plane.
            let visible_light_volume = math::calc_focused_light_volume_points(
                inv_proj_view,
                ray_direction.as_::<f64>(),
                visible_bounds_fine,
                1e-6,
            )
            .map(|v| v.as_::<f32>())
            .collect::<Vec<_>>();

            let up: math::Vec3<f32> = { math::Vec3::unit_y() };
            let cam_pos = math::Vec3::from(cam_pos);
            let ray_mat = math::Mat4::look_at_rh(cam_pos, cam_pos + ray_direction, up);
            let visible_bounds = math::Aabr::from(math::fit_psr(
                ray_mat,
                visible_light_volume.iter().copied(),
                |p| p,
            ));
            let ray_mat = ray_mat * math::Mat4::translation_3d(-focus_off);

            let can_shadow_sun = |pos: Vec2<i32>, chunk: &TerrainChunkData| {
                let chunk_pos = pos.as_::<f32>() * chunk_sz;

                // Ensure the chunk is within the PSR set.
                let chunk_box = math::Aabb {
                    min: math::Vec3::new(chunk_pos.x, chunk_pos.y, chunk.z_bounds.0),
                    max: math::Vec3::new(
                        chunk_pos.x + chunk_sz,
                        chunk_pos.y + chunk_sz,
                        chunk.z_bounds.1,
                    ),
                };

                let chunk_from_light = math::fit_psr(
                    ray_mat,
                    math::aabb_to_points(chunk_box).iter().copied(),
                    |p| p,
                );
                collides_with_aabr(chunk_from_light, visible_bounds)
            };

            // Handle potential shadow casters (chunks that aren't visible, but are still in
            // range) to see if they could cast shadows.
            self.chunks.iter_mut()
                // NOTE: We deliberately avoid doing this computation for chunks we already know
                // are visible, since by definition they'll always intersect the visible view
                // frustum.
                .filter(|chunk| !chunk.1.visible.in_frustum)
                .for_each(|(&pos, chunk)| {
                    chunk.can_shadow_sun = can_shadow_sun(pos, chunk);
                });

            // Handle dead chunks that we kept around only to make sure shadows don't blink
            // out when a chunk disappears.
            //
            // If the sun can currently cast shadows, we retain only those shadow chunks
            // that both: 1. have not been replaced by a real chunk instance,
            // and 2. are currently potential shadow casters (as witnessed by
            // `can_shadow_sun` returning true).
            //
            // NOTE: Please make sure this runs *after* any code that could insert a chunk!
            // Otherwise we may end up with multiple instances of the chunk trying to cast
            // shadows at the same time.
            let chunks = &self.chunks;
            self.shadow_chunks
                .retain(|(pos, chunk)| !chunks.contains_key(pos) && can_shadow_sun(*pos, chunk));

            (visible_light_volume, visible_bounds)
        } else {
            // There's no daylight or no shadows, so there's no reason to keep any
            // shadow chunks around.
            self.shadow_chunks.clear();
            (Vec::new(), math::Aabr {
                min: math::Vec2::zero(),
                max: math::Vec2::zero(),
            })
        };
        drop(guard);
        span!(guard, "Rain occlusion magic");
        // Check if there is rain near the camera
        let max_weather = scene_data
            .state
            .max_weather_near(focus_off.xy() + cam_pos.xy());
        let (visible_occlusion_volume, visible_por_bounds) = if max_weather.rain > RAIN_THRESHOLD {
            let visible_bounding_box = math::Aabb::<f32> {
                min: math::Vec3::from(visible_bounding_box.min - focus_off),
                max: math::Vec3::from(visible_bounding_box.max - focus_off),
            };
            let visible_bounds_fine = math::Aabb {
                min: visible_bounding_box.min.as_::<f64>(),
                max: visible_bounding_box.max.as_::<f64>(),
            };
            let weather = scene_data.state.weather_at(focus_off.xy() + cam_pos.xy());
            let ray_direction = math::Vec3::<f32>::from(weather.rain_vel().normalized());

            // NOTE: We use proj_mat_treeculler here because
            // calc_focused_light_volume_points makes the assumption that the
            // near plane lies before the far plane.
            let visible_volume = math::calc_focused_light_volume_points(
                inv_proj_view,
                ray_direction.as_::<f64>(),
                visible_bounds_fine,
                1e-6,
            )
            .map(|v| v.as_::<f32>())
            .collect::<Vec<_>>();
            let cam_pos = math::Vec3::from(cam_pos);
            let ray_mat =
                math::Mat4::look_at_rh(cam_pos, cam_pos + ray_direction, math::Vec3::unit_y());
            let visible_bounds = math::Aabr::from(math::fit_psr(
                ray_mat,
                visible_volume.iter().copied(),
                |p| p,
            ));

            (visible_volume, visible_bounds)
        } else {
            (Vec::new(), math::Aabr::default())
        };

        drop(guard);
        (
            visible_bounding_box,
            visible_light_volume,
            visible_psr_bounds,
            visible_occlusion_volume,
            visible_por_bounds,
        )
    }

    pub fn get(&self, chunk_key: Vec2<i32>) -> Option<&TerrainChunkData> {
        self.chunks.get(&chunk_key)
    }

    pub fn chunk_count(&self) -> usize { self.chunks.len() }

    pub fn visible_chunk_count(&self) -> usize {
        self.chunks
            .iter()
            .filter(|(_, c)| c.visible.is_visible())
            .count()
    }

    pub fn shadow_chunk_count(&self) -> usize { self.shadow_chunks.len() }

    pub fn render_shadows<'a>(
        &'a self,
        drawer: &mut TerrainShadowDrawer<'_, 'a>,
        focus_pos: Vec3<f32>,
        culling_mode: CullingMode,
    ) {
        span!(_guard, "render_shadows", "Terrain::render_shadows");
        let focus_chunk = Vec2::from(focus_pos).map2(TerrainChunk::RECT_SIZE, |e: f32, sz| {
            (e as i32).div_euclid(sz as i32)
        });

        let chunk_iter = Spiral2d::new()
            .filter_map(|rpos| {
                let pos = focus_chunk + rpos;
                self.chunks.get(&pos)
            })
            .take(self.chunks.len());

        // Directed shadows
        //
        // NOTE: We also render shadows for dead chunks that were found to still be
        // potential shadow casters, to avoid shadows suddenly disappearing at
        // very steep sun angles (e.g. sunrise / sunset).
        chunk_iter
            .filter(|chunk| chunk.can_shadow_sun())
            .chain(self.shadow_chunks.iter().map(|(_, chunk)| chunk))
            .filter_map(|chunk| {
                Some((
                    chunk.opaque_model.as_ref()?,
                    &chunk.locals,
                    &chunk.alt_indices,
                ))
            })
            .for_each(|(model, locals, alt_indices)| {
                drawer.draw(model, locals, alt_indices, culling_mode)
            });
    }

    pub fn render_rain_occlusion<'a>(
        &'a self,
        drawer: &mut TerrainShadowDrawer<'_, 'a>,
        focus_pos: Vec3<f32>,
    ) {
        span!(_guard, "render_occlusion", "Terrain::render_occlusion");
        let focus_chunk = Vec2::from(focus_pos).map2(TerrainChunk::RECT_SIZE, |e: f32, sz| {
            (e as i32).div_euclid(sz as i32)
        });
        let chunk_iter = Spiral2d::new()
            .filter_map(|rpos| {
                let pos = focus_chunk + rpos;
                self.chunks.get(&pos)
            })
            .take(self.chunks.len().min(RAIN_OCCLUSION_CHUNKS));

        chunk_iter
            // Find a way to keep this?
            // .filter(|chunk| chunk.can_shadow_sun())
            .filter_map(|chunk| Some((
                chunk
                    .opaque_model
                    .as_ref()?,
                &chunk.locals,
                &chunk.alt_indices,
            )))
            .for_each(|(model, locals, alt_indices)| drawer.draw(model, locals, alt_indices, CullingMode::None));
    }

    pub fn chunks_for_point_shadows(
        &self,
        focus_pos: Vec3<f32>,
    ) -> impl Clone
    + Iterator<
        Item = (
            &Model<pipelines::terrain::Vertex>,
            &pipelines::terrain::BoundLocals,
        ),
    > {
        let focus_chunk = Vec2::from(focus_pos).map2(TerrainChunk::RECT_SIZE, |e: f32, sz| {
            (e as i32).div_euclid(sz as i32)
        });

        let chunk_iter = Spiral2d::new()
            .filter_map(move |rpos| {
                let pos = focus_chunk + rpos;
                self.chunks.get(&pos)
            })
            .take(self.chunks.len());

        // Point shadows
        //
        // NOTE: We don't bother retaining chunks unless they cast sun shadows, so we
        // don't use `shadow_chunks` here.
        chunk_iter
            .filter(|chunk| chunk.can_shadow_point)
            .filter_map(|chunk| {
                chunk
                    .opaque_model
                    .as_ref()
                    .map(|model| (model, &chunk.locals))
            })
    }

    pub fn render<'a>(
        &'a self,
        drawer: &mut FirstPassDrawer<'a>,
        focus_pos: Vec3<f32>,
        culling_mode: CullingMode,
    ) {
        span!(_guard, "render", "Terrain::render");
        let mut drawer = drawer.draw_terrain();

        let focus_chunk = Vec2::from(focus_pos).map2(TerrainChunk::RECT_SIZE, |e: f32, sz| {
            (e as i32).div_euclid(sz as i32)
        });

        Spiral2d::new()
            .filter_map(|rpos| {
                let pos = focus_chunk + rpos;
                Some((rpos, self.chunks.get(&pos)?))
            })
            .take(self.chunks.len())
            .filter(|(_, chunk)| chunk.visible.is_visible())
            .filter_map(|(rpos, chunk)| {
                Some((
                    rpos,
                    chunk.opaque_model.as_ref()?,
                    &chunk.col_lights,
                    &chunk.locals,
                    &chunk.alt_indices,
                ))
            })
            .for_each(|(rpos, model, col_lights, locals, alt_indices)| {
                // Always draw all of close chunks to avoid terrain 'popping'
                let culling_mode = if rpos.magnitude_squared() < NEVER_CULL_DIST.pow(2) {
                    CullingMode::None
                } else {
                    culling_mode
                };
                drawer.draw(model, col_lights, locals, alt_indices, culling_mode)
            });
    }

    pub fn render_translucent<'a>(
        &'a self,
        drawer: &mut FirstPassDrawer<'a>,
        focus_pos: Vec3<f32>,
        cam_pos: Vec3<f32>,
        sprite_render_distance: f32,
        culling_mode: CullingMode,
    ) {
        span!(_guard, "render_translucent", "Terrain::render_translucent");
        let focus_chunk = Vec2::from(focus_pos).map2(TerrainChunk::RECT_SIZE, |e: f32, sz| {
            (e as i32).div_euclid(sz as i32)
        });

        // Avoid switching textures
        let chunk_iter = Spiral2d::new()
            .filter_map(|rpos| {
                let pos = focus_chunk + rpos;
                Some((rpos, pos, self.chunks.get(&pos)?))
            })
            .take(self.chunks.len());

        // Terrain sprites
        // TODO: move to separate functions
        span!(guard, "Terrain sprites");
        let chunk_size = V::RECT_SIZE.map(|e| e as f32);

        let sprite_low_detail_distance = sprite_render_distance * 0.75;
        let sprite_mid_detail_distance = sprite_render_distance * 0.5;
        let sprite_hid_detail_distance = sprite_render_distance * 0.35;
        let sprite_high_detail_distance = sprite_render_distance * 0.15;

        let mut sprite_drawer = drawer.draw_sprites(&self.sprite_globals, &self.sprite_col_lights);
        chunk_iter
            .clone()
            .filter(|(_, _, c)| c.visible.is_visible())
            .for_each(|(rpos, pos, chunk)| {
                // Skip chunk if it has no sprites
                if chunk.sprite_instances[0].0.count() == 0 {
                    return;
                }

                let chunk_center = pos.map2(chunk_size, |e, sz| (e as f32 + 0.5) * sz);
                let focus_dist_sqrd = Vec2::from(focus_pos).distance_squared(chunk_center);
                let dist_sqrd = Aabr {
                    min: chunk_center - chunk_size * 0.5,
                    max: chunk_center + chunk_size * 0.5,
                }
                .projected_point(cam_pos.xy())
                .distance_squared(cam_pos.xy());

                if focus_dist_sqrd < sprite_render_distance.powi(2) {
                    let lod_level = if dist_sqrd < sprite_high_detail_distance.powi(2) {
                        0
                    } else if dist_sqrd < sprite_hid_detail_distance.powi(2) {
                        1
                    } else if dist_sqrd < sprite_mid_detail_distance.powi(2) {
                        2
                    } else if dist_sqrd < sprite_low_detail_distance.powi(2) {
                        3
                    } else {
                        4
                    };

                    // Always draw all of close chunks to avoid terrain 'popping'
                    let culling_mode = if rpos.magnitude_squared() < NEVER_CULL_DIST.pow(2) {
                        CullingMode::None
                    } else {
                        culling_mode
                    };

                    sprite_drawer.draw(
                        &chunk.locals,
                        &chunk.sprite_instances[lod_level].0,
                        &chunk.sprite_instances[lod_level].1,
                        culling_mode,
                    );
                }
            });
        drop(sprite_drawer);
        drop(guard);

        // Translucent
        span!(guard, "Fluid chunks");
        let mut fluid_drawer = drawer.draw_fluid();
        chunk_iter
            .filter(|(_, _, chunk)| chunk.visible.is_visible())
            .filter_map(|(_, _, chunk)| {
                chunk
                    .fluid_model
                    .as_ref()
                    .map(|model| (model, &chunk.locals))
            })
            .collect::<Vec<_>>()
            .into_iter()
            .rev() // Render back-to-front
            .for_each(|(model, locals)| {
                fluid_drawer.draw(
                    model,
                    locals,
                )
            });
        drop(fluid_drawer);
        drop(guard);
    }
}
