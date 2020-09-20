mod watcher;

pub use self::watcher::BlocksOfInterest;

use crate::{
    mesh::{greedy::GreedyMesh, Meshable},
    render::{
        ColLightFmt, ColLightInfo, Consts, FluidPipeline, GlobalModel, Instances, Mesh, Model,
        RenderError, Renderer, ShadowPipeline, SpriteInstance, SpriteLocals, SpritePipeline,
        TerrainLocals, TerrainPipeline, Texture,
    },
};

use super::{math, LodData, SceneData};
use common::{
    assets::{Asset, Ron},
    figure::Segment,
    span,
    spiral::Spiral2d,
    terrain::{sprite, Block, SpriteKind, TerrainChunk},
    vol::{BaseVol, ReadVol, RectRasterableVol, SampleVol, Vox},
    volumes::vol_grid_2d::{VolGrid2d, VolGrid2dError},
};
use core::{f32, fmt::Debug, i32, marker::PhantomData, time::Duration};
use crossbeam::channel;
use dot_vox::DotVoxData;
use enum_iterator::IntoEnumIterator;
use guillotiere::AtlasAllocator;
use hashbrown::HashMap;
use image::DynamicImage;
use serde::Deserialize;
use std::sync::Arc;
use tracing::warn;
use treeculler::{BVol, Frustum, AABB};
use vek::*;

const SPRITE_SCALE: Vec3<f32> = Vec3::new(1.0 / 11.0, 1.0 / 11.0, 1.0 / 11.0);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Visibility {
    OutOfRange = 0,
    InRange = 1,
    Visible = 2,
}

pub struct TerrainChunkData {
    // GPU data
    load_time: f32,
    opaque_model: Model<TerrainPipeline>,
    fluid_model: Option<Model<FluidPipeline>>,
    col_lights: guillotiere::AllocId,
    sprite_instances: HashMap<(SpriteKind, usize), Instances<SpriteInstance>>,
    locals: Consts<TerrainLocals>,
    pub blocks_of_interest: BlocksOfInterest,

    visible: Visibility,
    can_shadow_point: bool,
    can_shadow_sun: bool,
    z_bounds: (f32, f32),
    frustum_last_plane_index: u8,
}

#[derive(Copy, Clone)]
struct ChunkMeshState {
    pos: Vec2<i32>,
    started_tick: u64,
    is_worker_active: bool,
}

