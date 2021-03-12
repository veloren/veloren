#[cfg(feature = "plugins")]
use crate::plugin::memory_manager::EcsWorld;
#[cfg(feature = "plugins")]
use crate::plugin::PluginMgr;
use common::{
    comp,
    event::{EventBus, LocalEvent, ServerEvent},
    region::RegionMap,
    resources::{DeltaTime, GameMode, Time, TimeOfDay},
    terrain::{Block, TerrainChunk, TerrainGrid},
    time::DayPeriod,
    trade::Trades,
    uid::UidAllocator,
    vol::{ReadVol, WriteVol},
};
use common_base::span;
use common_ecs::{PhysicsMetrics, SysMetrics};
use common_net::sync::WorldSyncExt;
use hashbrown::{HashMap, HashSet};
use rayon::{ThreadPool, ThreadPoolBuilder};
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

#[derive(Copy, Clone)]
pub enum ExecMode {
    Server,
    Client,
    Singleplayer,
}

/// A type used to represent game state stored on both the client and the
/// server. This includes things like entity components, terrain data, and
/// global states like weather, time of day, etc.
pub struct State {
    ecs: specs::World,
    // Avoid lifetime annotation by storing a thread pool instead of the whole dispatcher
    thread_pool: Arc<ThreadPool>,
}

impl State {
    /// Create a new `State` in client mode.
    pub fn client() -> Self { Self::new(GameMode::Client) }

    /// Create a new `State` in server mode.
    pub fn server() -> Self { Self::new(GameMode::Server) }

    pub fn new(game_mode: GameMode) -> Self {
        let thread_name_infix = match game_mode {
            GameMode::Server => "s",
            GameMode::Client => "c",
            GameMode::Singleplayer => "sp",
        };

        let thread_pool = Arc::new(
            ThreadPoolBuilder::new()
                .thread_name(move |i| format!("rayon-{}-{}", thread_name_infix, i))
                .build()
                .unwrap(),
        );
        Self {
            ecs: Self::setup_ecs_world(game_mode),
            thread_pool,
        }
    }

