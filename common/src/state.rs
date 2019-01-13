// Standard
use std::time::Duration;

// External
use specs::World as EcsWorld;

// Crate
use crate::{
    comp,
    terrain::TerrainMap,
};

/// How much faster should an in-game day be compared to a real day?
// TODO: Don't hard-code this
const DAY_CYCLE_FACTOR: f64 = 24.0 * 60.0;

/// A resource to store the time of day
struct TimeOfDay(f64);

/// A resource to store the tick (i.e: physics) time
struct Tick(f64);

/// A type used to represent game state stored on both the client and the server. This includes
/// things like entity components, terrain data, and global state like weather, time of day, etc.
pub struct State {
    ecs_world: EcsWorld,
    terrain_map: TerrainMap,
    time: f64,
}

impl State {
    /// Create a new `State`.
    pub fn new() -> Self {
        let mut ecs_world = EcsWorld::new();

        // Register resources used by the ECS
        ecs_world.add_resource(TimeOfDay(0.0));
        ecs_world.add_resource(Tick(0.0));

        // Register common components with the state
        comp::register_local_components(&mut ecs_world);

        Self {
            ecs_world,
            terrain_map: TerrainMap::new(),
            time: 0.0,
        }
    }

    /// Get the current in-game time of day.
    ///
    /// Note that this should not be used for physics, animations or other such localised timings.
    pub fn get_time_of_day(&self) -> f64 {
        self.ecs_world.read_resource::<TimeOfDay>().0
    }

    /// Get the current in-game tick time.
    ///
    /// Note that this does not correspond to the time of day.
    pub fn get_tick(&self) -> f64 {
        self.ecs_world.read_resource::<Tick>().0
    }

    /// Execute a single tick, simulating the game state by the given duration.
    pub fn tick(&mut self, dt: Duration) {
        // Change the time accordingly
        self.ecs_world.write_resource::<TimeOfDay>().0 += dt.as_float_secs() * DAY_CYCLE_FACTOR;
        self.ecs_world.write_resource::<Tick>().0 += dt.as_float_secs();
    }
}
