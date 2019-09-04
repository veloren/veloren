// Reexports
pub use sphynx::Uid;

use crate::{
    comp,
    event::{EventBus, LocalEvent, ServerEvent},
    msg::{EcsCompPacket, EcsResPacket},
    sys,
    terrain::{Block, TerrainChange, TerrainChunk, TerrainJournal},
};
use rayon::{ThreadPool, ThreadPoolBuilder};
use serde_derive::{Deserialize, Serialize};
use specs::{
    shred::Fetch,
    storage::{MaskedStorage as EcsMaskedStorage, Storage as EcsStorage},
    Component, DispatcherBuilder, Entity as EcsEntity,
};
use sphynx;
use std::{sync::Arc, time::Duration};
use vek::*;

/// How much faster should an in-game day be compared to a real day?
// TODO: Don't hard-code this.
const DAY_CYCLE_FACTOR: f64 = 24.0 * 2.0;

/// A resource that stores the time of day.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeOfDay(pub f64);

/// A resource that stores the tick (i.e: physics) time.
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Time(pub f64);

/// A resource that stores the time since the previous tick.
#[derive(Default)]
pub struct DeltaTime(pub f32);

/// At what point should we stop speeding up physics to compensate for lag? If we speed physics up
/// too fast, we'd skip important physics events like collisions. This constant determines the
/// upper limit. If delta time exceeds this value, the game's physics will begin to produce time
/// lag. Ideally, we'd avoid such a situation.
const MAX_DELTA_TIME: f32 = 1.0;
const HUMANOID_JUMP_ACCEL: f32 = 18.0;

/// A type used to represent game state stored on both the client and the server. This includes
/// things like entity components, terrain data, and global states like weather, time of day, etc.
pub struct State {
    ecs: sphynx::World<EcsCompPacket, EcsResPacket>,
    // Avoid lifetime annotation by storing a thread pool instead of the whole dispatcher
    thread_pool: Arc<ThreadPool>,
}

impl Default for State {
    /// Create a new `State`.
    fn default() -> Self {
        Self {
            ecs: sphynx::World::new(specs::World::new(), Self::setup_sphynx_world),
            thread_pool: Arc::new(ThreadPoolBuilder::new().build().unwrap()),
        }
    }
}

impl State {
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
        }
    }

    // Create a new Sphynx ECS world.
    // TODO: Split up registering into server and client (e.g. move EventBus<ServerEvent> to the server)
    fn setup_sphynx_world(ecs: &mut sphynx::World<EcsCompPacket, EcsResPacket>) {
        // Register server -> all clients synced components.
        ecs.register_synced::<comp::Body>();
        ecs.register_synced::<comp::Player>();
        ecs.register_synced::<comp::Stats>();
        ecs.register_synced::<comp::CanBuild>();
        ecs.register_synced::<comp::LightEmitter>();
        ecs.register_synced::<comp::Item>();
        ecs.register_synced::<comp::Scale>();

        // Register components send from clients -> server
        ecs.register::<comp::Controller>();

        // Register components send directly from server -> all but one client
        ecs.register::<comp::CharacterState>();
        ecs.register::<comp::PhysicsState>();

        // Register components synced from client -> server -> all other clients
        ecs.register::<comp::Pos>();
        ecs.register::<comp::Vel>();
        ecs.register::<comp::Ori>();
        ecs.register::<comp::Inventory>();

        // Register server-local components
        ecs.register::<comp::Last<comp::Pos>>();
        ecs.register::<comp::Last<comp::Vel>>();
        ecs.register::<comp::Last<comp::Ori>>();
        ecs.register::<comp::Last<comp::CharacterState>>();
        ecs.register::<comp::Agent>();
        ecs.register::<comp::ForceUpdate>();
        ecs.register::<comp::InventoryUpdate>();
        ecs.register::<comp::Inventory>();
        ecs.register::<comp::Admin>();

        // Register synced resources used by the ECS.
        ecs.add_resource_synced(TimeOfDay(0.0));

        // Register unsynced resources used by the ECS.
        ecs.add_resource(Time(0.0));
        ecs.add_resource(DeltaTime(0.0));
        ecs.add_resource(TerrainJournal::new());
        ecs.add_resource(EventBus::<ServerEvent>::default());
        ecs.add_resource(EventBus::<LocalEvent>::default());
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

    /// Get a reference to the `TerrainJournal` structure of the state. This contains
    /// information about terrain state and its changes in the last game tick.
    pub fn terrain_journal(&self) -> Fetch<TerrainJournal> {
        self.ecs.read_resource::<TerrainJournal>()
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

    /// Get the current delta time.
    pub fn get_delta_time(&self) -> f32 {
        self.ecs.read_resource::<DeltaTime>().0
    }

    /// Get a writable reference to this state's terrain.
    pub fn set_block(&mut self, pos: Vec3<i32>, block: Block) {
        self.ecs
            .write_resource::<TerrainJournal>()
            .request_vox_change(pos, block);
    }

    /// Removes every chunk of the terrain.
    pub fn clear_terrain(&mut self) {
        self.ecs
            .write_resource::<TerrainJournal>()
            .request_clearance();
    }

    /// Insert the provided chunk into this state's terrain.
    pub fn insert_chunk(&mut self, key: Vec2<i32>, chunk: Arc<TerrainChunk>) {
        self.ecs
            .write_resource::<TerrainJournal>()
            .request_change(key, TerrainChange::Insert(chunk));
    }

    /// Remove the chunk with the given key from this state's terrain, if it exists.
    pub fn remove_chunk(&mut self, key: Vec2<i32>) {
        // TODO (haslersn): Should we consider it an error if the chunk is does not exist?
        self.ecs
            .write_resource::<TerrainJournal>()
            .request_change(key, TerrainChange::Remove);
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

        // Apply terrain changes; this must be called **exactly** once per tick!
        self.ecs.write_resource::<TerrainJournal>().apply();

        // Process local events
        let events = self.ecs.read_resource::<EventBus<LocalEvent>>().recv_all();
        for event in events {
            let mut velocities = self.ecs.write_storage::<comp::Vel>();
            match event {
                LocalEvent::LandOnGround { entity, vel } => {
                    if let Some(stats) = self.ecs.write_storage::<comp::Stats>().get_mut(entity) {
                        let falldmg = (vel.z / 1.5 + 10.0) as i32;
                        if falldmg < 0 {
                            stats.health.change_by(falldmg, comp::HealthSource::World);
                        }
                    }
                }

                LocalEvent::Jump(entity) => {
                    if let Some(vel) = velocities.get_mut(entity) {
                        vel.0.z = HUMANOID_JUMP_ACCEL;
                    }
                }

                LocalEvent::Boost {
                    entity,
                    vel: extra_vel,
                } => {
                    if let Some(vel) = velocities.get_mut(entity) {
                        vel.0 += extra_vel;
                    }
                }
            }
        }
    }

    /// Clean up the state after a tick.
    pub fn cleanup(&mut self) {
        // With the curren't implementation (especially w.r.t. the `TerrainJournal`) there's
        // nothing to do here.
    }
}
