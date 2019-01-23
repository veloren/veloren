// Standard
use std::{
    collections::{HashMap, LinkedList},
    sync::mpsc,
    time::Duration,
};

// Library
use vek::*;

// Project
use client::Client;
use common::{
    terrain::TerrainMap,
    volumes::vol_map::VolMapErr,
    vol::SampleVol,
};

// Crate
use crate::{
    render::{
        Consts,
        Globals,
        Mesh,
        Model,
        Renderer,
        TerrainPipeline,
        TerrainLocals,
    },
    mesh::Meshable,
};

struct TerrainChunk {
    // GPU data
    model: Model<TerrainPipeline>,
    locals: Consts<TerrainLocals>,
}

struct ChunkMeshState {
    pos: Vec3<i32>,
    started_tick: u64,
    active_worker: bool,
}

/// A type produced by mesh worker threads corresponding to the position and mesh of a chunk
struct MeshWorkerResponse {
    pos: Vec3<i32>,
    mesh: Mesh<TerrainPipeline>,
    started_tick: u64,
}

/// Function executed by worker threads dedicated to chunk meshing
fn mesh_worker(
    pos: Vec3<i32>,
    started_tick: u64,
    volume: <TerrainMap as SampleVol>::Sample,
) -> MeshWorkerResponse {
    MeshWorkerResponse {
        pos,
        mesh: volume.generate_mesh(()),
        started_tick,
    }
}

pub struct Terrain {
    chunks: HashMap<Vec3<i32>, TerrainChunk>,

    // The mpsc sender and receiver used for talking to meshing worker threads.
    // We keep the sender component for no reason othe than to clone it and send it to new workers.
    mesh_send_tmp: mpsc::Sender<MeshWorkerResponse>,
    mesh_recv: mpsc::Receiver<MeshWorkerResponse>,
    mesh_todo: LinkedList<ChunkMeshState>,
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
            mesh_todo: LinkedList::new(),
        }
    }

    /// Maintain terrain data. To be called once per tick.
    pub fn maintain(&mut self, renderer: &mut Renderer, client: &Client) {
        let current_tick = client.get_tick();

        // Add any recently created or changed chunks to the list of chunks to be meshed
        for pos in client.state().changes().new_chunks.iter()
            .chain(client.state().changes().changed_chunks.iter())
        {
            // TODO: ANOTHER PROBLEM HERE!
            // What happens if the block on the edge of a chunk gets modified? We need to spawn
            // a mesh worker to remesh its neighbour(s) too since their ambient occlusion and face
            // elision information changes too!
            match self.mesh_todo.iter_mut().find(|todo| todo.pos == *pos) {
                Some(todo) => todo.started_tick = current_tick,
                // The chunk it's queued yet, add it to the queue
                None => self.mesh_todo.push_back(ChunkMeshState {
                    pos: *pos,
                    started_tick: current_tick,
                    active_worker: false,
                }),
            }
        }

        // Remove any models for chunks that have been recently removed
        for pos in &client.state().changes().removed_chunks {
            self.chunks.remove(pos);
            self.mesh_todo.drain_filter(|todo| todo.pos == *pos);
        }

        // Clone the sender to the thread can send us the chunk data back
        // TODO: It's a bit hacky cloning it here and then cloning it again below. Fix this.
        let send = self.mesh_send_tmp.clone();

        self.mesh_todo
            .iter_mut()
            // Only spawn workers for meshing jobs without an active worker already
            .filter(|todo| !todo.active_worker)
            .for_each(|todo| {
                // Find the area of the terrain we want. Because meshing needs to compute things like
                // ambient occlusion and edge elision, we also need to borders of the chunk's
                // neighbours too (hence the `- 1` and `+ 1`).
                let aabb = Aabb {
                    min: todo.pos.map2(TerrainMap::chunk_size(), |e, sz| e * sz as i32 - 1),
                    max: todo.pos.map2(TerrainMap::chunk_size(), |e, sz| (e + 1) * sz as i32 + 1),
                };

                // Copy out the chunk data we need to perform the meshing. We do this by taking a
                // sample of the terrain that includes both the chunk we want and
                let volume = match client.state().terrain().sample(aabb) {
                    Ok(sample) => sample,
                    // If either this chunk or its neighbours doesn't yet exist, so we keep it in the
                    // todo queue to be processed at a later date when we have its neighbours.
                    Err(VolMapErr::NoSuchChunk) => return,
                    _ => panic!("Unhandled edge case"),
                };

                // Clone various things to that they can be moved into the thread
                let send = send.clone();
                let pos = todo.pos;

                // Queue the worker thread
                client.thread_pool().execute(move || {
                    send.send(mesh_worker(pos, current_tick, volume))
                        .expect("Failed to send chunk mesh to main thread");
                });
                todo.active_worker = true;
            });

        // Receive chunk meshes from worker threads, upload them to the GPU and then store them
        while let Ok(response) = self.mesh_recv.recv_timeout(Duration::new(0, 0)) {
            match self.mesh_todo.iter().find(|todo| todo.pos == response.pos) {
                // It's the mesh we want, insert the newly finished model into the terrain model
                // data structure (convert the mesh to a model first of course)
                Some(todo) if response.started_tick == todo.started_tick => {
                    self.chunks.insert(response.pos, TerrainChunk {
                        model: renderer.create_model(&response.mesh).expect("Failed to upload chunk mesh to the GPU"),
                        locals: renderer.create_consts(&[TerrainLocals {
                            model_offs: response.pos.map2(TerrainMap::chunk_size(), |e, sz| e as f32 * sz as f32).into_array(),
                        }]).expect("Failed to upload chunk locals to the GPU"),
                    });
                },
                // Chunk must have been removed, or it was spawned on an old tick. Drop the mesh
                // since it's either out of date or no longer needed
                _ => continue,
            }
        }
    }

    pub fn render(&self, renderer: &mut Renderer, globals: &Consts<Globals>) {
        for (_, chunk) in &self.chunks {
            renderer.render_terrain_chunk(
                &chunk.model,
                globals,
                &chunk.locals,
            );
        }
    }
}
