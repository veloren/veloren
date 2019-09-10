use crate::{
    mesh::{vol, Meshable},
    render::{
        self, Consts, FluidPipeline, Instances, Mesh, Model, Renderer, SpriteInstance,
        TerrainLocals, TerrainPipeline,
    },
};
use common::{
    terrain::{Block, BlockKind},
    vol::{BaseVol, ReadVol, RectRasterableVol, SampleVol, SizedVol, Vox, WriteVol},
    volumes::vol_grid_2d::{VolGrid2d, VolGrid2dChange, VolGrid2dError, VolGrid2dJournal},
};
use crossbeam::channel;
use frustum_query::frustum::Frustum;
use hashbrown::HashMap;
use std::{cell::RefCell, f32, fmt::Debug, marker::PhantomData, sync::Arc, time::Duration};
use uvth::ThreadPool;
use vek::*;

type TerrainVertex = <TerrainPipeline as render::Pipeline>::Vertex;
type FluidVertex = <FluidPipeline as render::Pipeline>::Vertex;

fn block_shadow_density(kind: BlockKind) -> (f32, f32) {
    // (density, cap)
    match kind {
        BlockKind::Normal => (0.085, 0.3),
        BlockKind::Dense => (0.3, 0.0),
        BlockKind::Water => (0.15, 0.0),
        kind if kind.is_air() => (0.0, 0.0),
        _ => (1.0, 0.0),
    }
}

impl<V: RectRasterableVol<Vox = Block> + ReadVol + Debug> Meshable<TerrainPipeline, FluidPipeline>
    for VolGrid2d<V>
{
    type Pipeline = TerrainPipeline;
    type TranslucentPipeline = FluidPipeline;
    type Supplement = Aabb<i32>;

    fn generate_mesh(
        &self,
        range: Self::Supplement,
    ) -> (Mesh<Self::Pipeline>, Mesh<Self::TranslucentPipeline>) {
        let mut opaque_mesh = Mesh::new();
        let mut fluid_mesh = Mesh::new();

        for x in range.min.x + 1..range.max.x - 1 {
            for y in range.min.y + 1..range.max.y - 1 {
                let mut neighbour_light = [[[1.0f32; 3]; 3]; 3];

                for z in (range.min.z..range.max.z).rev() {
                    let pos = Vec3::new(x, y, z);
                    let offs = (pos - (range.min + 1) * Vec3::new(1, 1, 0)).map(|e| e as f32);

                    let block = self.get(pos).ok();

                    // Create mesh polygons
                    if let Some(col) = block
                        .filter(|vox| vox.is_opaque())
                        .and_then(|vox| vox.get_color())
                    {
                        let col = col.map(|e| e as f32 / 255.0);

                        vol::push_vox_verts(
                            &mut opaque_mesh,
                            self,
                            pos,
                            offs,
                            col,
                            |pos, norm, col, ao, light| {
                                TerrainVertex::new(pos, norm, col, light * ao)
                            },
                            false,
                            &neighbour_light,
                            |vox| !vox.is_opaque(),
                            |vox| vox.is_opaque(),
                        );
                    } else if let Some(col) = block
                        .filter(|vox| vox.is_fluid())
                        .and_then(|vox| vox.get_color())
                    {
                        let col = col.map(|e| e as f32 / 255.0);

                        vol::push_vox_verts(
                            &mut fluid_mesh,
                            self,
                            pos,
                            offs,
                            col,
                            |pos, norm, col, ao, light| {
                                FluidVertex::new(pos, norm, col, light * ao, 0.3)
                            },
                            false,
                            &neighbour_light,
                            |vox| vox.is_air(),
                            |vox| vox.is_opaque(),
                        );
                    }

                    // Shift lighting
                    neighbour_light[2] = neighbour_light[1];
                    neighbour_light[1] = neighbour_light[0];

                    // Accumulate shade under opaque blocks
                    for i in 0..3 {
                        for j in 0..3 {
                            let (density, cap) = self
                                .get(pos + Vec3::new(i as i32 - 1, j as i32 - 1, -1))
                                .ok()
                                .map(|vox| block_shadow_density(vox.kind()))
                                .unwrap_or((0.0, 0.0));

                            neighbour_light[0][i][j] = (neighbour_light[0][i][j] * (1.0 - density))
                                .max(cap.min(neighbour_light[1][i][j]));
                        }
                    }

                    // Spread light
                    neighbour_light[0] = [[neighbour_light[0]
                        .iter()
                        .map(|col| col.iter())
                        .flatten()
                        .copied()
                        .fold(0.0, |a, x| a + x)
                        / 9.0; 3]; 3];
                }
            }
        }

        (opaque_mesh, fluid_mesh)
    }
}

