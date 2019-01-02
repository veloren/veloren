// Standard
use std::time::Duration;

// External
use specs::World as EcsWorld;

// Crate
use crate::{
    comp,
    terrain::TerrainMap,
    vol::VolSize,
};

// A type used to represent game state stored on both the client and the server. This includes
// things like entity components, terrain data, and global state like weather, time of day, etc.
pub struct State {
    ecs_world: EcsWorld,
    terrain_map: TerrainMap,
    time: f64,
}

impl State {
    pub fn new() -> Self {
        let mut ecs_world = EcsWorld::new();

        comp::register_local_components(&mut ecs_world);

        Self {
            ecs_world,
            terrain_map: TerrainMap::new(),
            time: 0.0,
        }
    }

    // Execute a single tick, simulating the game state by the given duration
    pub fn tick(&mut self, dt: Duration) {
        println!("Ticked!");
    }
}
