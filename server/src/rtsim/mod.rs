mod load_chunks;
mod unload_chunks;

use vek::*;
use world::util::Grid;
use common::state::State;
use specs::{DispatcherBuilder, Component, WorldExt};
use specs_idvs::IdvStorage;

type EntityId = u64;

pub struct RtSim {
    chunks: Grid<Chunk>,
    chunks_to_load: Vec<Vec2<i32>>,
    chunks_to_unload: Vec<Vec2<i32>>,
}

impl RtSim {
    pub fn new(world_chunk_size: Vec2<u32>) -> Self {
        Self {
            chunks: Grid::populate_from(world_chunk_size.map(|e| e as i32), |_| Chunk {
                is_loaded: false,
            }),
            chunks_to_load: Vec::new(),
            chunks_to_unload: Vec::new(),
        }
    }

    pub fn hook_load_chunk(&mut self, key: Vec2<i32>) {
        if let Some(chunk) = self.chunks.get_mut(key) {
            if !chunk.is_loaded {
                chunk.is_loaded = true;
                self.chunks_to_load.push(key);
            }
        }
    }

    pub fn hook_unload_chunk(&mut self, key: Vec2<i32>) {
        if let Some(chunk) = self.chunks.get_mut(key) {
            if chunk.is_loaded {
                chunk.is_loaded = false;
                self.chunks_to_unload.push(key);
            }
        }
    }

    pub fn assimilate_entity(&mut self, entity: EntityId) {
        // TODO
    }

    pub fn update_entity(&mut self, entity: EntityId, pos: Vec3<f32>) {
        // TODO
    }
}

pub struct Chunk {
    is_loaded: bool,
}

pub struct RtSimEntity(EntityId);

impl Component for RtSimEntity {
    type Storage = IdvStorage<Self>;
}

const LOAD_CHUNK_SYS: &str = "rtsim_load_chunk_sys";
const UNLOAD_CHUNK_SYS: &str = "rtsim_unload_chunk_sys";

pub fn add_server_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch_builder.add(load_chunks::Sys, LOAD_CHUNK_SYS, &[]);
    dispatch_builder.add(unload_chunks::Sys, UNLOAD_CHUNK_SYS, &[]);
}

pub fn init(state: &mut State, world: &world::World) {
    state.ecs_mut().insert(RtSim::new(world.sim().get_size()));
    state.ecs_mut().register::<RtSimEntity>();
    tracing::info!("Initiated real-time world simulation");
}
