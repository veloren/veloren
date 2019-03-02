// Standard
use std::time::Duration;

// Library
use shred::{Fetch, FetchMut};
use specs::{Builder, Component, DispatcherBuilder, Entity as EcsEntity, World as EcsWorld};
use vek::*;

// Crate
use crate::{comp, sys, terrain::TerrainMap};

/// How much faster should an in-game day be compared to a real day?
// TODO: Don't hard-code this
const DAY_CYCLE_FACTOR: f64 = 24.0 * 60.0;

/// A resource to store the time of day
struct TimeOfDay(f64);

/// A resource to store the tick (i.e: physics) time
struct Time(f64);

/// A resource used to store the time since the last tick
#[derive(Default)]
pub struct DeltaTime(pub f64);

pub struct Changes {
    pub new_chunks: Vec<Vec3<i32>>,
    pub changed_chunks: Vec<Vec3<i32>>,
    pub removed_chunks: Vec<Vec3<i32>>,
}

impl Changes {
    pub fn default() -> Self {
        Self {
            new_chunks: vec![],
            changed_chunks: vec![],
            removed_chunks: vec![],
        }
    }

    pub fn cleanup(&mut self) {
        self.new_chunks.clear();
        self.changed_chunks.clear();
        self.removed_chunks.clear();
    }
}

/// A type used to represent game state stored on both the client and the server. This includes
/// things like entity components, terrain data, and global state like weather, time of day, etc.
pub struct State {
    ecs_world: EcsWorld,
    changes: Changes,
}

impl State {
    /// Create a new `State`.
    pub fn new() -> Self {
        let mut ecs_world = EcsWorld::new();

        // Register resources used by the ECS
        ecs_world.add_resource(TimeOfDay(0.0));
        ecs_world.add_resource(Time(0.0));
        ecs_world.add_resource(DeltaTime(0.0));
        ecs_world.add_resource(TerrainMap::new());

        // Register common components with the state
        comp::register_local_components(&mut ecs_world);

        Self {
            ecs_world,
            changes: Changes::default(),
        }
    }

    // TODO: Get rid of this
    pub fn new_test_player(&mut self) -> EcsEntity {
        self.ecs_world
            .create_entity()
            .with(comp::phys::Pos(Vec3::default()))
            .with(comp::phys::Vel(Vec3::default()))
            .with(comp::phys::Dir(Vec3::default()))
            .build()
    }

    /// Write a component
    pub fn write_component<C: Component>(&mut self, e: EcsEntity, c: C) {
        let _ = self.ecs_world.write_storage().insert(e, c);
    }

    /// Get a reference to the internal ECS world
    pub fn ecs_world(&self) -> &EcsWorld {
        &self.ecs_world
    }

    /// Get a reference to the `Changes` structure of the state. This contains
    /// information about state that has changed since the last game tick.
    pub fn changes(&self) -> &Changes {
        &self.changes
    }

    // TODO: Get rid of this since it shouldn't be needed
    pub fn changes_mut(&mut self) -> &mut Changes {
        &mut self.changes
    }

    /// Get the current in-game time of day.
    ///
    /// Note that this should not be used for physics, animations or other such localised timings.
    pub fn get_time_of_day(&self) -> f64 {
        self.ecs_world.read_resource::<TimeOfDay>().0
    }

    /// Get the current in-game time.
    ///
    /// Note that this does not correspond to the time of day.
    pub fn get_time(&self) -> f64 {
        self.ecs_world.read_resource::<Time>().0
    }

    /// Get a reference to this state's terrain.
    pub fn terrain(&self) -> Fetch<TerrainMap> {
        self.ecs_world.read_resource::<TerrainMap>()
    }

    // TODO: Get rid of this since it shouldn't be needed
    pub fn terrain_mut(&mut self) -> FetchMut<TerrainMap> {
        self.ecs_world.write_resource::<TerrainMap>()
    }

    /// Execute a single tick, simulating the game state by the given duration.
    pub fn tick(&mut self, dt: Duration) {
        // Change the time accordingly
        self.ecs_world.write_resource::<TimeOfDay>().0 += dt.as_float_secs() * DAY_CYCLE_FACTOR;
        self.ecs_world.write_resource::<Time>().0 += dt.as_float_secs();

        // Run systems to update the world
        self.ecs_world.write_resource::<DeltaTime>().0 = dt.as_float_secs();

        // Create and run dispatcher for ecs systems
        let mut dispatch_builder = DispatcherBuilder::new();
        sys::add_local_systems(&mut dispatch_builder);
        // This dispatches all the systems in parallel
        dispatch_builder.build().dispatch(&self.ecs_world.res);

        self.ecs_world.maintain();
    }

    /// Clean up the state after a tick
    pub fn cleanup(&mut self) {
        // Clean up data structures from the last tick
        self.changes.cleanup();
    }
}
