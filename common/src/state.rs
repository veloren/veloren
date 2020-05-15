use crate::{
    comp,
    event::{EventBus, LocalEvent, ServerEvent, SfxEventItem},
    region::RegionMap,
    sync::WorldSyncExt,
    sys,
    terrain::{Block, TerrainChunk, TerrainGrid},
    vol::WriteVol,
};
use hashbrown::{HashMap, HashSet};
use rayon::{ThreadPool, ThreadPoolBuilder};
use serde_derive::{Deserialize, Serialize};
use specs::{
    shred::{Fetch, FetchMut},
    storage::{MaskedStorage as EcsMaskedStorage, Storage as EcsStorage},
    Component, DispatcherBuilder, Entity as EcsEntity, WorldExt,
};
use std::{sync::Arc, time::Duration};
use vek::*;

/// How much faster should an in-game day be compared to a real day?
// TODO: Don't hard-code this.
const DAY_CYCLE_FACTOR: f64 = 24.0 * 2.0;

/// A resource that stores the time of day.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct TimeOfDay(pub f64);

/// A resource that stores the tick (i.e: physics) time.
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Time(pub f64);

/// A resource that stores the time since the previous tick.
#[derive(Default)]
pub struct DeltaTime(pub f32);

/// At what point should we stop speeding up physics to compensate for lag? If
/// we speed physics up too fast, we'd skip important physics events like
/// collisions. This constant determines the upper limit. If delta time exceeds
/// this value, the game's physics will begin to produce time lag. Ideally, we'd
/// avoid such a situation.
const MAX_DELTA_TIME: f32 = 1.0;
const HUMANOID_JUMP_ACCEL: f32 = 16.0;

#[derive(Default)]
pub struct BlockChange {
    blocks: HashMap<Vec3<i32>, Block>,
}

impl BlockChange {
    pub fn set(&mut self, pos: Vec3<i32>, block: Block) { self.blocks.insert(pos, block); }

    pub fn try_set(&mut self, pos: Vec3<i32>, block: Block) -> Option<()> {
        if !self.blocks.contains_key(&pos) {
            self.blocks.insert(pos, block);
            Some(())
        } else {
            None
        }
    }