/// A type produced by mesh worker threads corresponding to the position and
/// mesh of a chunk.
struct MeshWorkerResponse {
    pos: Vec2<i32>,
    z_bounds: (f32, f32),
    opaque_mesh: Mesh<TerrainPipeline>,
    fluid_mesh: Mesh<FluidPipeline>,
    col_lights_info: ColLightInfo,
    sprite_instances: HashMap<(SpriteKind, usize), Vec<SpriteInstance>>,
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

/// Configuration data for all sprite models.
///
/// NOTE: Model is an asset path to the appropriate sprite .vox model.
type SpriteSpec = sprite::sprite_kind::PureCases<Option<SpriteConfig<String>>>;

/// Function executed by worker threads dedicated to chunk meshing.
#[allow(clippy::or_fun_call)] // TODO: Pending review in #587

fn mesh_worker<V: BaseVol<Vox = Block> + RectRasterableVol + ReadVol + Debug>(
    pos: Vec2<i32>,
    z_bounds: (f32, f32),
    started_tick: u64,
    volume: <VolGrid2d<V> as SampleVol<Aabr<i32>>>::Sample,
    max_texture_size: u16,
    chunk: Arc<TerrainChunk>,
    range: Aabb<i32>,
    sprite_data: &HashMap<(SpriteKind, usize), Vec<SpriteData>>,
    sprite_config: &SpriteSpec,
) -> MeshWorkerResponse {
    span!(_guard, "mesh_worker");
    let (opaque_mesh, fluid_mesh, _shadow_mesh, (bounds, col_lights_info)) =
        volume.generate_mesh((range, Vec2::new(max_texture_size, max_texture_size)));
    MeshWorkerResponse {
        pos,
        z_bounds: (bounds.min.z, bounds.max.z),
        opaque_mesh,
        fluid_mesh,
        col_lights_info,
        // Extract sprite locations from volume
        sprite_instances: {
            span!(_guard, "extract sprite_instances");
            let mut instances = HashMap::new();

            for x in 0..V::RECT_SIZE.x as i32 {
                for y in 0..V::RECT_SIZE.y as i32 {
                    for z in z_bounds.0 as i32..z_bounds.1 as i32 + 1 {
                        let rel_pos = Vec3::new(x, y, z);
                        let wpos = Vec3::from(pos * V::RECT_SIZE.map(|e: u32| e as i32)) + rel_pos;

                        let block = volume.get(wpos).ok().copied().unwrap_or(Block::empty());
                        let sprite = if let Some(sprite) = block.get_sprite() {
                            sprite
                        } else {
                            continue;
                        };

                        if let Some(cfg) = sprite.elim_case_pure(&sprite_config) {
                            let seed = wpos.x as u64 * 3
                                + wpos.y as u64 * 7
                                + wpos.x as u64 * wpos.y as u64; // Awful PRNG
                            let ori = (block.get_ori().unwrap_or((seed % 4) as u8 * 2)) & 0b111;
                            let variation = seed as usize % cfg.variations.len();
                            let key = (sprite, variation);
                            // NOTE: Safe because we called sprite_config_for already.
                            // NOTE: Safe because 0 ≤ ori < 8
                            let sprite_data = &sprite_data[&key][0];
                            let instance = SpriteInstance::new(
                                Mat4::identity()
                                    .translated_3d(sprite_data.offset)
                                    .rotated_z(f32::consts::PI * 0.25 * ori as f32)
                                    .translated_3d(
                                        (rel_pos.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0))
                                            / SPRITE_SCALE,
                                    ),
                                cfg.wind_sway,
                                rel_pos,
                                ori,
                            );

                            instances.entry(key).or_insert(Vec::new()).push(instance);
                        }
                    }
                }
            }

            instances
        },
        blocks_of_interest: BlocksOfInterest::from_chunk(&chunk),
        started_tick,
    }
}

struct SpriteData {
    /* mat: Mat4<f32>, */
    locals: Consts<SpriteLocals>,
    model: Model<SpritePipeline>,
    /* scale: Vec3<f32>, */
    offset: Vec3<f32>,
}

pub struct Terrain<V: RectRasterableVol> {
    atlas: AtlasAllocator,
    sprite_config: Arc<SpriteSpec>,
    chunks: HashMap<Vec2<i32>, TerrainChunkData>,
    /// Temporary storage for dead chunks that might still be shadowing chunks
    /// in view.  We wait until either the chunk definitely cannot be
    /// shadowing anything the player can see, the chunk comes back into
    /// view, or for daylight to end, before removing it (whichever comes
    /// first).
    ///
    /// Note that these chunks are not complete; for example, they are missing
    /// texture data.
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

    // GPU data
    sprite_data: Arc<HashMap<(SpriteKind, usize), Vec<SpriteData>>>,
    sprite_col_lights: Texture<ColLightFmt>,
    col_lights: Texture<ColLightFmt>,
    waves: Texture,

    phantom: PhantomData<V>,
}

impl TerrainChunkData {
    pub fn can_shadow_sun(&self) -> bool {
        self.visible == Visibility::Visible || self.can_shadow_sun
    }
}

