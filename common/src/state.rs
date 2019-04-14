// Reexports
pub use sphynx::Uid;

use std::{
    time::Duration,
    collections::HashSet,
};
use shred::{Fetch, FetchMut};
use specs::{
    Builder,
    Component,
    DispatcherBuilder,
    EntityBuilder as EcsEntityBuilder,
    Entity as EcsEntity,
    storage::{
        Storage as EcsStorage,
        MaskedStorage as EcsMaskedStorage,
    },
    saveload::{MarkedBuilder, MarkerAllocator},
};
use sphynx;
use vek::*;
use crate::{
    comp,
    sys,
    terrain::{
        TerrainMap,
        TerrainChunk,
    },
    msg::EcsPacket,
};

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
    pub new_chunks: HashSet<Vec3<i32>>,
    pub changed_chunks: HashSet<Vec3<i32>>,
    pub removed_chunks: HashSet<Vec3<i32>>,
}

impl Changes {
    pub fn default() -> Self {
        Self {
            new_chunks: HashSet::new(),
            changed_chunks: HashSet::new(),
            removed_chunks: HashSet::new(),
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
    ecs: sphynx::World<EcsPacket>,
    changes: Changes,
}

impl State {
    /// Create a new `State`.
    pub fn new() -> Self {
        Self {
            ecs: sphynx::World::new(specs::World::new(), Self::setup_sphynx_world),
            changes: Changes::default(),
        }
    }

    /// Create a new `State` from an ECS state package
    pub fn from_state_package(state_package: sphynx::StatePackage<EcsPacket>) -> Self {
        Self {
            ecs: sphynx::World::from_state_package(specs::World::new(), Self::setup_sphynx_world, state_package),
            changes: Changes::default(),
        }
    }

    // Create a new Sphynx ECS world
    fn setup_sphynx_world(ecs: &mut sphynx::World<EcsPacket>) {
        // Register synced components
        ecs.register_synced::<comp::Character>();
        ecs.register_synced::<comp::Player>();

        // Register unsynched (or synced by other means) components
        ecs.internal_mut().register::<comp::phys::Pos>();
        ecs.internal_mut().register::<comp::phys::Vel>();
        ecs.internal_mut().register::<comp::phys::Dir>();

        // Register resources used by the ECS
        ecs.internal_mut().add_resource(TimeOfDay(0.0));
        ecs.internal_mut().add_resource(Time(0.0));
        ecs.internal_mut().add_resource(DeltaTime(0.0));
        ecs.internal_mut().add_resource(TerrainMap::new());
    }

    /// Register a component with the state's ECS
    pub fn with_component<T: Component>(mut self) -> Self
        where <T as Component>::Storage: Default
    {
        self.ecs.internal_mut().register::<T>();
        self
    }

    /// Write a component attributed to a particular entity
    pub fn write_component<C: Component>(&mut self, entity: EcsEntity, comp: C) {
        let _ = self.ecs.internal_mut().write_storage().insert(entity, comp);
    }

    /// Read a component attributed to a particular entity
    pub fn read_component_cloned<C: Component + Clone>(&self, entity: EcsEntity) -> Option<C> {
        self.ecs.internal().read_storage().get(entity).cloned()
    }

    /// Get a read-only reference to the storage of a particular component type
    pub fn read_storage<C: Component>(&self) -> EcsStorage<C, Fetch<EcsMaskedStorage<C>>> {
        self.ecs.internal().read_storage::<C>()
    }

    /// Get a reference to the internal ECS world
    pub fn ecs(&self) -> &sphynx::World<EcsPacket> {
        &self.ecs
    }

    /// Get a mutable reference to the internal ECS world
    pub fn ecs_mut(&mut self) -> &mut sphynx::World<EcsPacket> {
        &mut self.ecs
    }

    /// Get a reference to the `Changes` structure of the state. This contains
    /// information about state that has changed since the last game tick.
    pub fn changes(&self) -> &Changes {
        &self.changes
    }

    /// Get the current in-game time of day.
    ///
    /// Note that this should not be used for physics, animations or other such localised timings.
    pub fn get_time_of_day(&self) -> f64 {
        self.ecs.internal().read_resource::<TimeOfDay>().0
    }

    /// Get the current in-game time.
    ///
    /// Note that this does not correspond to the time of day.
    pub fn get_time(&self) -> f64 {
        self.ecs.internal().read_resource::<Time>().0
    }

    /// Get a reference to this state's terrain.
    pub fn terrain(&self) -> Fetch<TerrainMap> {
        self.ecs
            .internal()
            .read_resource::<TerrainMap>()
    }

    /// Insert the provided chunk into this state's terrain.
    pub fn insert_chunk(&mut self, key: Vec3<i32>, chunk: TerrainChunk) {
        if self.ecs
            .internal_mut()
            .write_resource::<TerrainMap>()
            .insert(key, chunk)
            .is_some()
        {
            self.changes.changed_chunks.insert(key);
        } else {
            self.changes.new_chunks.insert(key);
        }
    }

    /// Execute a single tick, simulating the game state by the given duration.
    pub fn tick(&mut self, dt: Duration) {
        // Change the time accordingly
        self.ecs.internal_mut().write_resource::<TimeOfDay>().0 += dt.as_secs_f64() * DAY_CYCLE_FACTOR;
        self.ecs.internal_mut().write_resource::<Time>().0 += dt.as_secs_f64();

        // Run systems to update the world
        self.ecs.internal_mut().write_resource::<DeltaTime>().0 = dt.as_secs_f64();

        // Create and run dispatcher for ecs systems
        let mut dispatch_builder = DispatcherBuilder::new();
        sys::add_local_systems(&mut dispatch_builder);
        // This dispatches all the systems in parallel
        dispatch_builder.build().dispatch(&self.ecs.internal_mut().res);

        self.ecs.internal_mut().maintain();
    }

    /// Clean up the state after a tick
    pub fn cleanup(&mut self) {
        // Clean up data structures from the last tick
        self.changes.cleanup();
    }
}
