// Reexports
pub use sphynx::Uid;

use crate::{
    comp, inventory,
    msg::{EcsCompPacket, EcsResPacket},
    sys,
    terrain::{TerrainChunk, TerrainMap},
};
use rayon::{ThreadPool, ThreadPoolBuilder};
use serde_derive::{Deserialize, Serialize};
use specs::{
    saveload::{MarkedBuilder, MarkerAllocator},
    shred::{Fetch, FetchMut},
    storage::{MaskedStorage as EcsMaskedStorage, Storage as EcsStorage},
    Builder, Component, DispatcherBuilder, Entity as EcsEntity, EntityBuilder as EcsEntityBuilder,
};
use sphynx;
use std::{collections::HashSet, sync::Arc, time::Duration};
use vek::*;

/// How much faster should an in-game day be compared to a real day?
// TODO: Don't hard-code this.
const DAY_CYCLE_FACTOR: f64 = 24.0 * 60.0;

/// A resource that stores the time of day.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeOfDay(f64);

/// A resource that stores the tick (i.e: physics) time.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Time(f64);

/// A resource that stores the time since the previous tick.
#[derive(Default)]
pub struct DeltaTime(pub f32);

/// At what point should we stop speeding up physics to compensate for lag? If we speed physics up
/// too fast, we'd skip important physics events like collisions. This constant determines the
/// upper limit. If delta time exceeds this value, the game's physics will begin to produce time
/// lag. Ideally, we'd avoid such a situation.
const MAX_DELTA_TIME: f32 = 0.15;

pub struct Changes {
    pub new_chunks: HashSet<Vec2<i32>>,
    pub changed_chunks: HashSet<Vec2<i32>>,
    pub removed_chunks: HashSet<Vec2<i32>>,
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
/// things like entity components, terrain data, and global states like weather, time of day, etc.
pub struct State {
    ecs: sphynx::World<EcsCompPacket, EcsResPacket>,
    // Avoid lifetime annotation by storing a thread pool instead of the whole dispatcher
    thread_pool: Arc<ThreadPool>,
    changes: Changes,
}

impl State {
    /// Create a new `State`.
    pub fn new() -> Self {
        Self {
            ecs: sphynx::World::new(specs::World::new(), Self::setup_sphynx_world),
            thread_pool: Arc::new(ThreadPoolBuilder::new().build().unwrap()),
            changes: Changes::default(),
        }
    }

    /// Create a new `State` from an ECS state package.
    pub fn from_state_package(
        state_package: sphynx::StatePackage<EcsCompPacket, EcsResPacket>,
    ) -> Self {
        Self {
            ecs: sphynx::World::from_state_package(
                specs::World::new(),
                Self::setup_sphynx_world,
                state_package,
            ),
            thread_pool: Arc::new(ThreadPoolBuilder::new().build().unwrap()),
            changes: Changes::default(),
        }
    }

    // Create a new Sphynx ECS world.
    fn setup_sphynx_world(ecs: &mut sphynx::World<EcsCompPacket, EcsResPacket>) {
        // Register synced components.
        ecs.register_synced::<comp::Actor>();
        ecs.register_synced::<comp::Player>();
        ecs.register_synced::<comp::Stats>();

        // Register unsynced (or synced by other means) components.
        ecs.register::<comp::phys::Pos>();
        ecs.register::<comp::phys::Vel>();
        ecs.register::<comp::phys::Dir>();
        ecs.register::<comp::ActionState>();
        ecs.register::<comp::Agent>();
        ecs.register::<comp::Control>();
        ecs.register::<inventory::Inventory>();

        // Register synced resources used by the ECS.
        ecs.add_resource_synced(TimeOfDay(0.0));

        // Register unsynced resources used by the ECS.
        ecs.add_resource(Time(0.0));
        ecs.add_resource(DeltaTime(0.0));
        ecs.add_resource(TerrainMap::new().unwrap());
    }

    /// Register a component with the state's ECS.
    pub fn with_component<T: Component>(mut self) -> Self
    where
        <T as Component>::Storage: Default,
    {
        self.ecs.register::<T>();
        self
    }

    /// Write a component attributed to a particular entity.
    pub fn write_component<C: Component>(&mut self, entity: EcsEntity, comp: C) {
        let _ = self.ecs.write_storage().insert(entity, comp);
    }

    /// Read a component attributed to a particular entity.
    pub fn read_component_cloned<C: Component + Clone>(&self, entity: EcsEntity) -> Option<C> {
        self.ecs.read_storage().get(entity).cloned()
    }

    /// Get a read-only reference to the storage of a particular component type.
    pub fn read_storage<C: Component>(&self) -> EcsStorage<C, Fetch<EcsMaskedStorage<C>>> {
        self.ecs.read_storage::<C>()
    }

    /// Get a reference to the internal ECS world.
    pub fn ecs(&self) -> &sphynx::World<EcsCompPacket, EcsResPacket> {
        &self.ecs
    }

    /// Get a mutable reference to the internal ECS world.
    pub fn ecs_mut(&mut self) -> &mut sphynx::World<EcsCompPacket, EcsResPacket> {
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
        self.ecs.read_resource::<TimeOfDay>().0
    }

    /// Get the current in-game time.
    ///
    /// Note that this does not correspond to the time of day.
    pub fn get_time(&self) -> f64 {
        self.ecs.read_resource::<Time>().0
    }

    /// Get a reference to this state's terrain.
    pub fn terrain(&self) -> Fetch<TerrainMap> {
        self.ecs.read_resource::<TerrainMap>()
    }

    /// Insert the provided chunk into this state's terrain.
    pub fn insert_chunk(&mut self, key: Vec2<i32>, chunk: TerrainChunk) {
        if self
            .ecs
            .write_resource::<TerrainMap>()
            .insert(key, Arc::new(chunk))
            .is_some()
        {
            self.changes.changed_chunks.insert(key);
        } else {
            self.changes.new_chunks.insert(key);
        }
    }

    /// Remove the chunk with the given key from this state's terrain, if it exists.
    pub fn remove_chunk(&mut self, key: Vec2<i32>) {
        if self
            .ecs
            .write_resource::<TerrainMap>()
            .remove(key)
            .is_some()
        {
            self.changes.removed_chunks.insert(key);
        }
    }

    /// Execute a single tick, simulating the game state by the given duration.
    pub fn tick(&mut self, dt: Duration) {
        // Change the time accordingly.
        self.ecs.write_resource::<TimeOfDay>().0 += dt.as_secs_f64() * DAY_CYCLE_FACTOR;
        self.ecs.write_resource::<Time>().0 += dt.as_secs_f64();

        // Update delta time.
        // Beyond a delta time of MAX_DELTA_TIME, start lagging to avoid skipping important physics events.
        self.ecs.write_resource::<DeltaTime>().0 = dt.as_secs_f32().min(MAX_DELTA_TIME);

        // Run systems to update the world.
        // Create and run a dispatcher for ecs systems.
        let mut dispatch_builder = DispatcherBuilder::new().with_pool(self.thread_pool.clone());
        sys::add_local_systems(&mut dispatch_builder);
        // This dispatches all the systems in parallel.
        dispatch_builder.build().dispatch(&self.ecs.res);

        self.ecs.maintain();
    }

    /// Clean up the state after a tick.
    pub fn cleanup(&mut self) {
        // Clean up data structures from the last tick.
        self.changes.cleanup();
    }
}