    /// Creates ecs world and registers all the common components and resources
    // TODO: Split up registering into server and client (e.g. move
    // EventBus<ServerEvent> to the server)
    fn setup_ecs_world(game_mode: GameMode) -> specs::World {
        let mut ecs = specs::World::new();
        // Uids for sync
        ecs.register_sync_marker();
        // Register server -> all clients synced components.
        ecs.register::<comp::Body>();
        ecs.register::<comp::Player>();
        ecs.register::<comp::Stats>();
        ecs.register::<comp::Buffs>();
        ecs.register::<comp::Auras>();
        ecs.register::<comp::Energy>();
        ecs.register::<comp::Combo>();
        ecs.register::<comp::Health>();
        ecs.register::<comp::Poise>();
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
        ecs.register::<comp::Object>();
        ecs.register::<comp::Group>();
        ecs.register::<comp::Shockwave>();
        ecs.register::<comp::ShockwaveHitEntities>();
        ecs.register::<comp::BeamSegment>();

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
        ecs.register::<comp::Alignment>();
        ecs.register::<comp::Agent>();
        ecs.register::<comp::WaypointArea>();
        ecs.register::<comp::ForceUpdate>();
        ecs.register::<comp::InventoryUpdate>();
        ecs.register::<comp::Admin>();
        ecs.register::<comp::Waypoint>();
        ecs.register::<comp::Projectile>();
        ecs.register::<comp::Melee>();
        ecs.register::<comp::ItemDrop>();
        ecs.register::<comp::ChatMode>();
        ecs.register::<comp::Faction>();
        ecs.register::<comp::invite::Invite>();
        ecs.register::<comp::invite::PendingInvites>();
        ecs.register::<comp::Beam>();
        ecs.register::<comp::PreviousPhysCache>();

        // Register synced resources used by the ECS.
        ecs.insert(TimeOfDay(0.0));

        // Register unsynced resources used by the ECS.
        ecs.insert(Time(0.0));
        ecs.insert(DeltaTime(0.0));
        ecs.insert(TerrainGrid::new().unwrap());
        ecs.insert(BlockChange::default());
        ecs.insert(TerrainChanges::default());
        ecs.insert(EventBus::<LocalEvent>::default());
        ecs.insert(game_mode);
        ecs.insert(Vec::<common::outcome::Outcome>::new());
        // TODO: only register on the server
        ecs.insert(EventBus::<ServerEvent>::default());
        ecs.insert(comp::group::GroupManager::default());
        ecs.insert(RegionMap::new());
        ecs.insert(SysMetrics::default());
        ecs.insert(PhysicsMetrics::default());
        ecs.insert(Trades::default());

        // Load plugins from asset directory
        #[cfg(feature = "plugins")]
        ecs.insert(match PluginMgr::from_assets() {
            Ok(plugin_mgr) => {
                let ecs_world = EcsWorld {
                    entities: &ecs.entities(),
                    health: ecs.read_component().into(),
                    uid: ecs.read_component().into(),
                    uid_allocator: &ecs.read_resource::<UidAllocator>().into(),
                    player: ecs.read_component().into(),
                };
                if let Err(e) = plugin_mgr
                    .execute_event(&ecs_world, &plugin_api::event::PluginLoadEvent {
                        game_mode,
                    })
                {
                    tracing::error!(?e, "Failed to run plugin init");
                    tracing::info!(
                        "Error occurred when loading plugins. Running without plugins instead."
                    );
                    PluginMgr::default()
                } else {
                    plugin_mgr
                }
            },
            Err(e) => {
                tracing::error!(?e, "Failed to read plugins from assets");
                tracing::info!(
                    "Error occurred when loading plugins. Running without plugins instead."
                );
                PluginMgr::default()
            },
        });

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

    /// Read a component attributed to a particular entity.
    pub fn read_component_copied<C: Component + Copy>(&self, entity: EcsEntity) -> Option<C> {
        self.ecs.read_storage().get(entity).copied()
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

    /// Get the current in-game day period (period of the day/night cycle)
    /// Get the current in-game day period (period of the day/night cycle)
    pub fn get_day_period(&self) -> DayPeriod { self.get_time_of_day().into() }

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

    /// Get a block in this state's terrain.
    pub fn get_block(&self, pos: Vec3<i32>) -> Option<Block> {
        self.terrain().get(pos).ok().copied()
    }

    /// Set a block in this state's terrain.
    pub fn set_block(&mut self, pos: Vec3<i32>, block: Block) {
        self.ecs.write_resource::<BlockChange>().set(pos, block);
    }

    /// Check if the block at given position `pos` has already been modified
    /// this tick.
    pub fn can_set_block(&mut self, pos: Vec3<i32>) -> bool {
        !self
            .ecs
            .write_resource::<BlockChange>()
            .blocks
            .contains_key(&pos)
    }

    /// Removes every chunk of the terrain.
    pub fn clear_terrain(&mut self) {
        let removed_chunks = &mut self.ecs.write_resource::<TerrainChanges>().removed_chunks;

        self.terrain_mut().drain().for_each(|(key, _)| {
            removed_chunks.insert(key);
        });
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
        span!(_guard, "update_region_map", "State::update_region_map");
        self.ecs.write_resource::<RegionMap>().tick(
            self.ecs.read_storage::<comp::Pos>(),
            self.ecs.read_storage::<comp::Vel>(),
            self.ecs.entities(),
        );
    }

    // Apply terrain changes
    pub fn apply_terrain_changes(&self) {
        span!(
            _guard,
            "apply_terrain_changes",
            "State::apply_terrain_changes"
        );
        let mut terrain = self.ecs.write_resource::<TerrainGrid>();
        let mut modified_blocks =
            std::mem::take(&mut self.ecs.write_resource::<BlockChange>().blocks);
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
        span!(_guard, "tick", "State::tick");
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

        span!(guard, "create dispatcher");
        // Run systems to update the world.
        // Create and run a dispatcher for ecs systems.
        let mut dispatch_builder =
            DispatcherBuilder::new().with_pool(Arc::clone(&self.thread_pool));
        crate::add_local_systems(&mut dispatch_builder);
        // TODO: Consider alternative ways to do this
        add_foreign_systems(&mut dispatch_builder);
        // This dispatches all the systems in parallel.
        let mut dispatcher = dispatch_builder.build();
        drop(guard);
        span!(guard, "run systems");
        dispatcher.dispatch(&self.ecs);
        drop(guard);

        span!(guard, "maintain ecs");
        self.ecs.maintain();
        drop(guard);

        if update_terrain_and_regions {
            self.apply_terrain_changes();
        }

        // Process local events
        span!(guard, "process local events");
        let events = self.ecs.read_resource::<EventBus<LocalEvent>>().recv_all();
        for event in events {
            let mut velocities = self.ecs.write_storage::<comp::Vel>();
            let mut controllers = self.ecs.write_storage::<comp::Controller>();
            let physics = self.ecs.read_storage::<comp::PhysicsState>();
            match event {
                LocalEvent::Jump(entity) => {
                    if let Some(vel) = velocities.get_mut(entity) {
                        vel.0.z = HUMANOID_JUMP_ACCEL
                            + physics.get(entity).map_or(0.0, |ps| ps.ground_vel.z);
                    }
                },
                LocalEvent::ApplyImpulse { entity, impulse } => {
                    if let Some(vel) = velocities.get_mut(entity) {
                        vel.0 = impulse;
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
        drop(guard);
    }

    /// Clean up the state after a tick.
    pub fn cleanup(&mut self) {
        span!(_guard, "cleanup", "State::cleanup");
        // Clean up data structures from the last tick.
        self.ecs.write_resource::<TerrainChanges>().clear();
    }
}