struct SpriteConfig {
    variations: usize,
    wind_sway: f32, // 1.0 is normal
}

fn sprite_config_for(kind: BlockKind) -> Option<SpriteConfig> {
    match kind {
        BlockKind::LargeCactus => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::BarrelCactus => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::RoundCactus => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::ShortCactus => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::MedFlatCactus => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::ShortFlatCactus => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),

        BlockKind::BlueFlower => Some(SpriteConfig {
            variations: 5,
            wind_sway: 0.1,
        }),
        BlockKind::PinkFlower => Some(SpriteConfig {
            variations: 4,
            wind_sway: 0.1,
        }),
        BlockKind::RedFlower => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.1,
        }),
        BlockKind::WhiteFlower => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.1,
        }),
        BlockKind::YellowFlower => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.1,
        }),
        BlockKind::Sunflower => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.1,
        }),

        BlockKind::LongGrass => Some(SpriteConfig {
            variations: 7,
            wind_sway: 0.8,
        }),
        BlockKind::MediumGrass => Some(SpriteConfig {
            variations: 5,
            wind_sway: 0.5,
        }),
        BlockKind::ShortGrass => Some(SpriteConfig {
            variations: 5,
            wind_sway: 0.1,
        }),

        BlockKind::Apple => Some(SpriteConfig {
            variations: 1,
            wind_sway: 0.0,
        }),
        BlockKind::Mushroom => Some(SpriteConfig {
            variations: 10,
            wind_sway: 0.0,
        }),
        BlockKind::Liana => Some(SpriteConfig {
            variations: 2,
            wind_sway: 0.05,
        }),
        _ => None,
    }
}

/// A type produced by mesh worker threads corresponding to the position and mesh of a chunk.
struct MeshWorkerResponse {
    key: Vec2<i32>,
    z_bounds: (f32, f32),
    opaque_mesh: Mesh<TerrainPipeline>,
    fluid_mesh: Mesh<FluidPipeline>,
    sprite_instances: HashMap<(BlockKind, usize), Vec<SpriteInstance>>,
    started_tick: u64,
}

/// Function executed by worker threads dedicated to chunk meshing.
fn mesh_worker<V: BaseVol<Vox = Block> + RectRasterableVol + ReadVol + Debug>(
    key: Vec2<i32>,
    z_bounds: (f32, f32),
    started_tick: u64,
    volume: <VolGrid2d<V> as SampleVol<Aabr<i32>>>::Sample,
    range: Aabb<i32>,
) -> MeshWorkerResponse {
    let (opaque_mesh, fluid_mesh) = volume.generate_mesh(range);
    MeshWorkerResponse {
        key,
        z_bounds,
        opaque_mesh,
        fluid_mesh,
        // Extract sprite locations from volume
        sprite_instances: {
            let mut instances = HashMap::new();

            for x in 0..V::RECT_SIZE.x as i32 {
                for y in 0..V::RECT_SIZE.y as i32 {
                    for z in z_bounds.0 as i32..z_bounds.1 as i32 + 1 {
                        let wpos = Vec3::from(key * V::RECT_SIZE.map(|e: u32| e as i32))
                            + Vec3::new(x, y, z);

                        let kind = volume.get(wpos).unwrap_or(&Block::empty()).kind();

                        if let Some(cfg) = sprite_config_for(kind) {
                            let seed = wpos.x * 3 + wpos.y * 7 + wpos.x * wpos.y; // Awful PRNG

                            let instance = SpriteInstance::new(
                                Mat4::identity()
                                    .rotated_z(f32::consts::PI * 0.5 * (seed % 4) as f32)
                                    .translated_3d(
                                        wpos.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0),
                                    ),
                                Rgb::broadcast(1.0),
                                cfg.wind_sway,
                            );

                            instances
                                .entry((kind, seed as usize % cfg.variations))
                                .or_insert_with(|| Vec::new())
                                .push(instance);
                        }
                    }
                }
            }

            instances
        },
        started_tick,
    }
}