    pub fn clear(&mut self) { self.blocks.clear(); }
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

/// A type used to represent game state stored on both the client and the
/// server. This includes things like entity components, terrain data, and
/// global states like weather, time of day, etc.
pub struct State {
    ecs: specs::World,
    // Avoid lifetime annotation by storing a thread pool instead of the whole dispatcher
    thread_pool: Arc<ThreadPool>,
}

impl Default for State {
    /// Create a new `State`.
    fn default() -> Self {
        Self {
            ecs: Self::setup_ecs_world(),
            thread_pool: Arc::new(ThreadPoolBuilder::new().build().unwrap()),
        }
    }
}

impl State {
    /// Creates ecs world and registers all the common components and resources
    // TODO: Split up registering into server and client (e.g. move
    // EventBus<ServerEvent> to the server)
    fn setup_ecs_world() -> specs::World {
        let mut ecs = specs::World::new();
        // Uids for sync
        ecs.register_sync_marker();
        // Register server -> all clients synced components.
        ecs.register::<comp::Loadout>();
        ecs.register::<comp::Body>();
        ecs.register::<comp::Player>();
        ecs.register::<comp::Stats>();
        ecs.register::<comp::Energy>();
        ecs.register::<comp::CanBuild>();
        ecs.register::<comp::LightEmitter>();
        ecs.register::<comp::Item>();
        ecs.register::<comp::Scale>();
        ecs.register::<comp::Mounting>();
        ecs.register::<comp::MountState>();
        ecs.register::<comp::Mass>();
        ecs.register::<comp::Collider>();
        ecs.register::<comp::Sticky>();
        ecs.register::<comp::Gravity>();
        ecs.register::<comp::CharacterState>();

        // Register components send from clients -> server
        ecs.register::<comp::Controller>();

        // Register components send directly from server -> all but one client
        ecs.register::<comp::PhysicsState>();

        // Register components synced from client -> server -> all other clients
        ecs.register::<comp::Pos>();
        ecs.register::<comp::Vel>();
        ecs.register::<comp::Ori>();
        ecs.register::<comp::Inventory>();

        // Register client-local components
        // TODO: only register on the client
        ecs.register::<comp::LightAnimation>();

        // Register server-local components
        // TODO: only register on the server
        ecs.register::<comp::Last<comp::Pos>>();
        ecs.register::<comp::Last<comp::Vel>>();
        ecs.register::<comp::Last<comp::Ori>>();
        ecs.register::<comp::Agent>();
        ecs.register::<comp::Alignment>();
        ecs.register::<comp::WaypointArea>();
        ecs.register::<comp::ForceUpdate>();
        ecs.register::<comp::InventoryUpdate>();
        ecs.register::<comp::Admin>();
        ecs.register::<comp::Waypoint>();
        ecs.register::<comp::Projectile>();
        ecs.register::<comp::Attacking>();
        ecs.register::<comp::ItemDrop>();

        // Register synced resources used by the ECS.
        ecs.insert(TimeOfDay(0.0));

        // Register unsynced resources used by the ECS.
        ecs.insert(Time(0.0));
        ecs.insert(DeltaTime(0.0));
        ecs.insert(TerrainGrid::new().unwrap());
        ecs.insert(BlockChange::default());
        ecs.insert(TerrainChanges::default());
        // TODO: only register on the server
        ecs.insert(EventBus::<ServerEvent>::default());
        ecs.insert(EventBus::<LocalEvent>::default());
        ecs.insert(EventBus::<SfxEventItem>::default());
        ecs.insert(RegionMap::new());

        ecs
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
    pub fn ecs(&self) -> &specs::World { &self.ecs }

    /// Get a mutable reference to the internal ECS world.
    pub fn ecs_mut(&mut self) -> &mut specs::World { &mut self.ecs }

    /// Get a reference to the `TerrainChanges` structure of the state. This
    /// contains information about terrain state that has changed since the
    /// last game tick.
    pub fn terrain_changes(&self) -> Fetch<TerrainChanges> { self.ecs.read_resource() }

    /// Get the current in-game time of day.
    ///
    /// Note that this should not be used for physics, animations or other such
    /// localised timings.
    pub fn get_time_of_day(&self) -> f64 { self.ecs.read_resource::<TimeOfDay>().0 }

    /// Get the current in-game time.
    ///
    /// Note that this does not correspond to the time of day.
    pub fn get_time(&self) -> f64 { self.ecs.read_resource::<Time>().0 }

    /// Get the current delta time.
    pub fn get_delta_time(&self) -> f32 { self.ecs.read_resource::<DeltaTime>().0 }

    /// Get a reference to this state's terrain.
    pub fn terrain(&self) -> Fetch<TerrainGrid> { self.ecs.read_resource() }

    /// Get a writable reference to this state's terrain.
    pub fn terrain_mut(&self) -> FetchMut<TerrainGrid> { self.ecs.write_resource() }

    /// Set a block in this state's terrain.
    pub fn set_block(&mut self, pos: Vec3<i32>, block: Block) {
        self.ecs.write_resource::<BlockChange>().set(pos, block);
    }

    /// Set a block in this state's terrain. Will return `None` if the block has
    /// already been modified this tick.
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

    /// Remove the chunk with the given key from this state's terrain, if it
    /// exists.
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

    // Run RegionMap tick to update entity region occupancy
    pub fn update_region_map(&self) {
        self.ecs.write_resource::<RegionMap>().tick(
            self.ecs.read_storage::<comp::Pos>(),
            self.ecs.read_storage::<comp::Vel>(),
            self.ecs.entities(),
        );
    }

    // Apply terrain changes
    pub fn apply_terrain_changes(&self) {
        let mut terrain = self.ecs.write_resource::<TerrainGrid>();
        let mut modified_blocks = std::mem::replace(
            &mut self.ecs.write_resource::<BlockChange>().blocks,
            Default::default(),
        );
        // Apply block modifications
        // Only include in `TerrainChanges` if successful
        modified_blocks.retain(|pos, block| terrain.set(*pos, *block).is_ok());
        self.ecs.write_resource::<TerrainChanges>().modified_blocks = modified_blocks;
    }

    /// Execute a single tick, simulating the game state by the given duration.
    pub fn tick(
        &mut self,
        dt: Duration,
        add_foreign_systems: impl Fn(&mut DispatcherBuilder),
        update_terrain_and_regions: bool,
    ) {
        // Change the time accordingly.
        self.ecs.write_resource::<TimeOfDay>().0 += dt.as_secs_f64() * DAY_CYCLE_FACTOR;
        self.ecs.write_resource::<Time>().0 += dt.as_secs_f64();

        // Update delta time.
        // Beyond a delta time of MAX_DELTA_TIME, start lagging to avoid skipping
        // important physics events.
        self.ecs.write_resource::<DeltaTime>().0 = dt.as_secs_f32().min(MAX_DELTA_TIME);

        if update_terrain_and_regions {
            self.update_region_map();
        }

        // Run systems to update the world.
        // Create and run a dispatcher for ecs systems.
        let mut dispatch_builder = DispatcherBuilder::new().with_pool(self.thread_pool.clone());
        sys::add_local_systems(&mut dispatch_builder);
        // TODO: Consider alternative ways to do this
        add_foreign_systems(&mut dispatch_builder);
        // This dispatches all the systems in parallel.
        dispatch_builder.build().dispatch(&self.ecs);

        self.ecs.maintain();

        if update_terrain_and_regions {
            self.apply_terrain_changes();
        }

        // Process local events
        let events = self.ecs.read_resource::<EventBus<LocalEvent>>().recv_all();
        for event in events {
            let mut velocities = self.ecs.write_storage::<comp::Vel>();
            let mut controllers = self.ecs.write_storage::<comp::Controller>();
            match event {
                LocalEvent::Jump(entity) => {
                    if let Some(vel) = velocities.get_mut(entity) {
                        vel.0.z = HUMANOID_JUMP_ACCEL;
                    }
                },
                LocalEvent::ApplyForce { entity, force } => {
                    // TODO: this sets the velocity directly to the value of `force`, consider
                    // renaming the event or changing the behavior
                    if let Some(vel) = velocities.get_mut(entity) {
                        vel.0 = force;
                    }
                },
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
                },
                LocalEvent::Boost {
                    entity,
                    vel: extra_vel,
                } => {
                    if let Some(vel) = velocities.get_mut(entity) {
                        vel.0 += extra_vel;
                    }
                },
            }
        }
    }

    /// Clean up the state after a tick.
    pub fn cleanup(&mut self) {
        // Clean up data structures from the last tick.
        self.ecs.write_resource::<TerrainChanges>().clear();
    }
}
