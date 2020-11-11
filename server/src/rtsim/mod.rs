mod load_chunks;
mod unload_chunks;
mod tick;

use vek::*;
use world::util::Grid;
use common::{
    state::State,
    terrain::TerrainChunk,
    rtsim::{RtSimEntity, RtSimId},
    vol::RectRasterableVol,
};
use specs::{DispatcherBuilder, WorldExt};
use specs_idvs::IdvStorage;
use slab::Slab;
use rand::prelude::*;

pub struct RtSim {
    world: RtWorld,
    entities: Slab<Entity>,
}

impl RtSim {
    pub fn new(world_chunk_size: Vec2<u32>) -> Self {
        Self {
            world: RtWorld {
                chunks: Grid::populate_from(world_chunk_size.map(|e| e as i32), |_| Chunk {
                    is_loaded: false,
                }),
                chunks_to_load: Vec::new(),
                chunks_to_unload: Vec::new(),
            },
            entities: Slab::new(),
        }
    }

    pub fn hook_load_chunk(&mut self, key: Vec2<i32>) {
        if let Some(chunk) = self.world.chunks.get_mut(key) {
            if !chunk.is_loaded {
                chunk.is_loaded = true;
                self.world.chunks_to_load.push(key);
            }
        }
    }

    pub fn hook_unload_chunk(&mut self, key: Vec2<i32>) {
        if let Some(chunk) = self.world.chunks.get_mut(key) {
            if chunk.is_loaded {
                chunk.is_loaded = false;
                self.world.chunks_to_unload.push(key);
            }
        }
    }

    pub fn assimilate_entity(&mut self, entity: RtSimId) {
        tracing::info!("Assimilated rtsim entity {}", entity);
        self.entities.get_mut(entity).map(|e| e.is_loaded = false);
    }

    pub fn reify_entity(&mut self, entity: RtSimId) {
        tracing::info!("Reified rtsim entity {}", entity);
        self.entities.get_mut(entity).map(|e| e.is_loaded = true);
    }

    pub fn update_entity(&mut self, entity: RtSimId, pos: Vec3<f32>) {
        self.entities.get_mut(entity).map(|e| e.pos = pos);
    }

    pub fn destroy_entity(&mut self, entity: RtSimId) {
        tracing::info!("Destroyed rtsim entity {}", entity);
        self.entities.remove(entity);
    }
}

pub struct RtWorld {
    chunks: Grid<Chunk>,
    chunks_to_load: Vec<Vec2<i32>>,
    chunks_to_unload: Vec<Vec2<i32>>,
}

impl RtWorld {
    pub fn chunk_at(&self, pos: Vec2<f32>) -> Option<&Chunk> {
        self.chunks.get(pos.map2(TerrainChunk::RECT_SIZE, |e, sz| (e.floor() as i32).div_euclid(sz as i32)))
    }
}

pub struct Chunk {
    is_loaded: bool,
}

pub struct Entity {
    is_loaded: bool,
    pos: Vec3<f32>,
    seed: u32,
}

const LOAD_CHUNK_SYS: &str = "rtsim_load_chunk_sys";
const UNLOAD_CHUNK_SYS: &str = "rtsim_unload_chunk_sys";
const TICK_SYS: &str = "rtsim_tick_sys";

pub fn add_server_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch_builder.add(unload_chunks::Sys, UNLOAD_CHUNK_SYS, &[]);
    dispatch_builder.add(load_chunks::Sys, LOAD_CHUNK_SYS, &[UNLOAD_CHUNK_SYS]);
    dispatch_builder.add(tick::Sys, TICK_SYS, &[LOAD_CHUNK_SYS, UNLOAD_CHUNK_SYS]);
}

pub fn init(state: &mut State, world: &world::World) {
    let mut rtsim = RtSim::new(world.sim().get_size());

    for _ in 0..10 {
        let pos = Vec2::new(
            thread_rng().gen_range(0, rtsim.world.chunks.size().x * TerrainChunk::RECT_SIZE.x as i32),
            thread_rng().gen_range(0, rtsim.world.chunks.size().y * TerrainChunk::RECT_SIZE.y as i32),
        );

        let id = rtsim.entities.insert(Entity {
            is_loaded: false,
            pos: Vec3::from(pos.map(|e| e as f32)),
            seed: thread_rng().gen(),
        });

        tracing::info!("Spawned rtsim NPC {} at {:?}", id, pos);
    }

    state.ecs_mut().insert(rtsim);
    state.ecs_mut().register::<RtSimEntity>();
    tracing::info!("Initiated real-time world simulation");
}