pub struct ChunkModel {
    // GPU data
    pub opaque_model: Model<TerrainPipeline>,
    pub fluid_model: Model<FluidPipeline>,
    pub sprite_instances: HashMap<(BlockKind, usize), Instances<SpriteInstance>>,
    pub locals: Consts<TerrainLocals>,
    pub lower_bound: Vec3<f32>,
    pub upper_bound: Vec3<f32>,
}

impl ChunkModel {
    pub fn in_frustum(&self, view_mat: &Mat4<f32>, proj_mat: &Mat4<f32>) -> bool {
        let frustum = Frustum::from_modelview_and_projection(
            &view_mat.into_col_array(),
            &proj_mat.into_col_array(),
        );
        let center = 0.5 * (self.upper_bound + self.lower_bound);
        let radius = (0.5 * (self.upper_bound - self.lower_bound)).magnitude();
        frustum.sphere_intersecting(&center.x, &center.y, &center.z, &radius)
    }
}

struct ChunkModelData {
    model: Option<Arc<ChunkModel>>,
    needs_worker: bool,
    version_from_tick: u64, // Tick from which the underlying terrain data was taken.
    _last_access_tick: u64, // Will later be used to decide which structs to delete.
}

pub struct ChunkModelCache<
    V: BaseVol<Vox = Block>
        + RectRasterableVol
        + SizedVol
        + ReadVol
        + WriteVol
        + Clone
        + Debug
        + Send
        + Sync
        + 'static,
> {
    chunk_models: RefCell<HashMap<Vec2<i32>, ChunkModelData>>,
    internal_tick: u64,
    // The mpsc sender and receiver used for talking to meshing worker threads.
    // We keep the sender component for no reason other than to clone it and send it to new workers.
    mesh_worker_tx: channel::Sender<MeshWorkerResponse>,
    mesh_worker_rx: channel::Receiver<MeshWorkerResponse>,
    phantom: PhantomData<V>,
}

