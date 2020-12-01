#![allow(dead_code)] // TODO: Remove this when rtsim is fleshed out

mod chunks;
mod entity;
mod load_chunks;
mod tick;
mod unload_chunks;

use self::{chunks::Chunks, entity::Entity};
use common::{
    comp,
    rtsim::{RtSimController, RtSimEntity, RtSimId},
    terrain::TerrainChunk,
    vol::RectRasterableVol,
};
use common_state::State;
use rand::prelude::*;
use slab::Slab;
use specs::{DispatcherBuilder, WorldExt};
use vek::*;

pub struct RtSim {
    tick: u64,
    chunks: Chunks,
    entities: Slab<Entity>,
}

impl RtSim {
    pub fn new(world_chunk_size: Vec2<u32>) -> Self {
        Self {
            tick: 0,
            chunks: Chunks::new(world_chunk_size),
            entities: Slab::new(),
        }
    }

    pub fn hook_load_chunk(&mut self, key: Vec2<i32>) {
        if let Some(chunk) = self.chunks.chunk_mut(key) {
            if !chunk.is_loaded {
                chunk.is_loaded = true;
                self.chunks.chunks_to_load.push(key);
            }
        }
    }

    pub fn hook_unload_chunk(&mut self, key: Vec2<i32>) {
        if let Some(chunk) = self.chunks.chunk_mut(key) {
            if chunk.is_loaded {
                chunk.is_loaded = false;
                self.chunks.chunks_to_unload.push(key);
            }
        }
    }

    pub fn assimilate_entity(&mut self, entity: RtSimId) {
        // tracing::info!("Assimilated rtsim entity {}", entity);
        self.entities.get_mut(entity).map(|e| e.is_loaded = false);
    }

    pub fn reify_entity(&mut self, entity: RtSimId) {
        // tracing::info!("Reified rtsim entity {}", entity);
        self.entities.get_mut(entity).map(|e| e.is_loaded = true);
    }

    pub fn update_entity(&mut self, entity: RtSimId, pos: Vec3<f32>) {
        self.entities.get_mut(entity).map(|e| e.pos = pos);
    }

    pub fn destroy_entity(&mut self, entity: RtSimId) {
        // tracing::info!("Destroyed rtsim entity {}", entity);
        self.entities.remove(entity);
    }
}

const LOAD_CHUNK_SYS: &str = "rtsim_load_chunk_sys";
const UNLOAD_CHUNK_SYS: &str = "rtsim_unload_chunk_sys";
const TICK_SYS: &str = "rtsim_tick_sys";

pub fn add_server_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch_builder.add(unload_chunks::Sys, UNLOAD_CHUNK_SYS, &[]);
    dispatch_builder.add(load_chunks::Sys, LOAD_CHUNK_SYS, &[UNLOAD_CHUNK_SYS]);
    dispatch_builder.add(tick::Sys, TICK_SYS, &[LOAD_CHUNK_SYS, UNLOAD_CHUNK_SYS]);
}

pub fn init(state: &mut State, #[cfg(feature = "worldgen")] world: &world::World) {
    #[cfg(feature = "worldgen")]
    let mut rtsim = RtSim::new(world.sim().get_size());
    #[cfg(not(feature = "worldgen"))]
    let mut rtsim = RtSim::new(Vec2::new(40, 40));

    for _ in 0..2500 {
        let pos = rtsim
            .chunks
            .size()
            .map2(TerrainChunk::RECT_SIZE, |sz, chunk_sz| {
                thread_rng().gen_range(0, sz * chunk_sz) as i32
            });

        rtsim.entities.insert(Entity {
            is_loaded: false,
            pos: Vec3::from(pos.map(|e| e as f32)),
            seed: thread_rng().gen(),
            controller: RtSimController::default(),
            last_tick: 0,
            brain: Default::default(),
        });
    }

    state.ecs_mut().insert(rtsim);
    state.ecs_mut().register::<RtSimEntity>();
    tracing::info!("Initiated real-time world simulation");
}