impl<V: RectRasterableVol> Terrain<V> {
    #[allow(clippy::float_cmp)] // TODO: Pending review in #587
    pub fn new(renderer: &mut Renderer) -> Self {
        // Load all the sprite config data.
        let sprite_config = Ron::<SpriteSpec>::load("voxygen.voxel.sprite_manifest")
            .expect("Failed to find sprite model data!");

        // Create a new mpsc (Multiple Produced, Single Consumer) pair for communicating
        // with worker threads that are meshing chunks.
        let (send, recv) = channel::unbounded();

        let (atlas, col_lights) =
            Self::make_atlas(renderer).expect("Failed to create atlas texture");

        let max_texture_size = renderer.max_texture_size();
        let max_size =
            guillotiere::Size::new(i32::from(max_texture_size), i32::from(max_texture_size));
        let mut greedy = GreedyMesh::new(max_size);
        let mut locals_buffer = [SpriteLocals::default(); 8];
        let sprite_config_ = &sprite_config;
        // NOTE: Tracks the start vertex of the next model to be meshed.

        let sprite_data: HashMap<(SpriteKind, usize), _> = SpriteKind::into_enum_iter()
            .filter_map(|kind| Some((kind, kind.elim_case_pure(&sprite_config_).as_ref()?)))
            .flat_map(|(kind, sprite_config)| {
                let wind_sway = sprite_config.wind_sway;
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
                        let model = DotVoxData::load_expect(model);
                        let zero = Vec3::zero();
                        let model_size = model
                            .models
                            .first()
                            .map(
                                |&dot_vox::Model {
                                     size: dot_vox::Size { x, y, z },
                                     ..
                                 }| Vec3::new(x, y, z),
                            )
                            .unwrap_or(zero);
                        let max_model_size = Vec3::new(15.0, 15.0, 63.0);
                        let model_scale = max_model_size.map2(model_size, |max_sz: f32, cur_sz| {
                            let scale = max_sz / max_sz.max(cur_sz as f32);
                            if scale < 1.0 && (cur_sz as f32 * scale).ceil() > max_sz {
                                scale - 0.001
                            } else {
                                scale
                            }
                        });
                        let sprite_mat: Mat4<f32> =
                            Mat4::translation_3d(offset).scaled_3d(SPRITE_SCALE);
                        move |greedy: &mut GreedyMesh, renderer: &mut Renderer| {
                            (
                                (kind, variation),
                                scaled
                                    .iter()
                                    .map(|&lod_scale_orig| {
                                        let lod_scale = model_scale
                                            * if lod_scale_orig == 1.0 {
                                                Vec3::broadcast(1.0)
                                            } else {
                                                lod_axes * lod_scale_orig
                                                    + lod_axes
                                                        .map(|e| if e == 0.0 { 1.0 } else { 0.0 })
                                            };
                                        // Mesh generation exclusively acts using side effects; it
                                        // has no
                                        // interesting return value, but updates the mesh.
                                        let mut opaque_mesh = Mesh::new();
                                        Meshable::<SpritePipeline, &mut GreedyMesh>::generate_mesh(
                                            Segment::from(model.as_ref()).scaled_by(lod_scale),
                                            (
                                                greedy,
                                                &mut opaque_mesh,
                                                wind_sway >= 0.4 && lod_scale_orig == 1.0,
                                            ),
                                        );
                                        let model = renderer.create_model(&opaque_mesh).expect(
                                            "Failed to upload sprite model data to the GPU!",
                                        );

                                        let sprite_scale = Vec3::one() / lod_scale;
                                        let sprite_mat: Mat4<f32> =
                                            sprite_mat * Mat4::scaling_3d(sprite_scale);
                                        locals_buffer.iter_mut().enumerate().for_each(
                                            |(ori, locals)| {
                                                let sprite_mat = sprite_mat
                                                    .rotated_z(f32::consts::PI * 0.25 * ori as f32);
                                                *locals = SpriteLocals::new(
                                                    sprite_mat,
                                                    sprite_scale,
                                                    offset,
                                                    wind_sway,
                                                );
                                            },
                                        );

                                        SpriteData {
                                            /* vertex_range */ model,
                                            offset,
                                            locals: renderer.create_consts(&locals_buffer).expect(
                                                "Failed to upload sprite locals to the GPU!",
                                            ),
                                        }
                                    })
                                    .collect::<Vec<_>>(),
                            )
                        }
                    },
                )
            })
            .map(|mut f| f(&mut greedy, renderer))
            .collect();

        let sprite_col_lights = ShadowPipeline::create_col_lights(renderer, greedy.finalize())
            .expect("Failed to upload sprite color and light data to the GPU!");

        Self {
            atlas,
            sprite_config,
            chunks: HashMap::default(),
            shadow_chunks: Vec::default(),
            mesh_send_tmp: send,
            mesh_recv: recv,
            mesh_todo: HashMap::default(),
            sprite_data: Arc::new(sprite_data),
            sprite_col_lights,
            waves: renderer
                .create_texture(
                    &DynamicImage::load_expect("voxygen.texture.waves"),
                    Some(gfx::texture::FilterMethod::Trilinear),
                    Some(gfx::texture::WrapMode::Tile),
                    None,
                )
                .expect("Failed to create wave texture"),
            col_lights,
            phantom: PhantomData,
        }
    }

    fn make_atlas(
        renderer: &mut Renderer,
    ) -> Result<(AtlasAllocator, Texture<ColLightFmt>), RenderError> {
        span!(_guard, "make_atlas", "Terrain::make_atlas");
        let max_texture_size = renderer.max_texture_size();
        let atlas_size =
            guillotiere::Size::new(i32::from(max_texture_size), i32::from(max_texture_size));
        let atlas = AtlasAllocator::with_options(atlas_size, &guillotiere::AllocatorOptions {
            // TODO: Verify some good empirical constants.
            small_size_threshold: 128,
            large_size_threshold: 1024,
            ..guillotiere::AllocatorOptions::default()
        });
        let texture = renderer.create_texture_raw(
            gfx::texture::Kind::D2(
                max_texture_size,
                max_texture_size,
                gfx::texture::AaMode::Single,
            ),
            1 as gfx::texture::Level,
            gfx::memory::Bind::SHADER_RESOURCE,
            gfx::memory::Usage::Dynamic,
            (0, 0),
            gfx::format::Swizzle::new(),
            gfx::texture::SamplerInfo::new(
                gfx::texture::FilterMethod::Bilinear,
                gfx::texture::WrapMode::Clamp,
            ),
        )?;
        Ok((atlas, texture))
    }

    fn remove_chunk_meta(&mut self, _pos: Vec2<i32>, chunk: &TerrainChunkData) {
        self.atlas.deallocate(chunk.col_lights);
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

    /// Maintain terrain data. To be called once per tick.
    #[allow(clippy::for_loops_over_fallibles)] // TODO: Pending review in #587
    #[allow(clippy::len_zero)] // TODO: Pending review in #587
    pub fn maintain(
        &mut self,
        renderer: &mut Renderer,
        scene_data: &SceneData,
        focus_pos: Vec3<f32>,
        loaded_distance: f32,
        view_mat: Mat4<f32>,
        proj_mat: Mat4<f32>,
    ) -> (Aabb<f32>, Vec<math::Vec3<f32>>, math::Aabr<f32>) {
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

                    if !self.chunks.contains_key(&pos) || modified {
                        let mut neighbours = true;
                        for i in -1..2 {
                            for j in -1..2 {
                                neighbours &= scene_data
                                    .state
                                    .terrain()
                                    .get_key(pos + Vec2::new(i, j))
                                    .is_some();
                            }
                        }

                        if neighbours {
                            self.mesh_todo.insert(pos, ChunkMeshState {
                                pos,
                                started_tick: current_tick,
                                is_worker_active: false,
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
        for pos in scene_data
            .state
            .terrain_changes()
            .modified_blocks
            .iter()
            .map(|(p, _)| *p)
        {
            // Handle block changes on chunk borders
            // Remesh all neighbours because we have complex lighting now
            // TODO: if lighting is on the server this can be updated to only remesh when
            // lighting changes in that neighbouring chunk or if the block
            // change was on the border
            for x in -1..2 {
                for y in -1..2 {
                    let neighbour_pos = pos + Vec3::new(x, y, 0);
                    let neighbour_chunk_pos = scene_data.state.terrain().pos_key(neighbour_pos);

                    // Only remesh if this chunk has all its neighbors
                    let mut neighbours = true;
                    for i in -1..2 {
                        for j in -1..2 {
                            neighbours &= scene_data
                                .state
                                .terrain()
                                .get_key(neighbour_chunk_pos + Vec2::new(i, j))
                                .is_some();
                        }
                    }
                    if neighbours {
                        self.mesh_todo.insert(neighbour_chunk_pos, ChunkMeshState {
                            pos: neighbour_chunk_pos,
                            started_tick: current_tick,
                            is_worker_active: false,
                        });
                    }
                }
            }
        }
        drop(guard);

        // Limit ourselves to u16::MAX even if larger textures are supported.
        let max_texture_size = renderer.max_texture_size();

        span!(guard, "Queue meshing from todo list");
        for (todo, chunk) in self
            .mesh_todo
            .values_mut()
            .filter(|todo| !todo.is_worker_active)
            .min_by_key(|todo| todo.started_tick)
            // Find a reference to the actual `TerrainChunk` we're meshing
            .and_then(|todo| {
                let pos = todo.pos;
                Some((todo, scene_data.state
                    .terrain()
                    .get_key_arc(pos)
                    .cloned()?))
            })
        {
            // TODO: find a alternative!
            if scene_data.thread_pool.queued_jobs() > 0 {
                break;
            }

            // Find the area of the terrain we want. Because meshing needs to compute things
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

            // Queue the worker thread.
            let started_tick = todo.started_tick;
            let sprite_data = Arc::clone(&self.sprite_data);
            let sprite_config = Arc::clone(&self.sprite_config);
            scene_data.thread_pool.execute(move || {
                let sprite_data = sprite_data;
                let _ = send.send(mesh_worker(
                    pos,
                    (min_z as f32, max_z as f32),
                    started_tick,
                    volume,
                    max_texture_size,
                    chunk,
                    aabb,
                    &sprite_data,
                    &sprite_config,
                ));
            });
            todo.is_worker_active = true;
        }
        drop(guard);

        // Receive a chunk mesh from a worker thread and upload it to the GPU, then
        // store it. Only pull out one chunk per frame to avoid an unacceptable
        // amount of blocking lag due to the GPU upload. That still gives us a
        // 60 chunks / second budget to play with.
        span!(guard, "Get/upload meshed chunk");
        if let Ok(response) = self.mesh_recv.recv_timeout(Duration::new(0, 0)) {
            match self.mesh_todo.get(&response.pos) {
                // It's the mesh we want, insert the newly finished model into the terrain model
                // data structure (convert the mesh to a model first of course).
                Some(todo) if response.started_tick <= todo.started_tick => {
                    let started_tick = todo.started_tick;
                    let load_time = self
                        .chunks
                        .get(&response.pos)
                        .map(|chunk| chunk.load_time)
                        .unwrap_or(current_time as f32);
                    // TODO: Allocate new atlas on allocation failure.
                    let (tex, tex_size) = response.col_lights_info;
                    let atlas = &mut self.atlas;
                    let allocation = atlas
                        .allocate(guillotiere::Size::new(
                            i32::from(tex_size.x),
                            i32::from(tex_size.y),
                        ))
                        .expect("Not yet implemented: allocate new atlas on allocation failure.");
                    // NOTE: Cast is safe since the origin was a u16.
                    let atlas_offs = Vec2::new(
                        allocation.rectangle.min.x as u16,
                        allocation.rectangle.min.y as u16,
                    );
                    if let Err(err) = renderer.update_texture(
                        &self.col_lights,
                        atlas_offs.into_array(),
                        tex_size.into_array(),
                        &tex,
                    ) {
                        warn!("Failed to update texture: {:?}", err);
                    }

                    self.insert_chunk(response.pos, TerrainChunkData {
                        load_time,
                        opaque_model: renderer
                            .create_model(&response.opaque_mesh)
                            .expect("Failed to upload chunk mesh to the GPU!"),
                        fluid_model: if response.fluid_mesh.vertices().len() > 0 {
                            Some(
                                renderer
                                    .create_model(&response.fluid_mesh)
                                    .expect("Failed to upload chunk mesh to the GPU!"),
                            )
                        } else {
                            None
                        },
                        col_lights: allocation.id,
                        sprite_instances: response
                            .sprite_instances
                            .into_iter()
                            .map(|(kind, instances)| {
                                (
                                    kind,
                                    renderer.create_instances(&instances).expect(
                                        "Failed to upload chunk sprite instances to the GPU!",
                                    ),
                                )
                            })
                            .collect(),
                        locals: renderer
                            .create_consts(&[TerrainLocals {
                                model_offs: Vec3::from(
                                    response.pos.map2(VolGrid2d::<V>::chunk_size(), |e, sz| {
                                        e as f32 * sz as f32
                                    }),
                                )
                                .into_array(),
                                atlas_offs: Vec4::new(
                                    i32::from(atlas_offs.x),
                                    i32::from(atlas_offs.y),
                                    0,
                                    0,
                                )
                                .into_array(),
                                load_time,
                            }])
                            .expect("Failed to upload chunk locals to the GPU!"),
                        visible: Visibility::OutOfRange,
                        can_shadow_point: false,
                        can_shadow_sun: false,
                        blocks_of_interest: response.blocks_of_interest,
                        z_bounds: response.z_bounds,
                        frustum_last_plane_index: 0,
                    });

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
            (proj_mat * view_mat * Mat4::translation_3d(-focus_off)).into_col_arrays(),
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
            let in_range = distance_2 < loaded_distance.powf(2.0);

            if !in_range {
                chunk.visible = Visibility::OutOfRange;
                continue;
            }

            // Ensure the chunk is within the view frustum
            let chunk_min = [chunk_pos.x, chunk_pos.y, chunk.z_bounds.0];
            let chunk_max = [
                chunk_pos.x + chunk_sz,
                chunk_pos.y + chunk_sz,
                chunk.z_bounds.1,
            ];

            let (in_frustum, last_plane_index) = AABB::new(chunk_min, chunk_max)
                .coherent_test_against_frustum(&frustum, chunk.frustum_last_plane_index);

            chunk.frustum_last_plane_index = last_plane_index;
            chunk.visible = if in_frustum {
                Visibility::Visible
            } else {
                Visibility::InRange
            };
            let chunk_box = Aabb {
                min: Vec3::from(chunk_min),
                max: Vec3::from(chunk_max),
            };

            if in_frustum {
                let visible_box = chunk_box;
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

        // PSCs: Potential shadow casters
        let ray_direction = scene_data.get_sun_dir();
        let collides_with_aabr = |a: math::Aabb<f32>, b: math::Aabr<f32>| {
            let min = math::Vec4::new(a.min.x, a.min.y, b.min.x, b.min.y);
            let max = math::Vec4::new(b.max.x, b.max.y, a.max.x, a.max.y);
            min.partial_cmple_simd(max).reduce_and()
        };
        let (visible_light_volume, visible_psr_bounds) = if ray_direction.z < 0.0
            && renderer.render_mode().shadow.is_map()
        {
            let visible_bounding_box = math::Aabb::<f32> {
                min: math::Vec3::from(visible_bounding_box.min - focus_off),
                max: math::Vec3::from(visible_bounding_box.max - focus_off),
            };
            let focus_off = math::Vec3::from(focus_off);
            let visible_bounds_fine = visible_bounding_box.as_::<f64>();
            let inv_proj_view =
                math::Mat4::from_col_arrays((proj_mat * view_mat).into_col_arrays())
                    .as_::<f64>()
                    .inverted();
            let ray_direction = math::Vec3::<f32>::from(ray_direction);
            let visible_light_volume = math::calc_focused_light_volume_points(
                inv_proj_view,
                ray_direction.as_::<f64>(),
                visible_bounds_fine,
                1e-6,
            )
            .map(|v| v.as_::<f32>())
            .collect::<Vec<_>>();

            let cam_pos = math::Vec4::from(view_mat.inverted() * Vec4::unit_w()).xyz();
            let up: math::Vec3<f32> = { math::Vec3::up() };

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
                .filter(|chunk| chunk.1.visible <= Visibility::InRange)
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

        (
            visible_bounding_box,
            visible_light_volume,
            visible_psr_bounds,
        )
    }

    pub fn get(&self, chunk_key: Vec2<i32>) -> Option<&TerrainChunkData> {
        self.chunks.get(&chunk_key)
    }

    pub fn chunk_count(&self) -> usize { self.chunks.len() }

    pub fn visible_chunk_count(&self) -> usize {
        self.chunks
            .iter()
            .filter(|(_, c)| c.visible == Visibility::Visible)
            .count()
    }

    pub fn shadow_chunk_count(&self) -> usize { self.shadow_chunks.len() }

    pub fn render_shadows(
        &self,
        renderer: &mut Renderer,
        global: &GlobalModel,
        (is_daylight, light_data): super::LightData,
        focus_pos: Vec3<f32>,
    ) {
        span!(_guard, "render_shadows", "Terrain::render_shadows");
        if !renderer.render_mode().shadow.is_map() {
            return;
        };

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
        if is_daylight {
            chunk_iter
                .clone()
                .filter(|chunk| chunk.can_shadow_sun())
                .chain(self.shadow_chunks.iter().map(|(_, chunk)| chunk))
                .for_each(|chunk| {
                    // Directed light shadows.
                    renderer.render_terrain_shadow_directed(
                        &chunk.opaque_model,
                        global,
                        &chunk.locals,
                        &global.shadow_mats,
                    );
                });
        }

        // Point shadows
        //
        // NOTE: We don't bother retaining chunks unless they cast sun shadows, so we
        // don't use `shadow_chunks` here.
        light_data.iter().take(1).for_each(|_light| {
            chunk_iter.clone().for_each(|chunk| {
                if chunk.can_shadow_point {
                    renderer.render_shadow_point(
                        &chunk.opaque_model,
                        global,
                        &chunk.locals,
                        &global.shadow_mats,
                    );
                }
            });
        });
    }

    pub fn render(
        &self,
        renderer: &mut Renderer,
        global: &GlobalModel,
        lod: &LodData,
        focus_pos: Vec3<f32>,
    ) {
        span!(_guard, "render", "Terrain::render");
        let focus_chunk = Vec2::from(focus_pos).map2(TerrainChunk::RECT_SIZE, |e: f32, sz| {
            (e as i32).div_euclid(sz as i32)
        });

        let chunk_iter = Spiral2d::new()
            .filter_map(|rpos| {
                let pos = focus_chunk + rpos;
                self.chunks.get(&pos).map(|c| (pos, c))
            })
            .take(self.chunks.len());

        for (_, chunk) in chunk_iter {
            if chunk.visible == Visibility::Visible {
                renderer.render_terrain_chunk(
                    &chunk.opaque_model,
                    &self.col_lights,
                    global,
                    &chunk.locals,
                    lod,
                );
            }
        }
    }

    pub fn render_translucent(
        &self,
        renderer: &mut Renderer,
        global: &GlobalModel,
        lod: &LodData,
        focus_pos: Vec3<f32>,
        cam_pos: Vec3<f32>,
        sprite_render_distance: f32,
    ) {
        span!(_guard, "render_translucent", "Terrain::render_translucent");
        let focus_chunk = Vec2::from(focus_pos).map2(TerrainChunk::RECT_SIZE, |e: f32, sz| {
            (e as i32).div_euclid(sz as i32)
        });

        // Avoid switching textures
        let chunk_iter = Spiral2d::new()
            .filter_map(|rpos| {
                let pos = focus_chunk + rpos;
                self.chunks.get(&pos).map(|c| (pos, c))
            })
            .take(self.chunks.len());

        // Terrain sprites
        // TODO: move to separate functions
        span!(guard, "Terrain sprites");
        let chunk_size = V::RECT_SIZE.map(|e| e as f32);
        let chunk_mag = (chunk_size * (f32::consts::SQRT_2 * 0.5)).magnitude_squared();
        for (pos, chunk) in chunk_iter.clone() {
            if chunk.visible == Visibility::Visible {
                let sprite_low_detail_distance = sprite_render_distance * 0.75;
                let sprite_mid_detail_distance = sprite_render_distance * 0.5;
                let sprite_hid_detail_distance = sprite_render_distance * 0.35;
                let sprite_high_detail_distance = sprite_render_distance * 0.15;

                let chunk_center = pos.map2(chunk_size, |e, sz| (e as f32 + 0.5) * sz);
                let focus_dist_sqrd = Vec2::from(focus_pos).distance_squared(chunk_center);
                let dist_sqrd =
                    Vec2::from(cam_pos)
                        .distance_squared(chunk_center)
                        .min(Vec2::from(cam_pos).distance_squared(chunk_center - chunk_size * 0.5))
                        .min(Vec2::from(cam_pos).distance_squared(
                            chunk_center - chunk_size.x * 0.5 + chunk_size.y * 0.5,
                        ))
                        .min(
                            Vec2::from(cam_pos).distance_squared(chunk_center + chunk_size.x * 0.5),
                        )
                        .min(Vec2::from(cam_pos).distance_squared(
                            chunk_center + chunk_size.x * 0.5 - chunk_size.y * 0.5,
                        ));
                if focus_dist_sqrd < sprite_render_distance.powf(2.0) {
                    for (kind, instances) in (&chunk.sprite_instances).into_iter() {
                        let SpriteData { model, locals, .. } = if kind
                            .0
                            .elim_case_pure(&self.sprite_config)
                            .as_ref()
                            .map(|config| config.wind_sway >= 0.4)
                            .unwrap_or(false)
                            && dist_sqrd <= chunk_mag
                            || dist_sqrd < sprite_high_detail_distance.powf(2.0)
                        {
                            &self.sprite_data[&kind][0]
                        } else if dist_sqrd < sprite_hid_detail_distance.powf(2.0) {
                            &self.sprite_data[&kind][1]
                        } else if dist_sqrd < sprite_mid_detail_distance.powf(2.0) {
                            &self.sprite_data[&kind][2]
                        } else if dist_sqrd < sprite_low_detail_distance.powf(2.0) {
                            &self.sprite_data[&kind][3]
                        } else {
                            &self.sprite_data[&kind][4]
                        };
                        renderer.render_sprites(
                            model,
                            &self.sprite_col_lights,
                            global,
                            &chunk.locals,
                            locals,
                            &instances,
                            lod,
                        );
                    }
                }
            }
        }
        drop(guard);

        // Translucent
        chunk_iter
            .clone()
            .filter(|(_, chunk)| chunk.visible == Visibility::Visible)
            .filter_map(|(_, chunk)| {
                chunk
                    .fluid_model
                    .as_ref()
                    .map(|model| (model, &chunk.locals))
            })
            .collect::<Vec<_>>()
            .into_iter()
            .rev() // Render back-to-front
            .for_each(|(model, locals)| {
                renderer.render_fluid_chunk(
                    model,
                    global,
                    locals,
                    lod,
                    &self.waves,
                )
            });
    }
}