impl<
        V: BaseVol<Vox = Block>
            + RectRasterableVol
            + SizedVol
            + ReadVol
            + WriteVol
            + Clone
            + Debug
            + Send
            + Sync
            + 'static,
    > ChunkModelCache<V>
{
    pub fn new() -> Self {
        let (mesh_worker_tx, mesh_worker_rx) = channel::unbounded();

        Self {
            chunk_models: RefCell::new(HashMap::new()),
            internal_tick: 0,
            mesh_worker_tx,
            mesh_worker_rx,
            phantom: PhantomData,
        }
    }

    pub fn maintain(
        &mut self,
        thread_pool: &ThreadPool,
        renderer: &mut Renderer,
        grid_journal: &VolGrid2dJournal<V>,
    ) {
        self.internal_tick += 1;

        // Add any recently created or changed chunks to the list of chunks to be meshed.
        for (key, c) in grid_journal.previous_changes() {
            if let VolGrid2dChange::<V>::Insert(_) = c {
                // If the block on the edge of a chunk gets modified, then we need to spawn a mesh
                // worker to remesh its neighbour(s) too since their ambient occlusion and face
                // elision information changes, too! Therefore we iterate [-1..1]x[-1..1].
                for key_offs_y in -1..=1 {
                    for key_offs_x in -1..=1 {
                        let key = key + Vec2::new(key_offs_x, key_offs_y);

                        if grid_journal.grid().get_key(key).is_some() {
                            self.chunk_models
                                .borrow_mut()
                                .get_mut(&key)
                                .map(|chunk_model| {
                                    chunk_model.needs_worker = true;
                                });
                        }
                    }
                }
            } else {
                self.chunk_models
                    .borrow_mut()
                    .get_mut(key)
                    .map(|chunk_model| {
                        chunk_model.needs_worker = true;
                    });
            }
        }

        'outer: for (&key, chunk_model) in self.chunk_models.borrow_mut().iter_mut() {
            if chunk_model.needs_worker {
                // Only mesh chunks whose neighbours are available.
                for key_offs_y in -1..=1 {
                    for key_offs_x in -1..=1 {
                        if grid_journal
                            .grid()
                            .get_key(key + Vec2::new(key_offs_x, key_offs_y))
                            .is_none()
                        {
                            continue 'outer;
                        }
                    }
                }

                // BEGIN enqueue job.

                // Find the area of the terrain we want. Because meshing needs to compute things like
                // ambient occlusion and edge elision, we also need the borders of the chunk's
                // neighbours too (hence the `- 1` and `+ 1`).
                let aabr = Aabr {
                    min: key.map2(VolGrid2d::<V>::chunk_size(), |e, sz| e * sz as i32 - 1),
                    max: key.map2(VolGrid2d::<V>::chunk_size(), |e, sz| {
                        (e + 1) * sz as i32 + 1
                    }),
                };

                // Copy out the chunk data we need to perform the meshing. We do this by taking a
                // sample of the terrain that includes both the chunk we want and its neighbours.
                let volume = match grid_journal.grid().sample(aabr) {
                    Ok(sample) => sample,
                    // Either this chunk or its neighbours doesn't yet exist, so we keep it in the
                    // queue to be processed at a later date when we have its neighbours.
                    Err(VolGrid2dError::NoSuchChunk) => return,
                    _ => panic!("Unhandled edge case"),
                };

                // The region to actually mesh
                let min_z = volume.iter().fold(std::i32::MAX, |min, (_, chunk)| {
                    chunk.lower_bound().z.min(min)
                });
                let max_z = volume.iter().fold(std::i32::MIN, |max, (_, chunk)| {
                    chunk.upper_bound().z.max(max)
                });

                let aabb = Aabb {
                    min: Vec3::from(aabr.min) + Vec3::unit_z() * (min_z - 1),
                    max: Vec3::from(aabr.max) + Vec3::unit_z() * (max_z + 1),
                };

                // Clone various things so that they can be moved into the thread.
                let mesh_worker_tx = self.mesh_worker_tx.clone();

                // Queue the worker thread.
                let started_tick = self.internal_tick;
                thread_pool.execute(move || {
                    let _ = mesh_worker_tx.send(mesh_worker(
                        key,
                        (min_z as f32, max_z as f32),
                        started_tick,
                        volume,
                        aabb,
                    ));
                });

                // END enqueue job.

                chunk_model.needs_worker = false;
            }
        }

        // Receive a chunk mesh from a worker thread and upload it to the GPU, then store it.
        // Only pull out one chunk per frame to avoid an unacceptable amount of blocking lag due
        // to the GPU upload. That still gives us a 60 chunks / second budget to play with.
        if let Ok(response) = self.mesh_worker_rx.recv_timeout(Duration::new(0, 0)) {
            match self.chunk_models.borrow_mut().get_mut(&response.key) {
                // It's the mesh we want, insert the newly finished model into the terrain model
                // data structure (convert the mesh to a model first of course).
                Some(ref mut chunk_model)
                    if response.started_tick > chunk_model.version_from_tick =>
                {
                    chunk_model.model = Some(Arc::new(ChunkModel {
                        opaque_model: renderer
                            .create_model(&response.opaque_mesh)
                            .expect("Failed to upload chunk mesh to the GPU!"),
                        fluid_model: renderer
                            .create_model(&response.fluid_mesh)
                            .expect("Failed to upload chunk mesh to the GPU!"),
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
                                    response.key.map2(VolGrid2d::<V>::chunk_size(), |e, sz| {
                                        e as f32 * sz as f32
                                    }),
                                )
                                .into_array(),
                            }])
                            .expect("Failed to upload chunk locals to the GPU!"),
                        lower_bound: Vec3::new(
                            (response.key.x * V::RECT_SIZE.x as i32) as f32,
                            ((response.key.y + 1) * V::RECT_SIZE.y as i32) as f32,
                            response.z_bounds.0,
                        ),
                        upper_bound: Vec3::new(
                            (response.key.x * V::RECT_SIZE.x as i32) as f32,
                            ((response.key.y + 1) * V::RECT_SIZE.y as i32) as f32,
                            response.z_bounds.1,
                        ),
                    }));
                    chunk_model.version_from_tick = response.started_tick;
                }
                // Chunk must have been removed, or it was spawned on an old tick. Drop the mesh
                // since it's either out of date or no longer needed.
                _ => {}
            }
        }
    }

    pub fn request_model(&self, key: Vec2<i32>) -> Option<Arc<ChunkModel>> {
        self.chunk_models
            .borrow_mut()
            .entry(key)
            .or_insert(ChunkModelData {
                model: None,
                needs_worker: true,
                version_from_tick: 0,
                _last_access_tick: self.internal_tick,
            })
            .model
            .clone() // Clones the `Arc` if contained in the `Option`.
    }
}
