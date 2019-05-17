use crate::{
    mesh::Meshable,
    render::{Consts, Globals, Mesh, Model, Renderer, TerrainLocals, TerrainPipeline},
};
use client::Client;
use common::{terrain::TerrainMap, vol::SampleVol, volumes::vol_map_2d::VolMap2dErr};
use std::{collections::HashMap, sync::mpsc, time::Duration};
use vek::*;

struct TerrainChunk {
    // GPU data
    model: Model<TerrainPipeline>,
    locals: Consts<TerrainLocals>,
}

struct ChunkMeshState {
    pos: Vec2<i32>,
    started_tick: u64,
    active_worker: bool,
}

/// A type produced by mesh worker threads corresponding to the position and mesh of a chunk
struct MeshWorkerResponse {
    pos: Vec2<i32>,
    mesh: Mesh<TerrainPipeline>,
    started_tick: u64,
}

/// Function executed by worker threads dedicated to chunk meshing
fn mesh_worker(
    pos: Vec2<i32>,
    started_tick: u64,
    volume: <TerrainMap as SampleVol<Aabr<i32>>>::Sample,
    range: Aabb<i32>,
) -> MeshWorkerResponse {
    MeshWorkerResponse {
        pos,
        mesh: volume.generate_mesh(range),
        started_tick,
    }
}

pub struct Terrain {
    chunks: HashMap<Vec2<i32>, TerrainChunk>,

    // The mpsc sender and receiver used for talking to meshing worker threads.
    // We keep the sender component for no reason othe than to clone it and send it to new workers.
    mesh_send_tmp: mpsc::Sender<MeshWorkerResponse>,
    mesh_recv: mpsc::Receiver<MeshWorkerResponse>,
    mesh_todo: HashMap<Vec2<i32>, ChunkMeshState>,
}

impl Terrain {
    pub fn new() -> Self {
        // Create a new mpsc (Multiple Produced, Single Consumer) pair for communicating with
        // worker threads that are meshing chunks.
        let (send, recv) = mpsc::channel();

        Self {
            chunks: HashMap::new(),

            mesh_send_tmp: send,
            mesh_recv: recv,
            mesh_todo: HashMap::new(),
        }
    }

    /// Maintain terrain data. To be called once per tick.
    pub fn maintain(&mut self, renderer: &mut Renderer, client: &Client) {
        let current_tick = client.get_tick();

        // Add any recently created or changed chunks to the list of chunks to be meshed
        for pos in client
            .state()
            .changes()
            .new_chunks
            .iter()
            .chain(client.state().changes().changed_chunks.iter())
        {
            // TODO: ANOTHER PROBLEM HERE!
            // What happens if the block on the edge of a chunk gets modified? We need to spawn
            // a mesh worker to remesh its neighbour(s) too since their ambient occlusion and face
            // elision information changes too!
            for i in -1..2 {
                for j in -1..2 {
                    let pos = pos + Vec2::new(i, j);

                    if !self.chunks.contains_key(&pos) {
                        let mut neighbours = true;
                        for i in -1..2 {
                            for j in -1..2 {
                                neighbours &= client
                                    .state()
                                    .terrain()
                                    .get_key(pos + Vec2::new(i, j))
                                    .is_some();
                            }
                        }

                        if neighbours {
                            self.mesh_todo.entry(pos).or_insert(ChunkMeshState {
                                pos,
                                started_tick: current_tick,
                                active_worker: false,
                            });
                        }
                    }
                }
            }
        }
        // Remove any models for chunks that have been recently removed
        for pos in &client.state().changes().removed_chunks {
            self.chunks.remove(pos);
            self.mesh_todo.remove(pos);
        }

        for todo in self
            .mesh_todo
            .values_mut()
            // Only spawn workers for meshing jobs without an active worker already
            .filter(|todo| !todo.active_worker)
        {
            // Find the area of the terrain we want. Because meshing needs to compute things like
            // ambient occlusion and edge elision, we also need to borders of the chunk's
            // neighbours too (hence the `- 1` and `+ 1`).
            let aabr = Aabr {
                min: todo
                    .pos
                    .map2(TerrainMap::chunk_size(), |e, sz| e * sz as i32 - 1),
                max: todo
                    .pos
                    .map2(TerrainMap::chunk_size(), |e, sz| (e + 1) * sz as i32 + 1),
            };

            let aabb = Aabb {
                min: Vec3::from(aabr.min),
                max: Vec3::from(aabr.max) + Vec3::unit_z() * 256,
            };

            // Copy out the chunk data we need to perform the meshing. We do this by taking a
            // sample of the terrain that includes both the chunk we want and
            let volume = match client.state().terrain().sample(aabr) {
                Ok(sample) => sample,
                // If either this chunk or its neighbours doesn't yet exist, so we keep it in the
                // todo queue to be processed at a later date when we have its neighbours.
                Err(VolMap2dErr::NoSuchChunk) => return,
                _ => panic!("Unhandled edge case"),
            };

            // Clone various things to that they can be moved into the thread
            let send = self.mesh_send_tmp.clone();
            let pos = todo.pos;

            // Queue the worker thread
            client.thread_pool().execute(move || {
                let _ = send.send(mesh_worker(pos, current_tick, volume, aabb));
            });
            todo.active_worker = true;
        }

        // Receive a chunk mesh from a worker thread, upload it to the GPU and then store it
        // Only pull out one chunk per frame to avoid an unacceptable amount of blocking lag due
        // to the GPU upload. That still gives us a 60 chunks / second budget to play with.
        if let Ok(response) = self.mesh_recv.recv_timeout(Duration::new(0, 0)) {
            match self.mesh_todo.get(&response.pos) {
                // It's the mesh we want, insert the newly finished model into the terrain model
                // data structure (convert the mesh to a model first of course)
                Some(todo) if response.started_tick == todo.started_tick => {
                    self.chunks.insert(
                        response.pos,
                        TerrainChunk {
                            model: renderer
                                .create_model(&response.mesh)
                                .expect("Failed to upload chunk mesh to the GPU"),
                            locals: renderer
                                .create_consts(&[TerrainLocals {
                                    model_offs: Vec3::from(
                                        response.pos.map2(TerrainMap::chunk_size(), |e, sz| {
                                            e as f32 * sz as f32
                                        }),
                                    )
                                    .into_array(),
                                }])
                                .expect("Failed to upload chunk locals to the GPU"),
                        },
                    );
                }
                // Chunk must have been removed, or it was spawned on an old tick. Drop the mesh
                // since it's either out of date or no longer needed
                _ => {}
            }
        }
    }

    pub fn render(&self, renderer: &mut Renderer, globals: &Consts<Globals>) {
        for (_, chunk) in &self.chunks {
            renderer.render_terrain_chunk(&chunk.model, globals, &chunk.locals);
        }
    }
}
