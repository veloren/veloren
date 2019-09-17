// Reexports
pub use sphynx::Uid;

use crate::{
    comp,
    event::{EventBus, LocalEvent, ServerEvent},
    msg::{EcsCompPacket, EcsResPacket},
    sys,
    terrain::{Block, TerrainChunk, TerrainGrid},
    vol::WriteVol,
};
use hashbrown::{HashMap, HashSet};
use rayon::{ThreadPool, ThreadPoolBuilder};
use serde_derive::{Deserialize, Serialize};
use specs::{
    saveload::Marker,
    shred::{Fetch, FetchMut},
    storage::{MaskedStorage as EcsMaskedStorage, Storage as EcsStorage},
    Component, DispatcherBuilder, Entity as EcsEntity, Join,
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
const HUMANOID_JUMP_ACCEL: f32 = 16.0;

#[derive(Default)]
pub struct BlockChange {
    blocks: HashMap<Vec3<i32>, Block>,
}

impl BlockChange {
    pub fn set(&mut self, pos: Vec3<i32>, block: Block) {
        self.blocks.insert(pos, block);
    }

    pub fn try_set(&mut self, pos: Vec3<i32>, block: Block) -> Option<()> {
        if !self.blocks.contains_key(&pos) {
            self.blocks.insert(pos, block);
            Some(())
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.blocks.clear();
    }
}

#[derive(Default)]
pub struct TerrainChanges {
    pub new_chunks: HashSet<Vec2<i32>>,
    pub modified_chunks: HashSet<Vec2<i32>>,
    pub removed_chunks: HashSet<Vec2<i32>>,
    pub modified_blocks: HashMap<Vec3<i32>, Block>,
}

impl TerrainChanges {
    pub fn clear(&mut self) {
        self.new_chunks.clear();
        self.modified_chunks.clear();
        self.removed_chunks.clear();
    }
}

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
        ecs.register_synced::<comp::Mounting>();
        ecs.register_synced::<comp::MountState>();
        ecs.register_synced::<comp::Mass>();

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
        ecs.register::<comp::Waypoint>();
        ecs.register::<comp::Projectile>();

        // Register synced resources used by the ECS.
        ecs.insert_synced(TimeOfDay(0.0));

        // Register unsynced resources used by the ECS.
        ecs.add_resource(Time(0.0));
        ecs.add_resource(DeltaTime(0.0));
        ecs.add_resource(TerrainGrid::new().unwrap());
        ecs.add_resource(BlockChange::default());
        ecs.add_resource(TerrainChanges::default());
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

    /// Delete a component attributed to a particular entity.
    pub fn delete_component<C: Component>(&mut self, entity: EcsEntity) -> Option<C> {
        self.ecs.write_storage().remove(entity)
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

    /// Get a reference to the `TerrainChanges` structure of the state. This contains
    /// information about terrain state that has changed since the last game tick.
    pub fn terrain_changes(&self) -> Fetch<TerrainChanges> {
        self.ecs.read_resource()
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

    /// Get a reference to this state's terrain.
    pub fn terrain(&self) -> Fetch<TerrainGrid> {
        self.ecs.read_resource()
    }

    /// Get a writable reference to this state's terrain.
    pub fn terrain_mut(&self) -> FetchMut<TerrainGrid> {
        self.ecs.write_resource()
    }

    /// Set a block in this state's terrain.
    pub fn set_block(&mut self, pos: Vec3<i32>, block: Block) {
        self.ecs.write_resource::<BlockChange>().set(pos, block);
    }

    /// Set a block in this state's terrain. Will return `None` if the block has already been modified this tick.
    pub fn try_set_block(&mut self, pos: Vec3<i32>, block: Block) -> Option<()> {
        self.ecs.write_resource::<BlockChange>().try_set(pos, block)
    }

    /// Removes every chunk of the terrain.
    pub fn clear_terrain(&mut self) {
        let keys = self
            .terrain_mut()
            .drain()
            .map(|(key, _)| key)
            .collect::<Vec<_>>();

        for key in keys {
            self.remove_chunk(key);
        }
    }

    /// Insert the provided chunk into this state's terrain.
    pub fn insert_chunk(&mut self, key: Vec2<i32>, chunk: TerrainChunk) {
        if self
            .ecs
            .write_resource::<TerrainGrid>()
            .insert(key, Arc::new(chunk))
            .is_some()
        {
            self.ecs
                .write_resource::<TerrainChanges>()
                .modified_chunks
                .insert(key);
        } else {
            self.ecs
                .write_resource::<TerrainChanges>()
                .new_chunks
                .insert(key);
        }
    }

    /// Remove the chunk with the given key from this state's terrain, if it exists.
    pub fn remove_chunk(&mut self, key: Vec2<i32>) {
        if self
            .ecs
            .write_resource::<TerrainGrid>()
            .remove(key)
            .is_some()
        {
            self.ecs
                .write_resource::<TerrainChanges>()
                .removed_chunks
                .insert(key);
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

        // Mounted entities. We handle this here because we need access to the Uid registry and I
        // forgot how to access that within a system. Anyhow, here goes.
        for (entity, mount_state) in (
            &self.ecs.entities(),
            &mut self.ecs.write_storage::<comp::MountState>(),
        )
            .join()
        {
            match mount_state {
                comp::MountState::Unmounted => {}
                comp::MountState::MountedBy(mounter) => {
                    if let Some((controller, mounter)) =
                        self.ecs.entity_from_uid(mounter.id()).and_then(|mounter| {
                            self.ecs
                                .read_storage::<comp::Controller>()
                                .get(mounter)
                                .cloned()
                                .map(|x| (x, mounter))
                        })
                    {
                        let pos = self.ecs.read_storage::<comp::Pos>().get(entity).copied();
                        let ori = self.ecs.read_storage::<comp::Ori>().get(entity).copied();
                        let vel = self.ecs.read_storage::<comp::Vel>().get(entity).copied();
                        if let (Some(pos), Some(ori), Some(vel)) = (pos, ori, vel) {
                            let _ = self
                                .ecs
                                .write_storage()
                                .insert(mounter, comp::Pos(pos.0 + Vec3::unit_z() * 1.0));
                            let _ = self.ecs.write_storage().insert(mounter, ori);
                            let _ = self.ecs.write_storage().insert(mounter, vel);
                        }
                        let _ = self
                            .ecs
                            .write_storage::<comp::Controller>()
                            .insert(entity, controller);
                    } else {
                        *mount_state = comp::MountState::Unmounted;
                    }
                }
            }
        }

        let mut to_unmount = Vec::new();
        for (entity, comp::Mounting(mountee)) in (
            &self.ecs.entities(),
            &self.ecs.read_storage::<comp::Mounting>(),
        )
            .join()
        {
            if self
                .ecs
                .entity_from_uid(mountee.id())
                .filter(|mountee| self.ecs.is_alive(*mountee))
                .is_none()
            {
                to_unmount.push(entity);
            }
        }
        for entity in to_unmount {
            self.ecs.write_storage::<comp::Mounting>().remove(entity);
        }

        // Run systems to update the world.
        // Create and run a dispatcher for ecs systems.
        let mut dispatch_builder = DispatcherBuilder::new().with_pool(self.thread_pool.clone());
        sys::add_local_systems(&mut dispatch_builder);
        // This dispatches all the systems in parallel.
        dispatch_builder.build().dispatch(&self.ecs.res);

        self.ecs.maintain();

        // Apply terrain changes
        let mut terrain = self.ecs.write_resource::<TerrainGrid>();
        self.ecs
            .read_resource::<BlockChange>()
            .blocks
            .iter()
            .for_each(|(pos, block)| {
                let _ = terrain.set(*pos, *block);
            });
        self.ecs.write_resource::<TerrainChanges>().modified_blocks = std::mem::replace(
            &mut self.ecs.write_resource::<BlockChange>().blocks,
            Default::default(),
        );

        // Process local events
        let events = self.ecs.read_resource::<EventBus<LocalEvent>>().recv_all();
        for event in events {
            let mut velocities = self.ecs.write_storage::<comp::Vel>();
            let mut controllers = self.ecs.write_storage::<comp::Controller>();
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
                LocalEvent::WallLeap { entity, wall_dir } => {
                    if let (Some(vel), Some(_controller)) =
                        (velocities.get_mut(entity), controllers.get_mut(entity))
                    {
                        let hspeed = Vec2::<f32>::from(vel.0).magnitude();
                        if hspeed > 0.001 && hspeed < 0.5 {
                            vel.0 += vel.0.normalized()
                                * Vec3::new(1.0, 1.0, 0.0)
                                * HUMANOID_JUMP_ACCEL
                                * 1.5
                                - wall_dir * 0.03;
                            vel.0.z = HUMANOID_JUMP_ACCEL * 0.5;
                        }
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
        // Clean up data structures from the last tick.
        self.ecs.write_resource::<TerrainChanges>().clear();
    }
}
