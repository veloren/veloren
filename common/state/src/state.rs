#[cfg(feature = "plugins")]
use crate::plugin::memory_manager::EcsWorld;
#[cfg(feature = "plugins")]
use crate::plugin::PluginMgr;
use crate::{BuildArea, NoDurabilityArea};
#[cfg(feature = "plugins")]
use common::uid::IdMaps;
use common::{
    calendar::Calendar,
    comp,
    event::{EventBus, LocalEvent},
    link::Is,
    mounting::{Mount, Rider, VolumeRider, VolumeRiders},
    outcome::Outcome,
    resources::{
        DeltaTime, EntitiesDiedLastTick, GameMode, PlayerEntity, PlayerPhysicsSettings,
        ProgramTime, Time, TimeOfDay, TimeScale,
    },
    shared_server_config::ServerConstants,
    slowjob::SlowJobPool,
    terrain::{Block, MapSizeLg, TerrainChunk, TerrainGrid},
    tether,
    time::DayPeriod,
    trade::Trades,
    vol::{ReadVol, WriteVol},
    weather::{Weather, WeatherGrid},
};
use common_base::{prof_span, span};
use common_ecs::{PhysicsMetrics, SysMetrics};
use common_net::sync::{interpolation as sync_interp, WorldSyncExt};
use core::{convert::identity, time::Duration};
use hashbrown::{HashMap, HashSet};
use rayon::{ThreadPool, ThreadPoolBuilder};
use specs::{
    prelude::Resource,
    shred::{Fetch, FetchMut, SendDispatcher},
    storage::{MaskedStorage as EcsMaskedStorage, Storage as EcsStorage},
    Component, DispatcherBuilder, Entity as EcsEntity, WorldExt,
};
use std::{sync::Arc, time::Instant};
use timer_queue::TimerQueue;
use vek::*;

/// At what point should we stop speeding up physics to compensate for lag? If
/// we speed physics up too fast, we'd skip important physics events like
/// collisions. This constant determines the upper limit. If delta time exceeds
/// this value, the game's physics will begin to produce time lag. Ideally, we'd
/// avoid such a situation.
const MAX_DELTA_TIME: f32 = 1.0;
/// convert seconds to milliseconds to use in TimerQueue
const SECONDS_TO_MILLISECONDS: f64 = 1000.0;

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

    /// Check if the block at given position `pos` has already been modified
    /// this tick.
    pub fn can_set_block(&self, pos: Vec3<i32>) -> bool { !self.blocks.contains_key(&pos) }

    pub fn clear(&mut self) { self.blocks.clear(); }
}

#[derive(Default)]
pub struct ScheduledBlockChange {
    changes: TimerQueue<HashMap<Vec3<i32>, Block>>,
    outcomes: TimerQueue<HashMap<Vec3<i32>, Block>>,
    last_poll_time: u64,
}
impl ScheduledBlockChange {
    pub fn set(&mut self, pos: Vec3<i32>, block: Block, replace_time: f64) {
        let timer = self.changes.insert(
            (replace_time * SECONDS_TO_MILLISECONDS) as u64,
            HashMap::new(),
        );
        self.changes.get_mut(timer).insert(pos, block);
    }

    pub fn outcome_set(&mut self, pos: Vec3<i32>, block: Block, replace_time: f64) {
        let outcome_timer = self.outcomes.insert(
            (replace_time * SECONDS_TO_MILLISECONDS) as u64,
            HashMap::new(),
        );
        self.outcomes.get_mut(outcome_timer).insert(pos, block);
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

#[derive(Clone)]
pub struct BlockDiff {
    pub wpos: Vec3<i32>,
    pub old: Block,
    pub new: Block,
}

/// A type used to represent game state stored on both the client and the
/// server. This includes things like entity components, terrain data, and
/// global states like weather, time of day, etc.
pub struct State {
    ecs: specs::World,
    // Avoid lifetime annotation by storing a thread pool instead of the whole dispatcher
    thread_pool: Arc<ThreadPool>,
    dispatcher: SendDispatcher<'static>,
}

pub type Pools = Arc<ThreadPool>;

impl State {
    pub fn pools(game_mode: GameMode) -> Pools {
        let thread_name_infix = match game_mode {
            GameMode::Server => "s",
            GameMode::Client => "c",
            GameMode::Singleplayer => "sp",
        };

        Arc::new(
            ThreadPoolBuilder::new()
                .num_threads(num_cpus::get().max(common::consts::MIN_RECOMMENDED_RAYON_THREADS))
                .thread_name(move |i| format!("rayon-{}-{}", thread_name_infix, i))
                .build()
                .unwrap(),
        )
    }

    /// Create a new `State` in client mode.
    pub fn client(
        pools: Pools,
        map_size_lg: MapSizeLg,
        default_chunk: Arc<TerrainChunk>,
        add_systems: impl Fn(&mut DispatcherBuilder),
    ) -> Self {
        Self::new(
            GameMode::Client,
            pools,
            map_size_lg,
            default_chunk,
            add_systems,
        )
    }

    /// Create a new `State` in server mode.
    pub fn server(
        pools: Pools,
        map_size_lg: MapSizeLg,
        default_chunk: Arc<TerrainChunk>,
        add_systems: impl Fn(&mut DispatcherBuilder),
    ) -> Self {
        Self::new(
            GameMode::Server,
            pools,
            map_size_lg,
            default_chunk,
            add_systems,
        )
    }

    pub fn new(
        game_mode: GameMode,
        pools: Pools,
        map_size_lg: MapSizeLg,
        default_chunk: Arc<TerrainChunk>,
        add_systems: impl Fn(&mut DispatcherBuilder),
    ) -> Self {
        prof_span!(guard, "create dispatcher");
        let mut dispatch_builder =
            DispatcherBuilder::<'static, 'static>::new().with_pool(Arc::clone(&pools));
        // TODO: Consider alternative ways to do this
        add_systems(&mut dispatch_builder);
        let dispatcher = dispatch_builder
            .build()
            .try_into_sendable()
            .unwrap_or_else(|_| panic!("Thread local systems not allowed"));
        drop(guard);

        Self {
            ecs: Self::setup_ecs_world(game_mode, Arc::clone(&pools), map_size_lg, default_chunk),
            thread_pool: pools,
            dispatcher,
        }
    }

    /// Creates ecs world and registers all the common components and resources
    // TODO: Split up registering into server and client (e.g. move
    // EventBus<ServerEvent> to the server)
    fn setup_ecs_world(
        game_mode: GameMode,
        thread_pool: Arc<ThreadPool>,
        map_size_lg: MapSizeLg,
        default_chunk: Arc<TerrainChunk>,
    ) -> specs::World {
        prof_span!("State::setup_ecs_world");
        let mut ecs = specs::World::new();
        // Uids for sync
        ecs.register_sync_marker();
        // Register server -> all clients synced components.
        ecs.register::<comp::Body>();
        ecs.register::<comp::Player>();
        ecs.register::<comp::Stats>();
        ecs.register::<comp::SkillSet>();
        ecs.register::<comp::ActiveAbilities>();
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
        ecs.register::<Is<Mount>>();
        ecs.register::<Is<Rider>>();
        ecs.register::<Is<VolumeRider>>();
        ecs.register::<Is<tether::Leader>>();
        ecs.register::<Is<tether::Follower>>();
        ecs.register::<comp::Mass>();
        ecs.register::<comp::Density>();
        ecs.register::<comp::Collider>();
        ecs.register::<comp::Sticky>();
        ecs.register::<comp::Immovable>();
        ecs.register::<comp::CharacterState>();
        ecs.register::<comp::CharacterActivity>();
        ecs.register::<comp::Object>();
        ecs.register::<comp::Group>();
        ecs.register::<comp::Shockwave>();
        ecs.register::<comp::ShockwaveHitEntities>();
        ecs.register::<comp::Beam>();
        ecs.register::<comp::Alignment>();
        ecs.register::<comp::LootOwner>();
        ecs.register::<comp::Admin>();
        ecs.register::<comp::Stance>();
        ecs.register::<comp::Teleporting>();

        // Register components send from clients -> server
        ecs.register::<comp::Controller>();

        // Register components send directly from server -> all but one client
        ecs.register::<comp::PhysicsState>();

        // Register components synced from client -> server -> all other clients
        ecs.register::<comp::Pos>();
        ecs.register::<comp::Vel>();
        ecs.register::<comp::Ori>();
        ecs.register::<comp::Inventory>();

        // Register common unsynced components
        ecs.register::<comp::PreviousPhysCache>();
        ecs.register::<comp::PosVelOriDefer>();

        // Register client-local components
        // TODO: only register on the client
        ecs.register::<comp::LightAnimation>();
        ecs.register::<sync_interp::InterpBuffer<comp::Pos>>();
        ecs.register::<sync_interp::InterpBuffer<comp::Vel>>();
        ecs.register::<sync_interp::InterpBuffer<comp::Ori>>();

        // Register server-local components
        // TODO: only register on the server
        ecs.register::<comp::Last<comp::Pos>>();
        ecs.register::<comp::Last<comp::Vel>>();
        ecs.register::<comp::Last<comp::Ori>>();
        ecs.register::<comp::Agent>();
        ecs.register::<comp::WaypointArea>();
        ecs.register::<comp::ForceUpdate>();
        ecs.register::<comp::InventoryUpdate>();
        ecs.register::<comp::Waypoint>();
        ecs.register::<comp::MapMarker>();
        ecs.register::<comp::Projectile>();
        ecs.register::<comp::Melee>();
        ecs.register::<comp::ItemDrops>();
        ecs.register::<comp::ChatMode>();
        ecs.register::<comp::Faction>();
        ecs.register::<comp::invite::Invite>();
        ecs.register::<comp::invite::PendingInvites>();
        ecs.register::<VolumeRiders>();

        // Register synced resources used by the ECS.
        ecs.insert(TimeOfDay(0.0));
        ecs.insert(Calendar::default());
        ecs.insert(WeatherGrid::new(Vec2::zero()));
        ecs.insert(Time(0.0));
        ecs.insert(ProgramTime(0.0));
        ecs.insert(TimeScale(1.0));

        // Register unsynced resources used by the ECS.
        ecs.insert(DeltaTime(0.0));
        ecs.insert(PlayerEntity(None));
        ecs.insert(TerrainGrid::new(map_size_lg, default_chunk).unwrap());
        ecs.insert(BlockChange::default());
        ecs.insert(ScheduledBlockChange::default());
        ecs.insert(crate::special_areas::AreasContainer::<BuildArea>::default());
        ecs.insert(crate::special_areas::AreasContainer::<NoDurabilityArea>::default());
        ecs.insert(TerrainChanges::default());
        ecs.insert(EventBus::<LocalEvent>::default());
        ecs.insert(game_mode);
        ecs.insert(EventBus::<Outcome>::default());
        ecs.insert(common::CachedSpatialGrid::default());
        ecs.insert(EntitiesDiedLastTick::default());

        let num_cpu = num_cpus::get() as u64;
        let slow_limit = (num_cpu / 2 + num_cpu / 4).max(1);
        tracing::trace!(?slow_limit, "Slow Thread limit");
        ecs.insert(SlowJobPool::new(slow_limit, 10_000, thread_pool));

        // TODO: only register on the server
        ecs.insert(comp::group::GroupManager::default());
        ecs.insert(SysMetrics::default());
        ecs.insert(PhysicsMetrics::default());
        ecs.insert(Trades::default());
        ecs.insert(PlayerPhysicsSettings::default());
        ecs.insert(VolumeRiders::default());

        // Load plugins from asset directory
        #[cfg(feature = "plugins")]
        ecs.insert(match PluginMgr::from_assets() {
            Ok(mut plugin_mgr) => {
                let ecs_world = EcsWorld {
                    entities: &ecs.entities(),
                    health: ecs.read_component().into(),
                    uid: ecs.read_component().into(),
                    id_maps: &ecs.read_resource::<IdMaps>().into(),
                    player: ecs.read_component().into(),
                };
                if let Err(e) = plugin_mgr
                    .execute_event(&ecs_world, &plugin_api::event::PluginLoadEvent {
                        game_mode,
                    })
                {
                    tracing::debug!(?e, "Failed to run plugin init");
                    tracing::info!("Plugins disabled, enable debug logging for more information.");
                    PluginMgr::default()
                } else {
                    plugin_mgr
                }
            },
            Err(e) => {
                tracing::debug!(?e, "Failed to read plugins from assets");
                tracing::info!("Plugins disabled, enable debug logging for more information.");
                PluginMgr::default()
            },
        });

        ecs
    }

    /// Register a component with the state's ECS.
    #[must_use]
    pub fn with_component<T: Component>(mut self) -> Self
    where
        <T as Component>::Storage: Default,
    {
        self.ecs.register::<T>();
        self
    }

    /// Write a component attributed to a particular entity, ignoring errors.
    ///
    /// This should be used *only* when we can guarantee that the rest of the
    /// code does not rely on the insert having succeeded (meaning the
    /// entity is no longer alive!).
    ///
    /// Returns None if the entity was dead or there was no previous entry for
    /// this component; otherwise, returns Some(old_component).
    pub fn write_component_ignore_entity_dead<C: Component>(
        &mut self,
        entity: EcsEntity,
        comp: C,
    ) -> Option<C> {
        self.ecs
            .write_storage()
            .insert(entity, comp)
            .ok()
            .and_then(identity)
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

    /// # Panics
    /// Panics if `EventBus<E>` is borrowed
    pub fn emit_event_now<E>(&self, event: E)
    where
        EventBus<E>: Resource,
    {
        self.ecs.write_resource::<EventBus<E>>().emit_now(event)
    }

    /// Given mutable access to the resource R, assuming the resource
    /// component exists (this is already the behavior of functions like `fetch`
    /// and `write_component_ignore_entity_dead`).  Since all of our resources
    /// are generated up front, any failure here is definitely a code bug.
    pub fn mut_resource<R: Resource>(&mut self) -> &mut R {
        self.ecs.get_mut::<R>().expect(
            "Tried to fetch an invalid resource even though all our resources should be known at \
             compile time.",
        )
    }

    /// Get a read-only reference to the storage of a particular component type.
    pub fn read_storage<C: Component>(&self) -> EcsStorage<C, Fetch<EcsMaskedStorage<C>>> {
        self.ecs.read_storage::<C>()
    }

    /// Get a reference to the internal ECS world.
    pub fn ecs(&self) -> &specs::World { &self.ecs }

    /// Get a mutable reference to the internal ECS world.
    pub fn ecs_mut(&mut self) -> &mut specs::World { &mut self.ecs }

    pub fn thread_pool(&self) -> &Arc<ThreadPool> { &self.thread_pool }

    /// Get a reference to the `TerrainChanges` structure of the state. This
    /// contains information about terrain state that has changed since the
    /// last game tick.
    pub fn terrain_changes(&self) -> Fetch<TerrainChanges> { self.ecs.read_resource() }

    /// Get a reference the current in-game weather grid.
    pub fn weather_grid(&self) -> Fetch<WeatherGrid> { self.ecs.read_resource() }

    /// Get a mutable reference the current in-game weather grid.
    pub fn weather_grid_mut(&mut self) -> FetchMut<WeatherGrid> { self.ecs.write_resource() }

    /// Get the current weather at a position in worldspace.
    pub fn weather_at(&self, pos: Vec2<f32>) -> Weather {
        self.weather_grid().get_interpolated(pos)
    }

    /// Get the max weather near a position in worldspace.
    pub fn max_weather_near(&self, pos: Vec2<f32>) -> Weather {
        self.weather_grid().get_max_near(pos)
    }

    /// Get the current in-game time of day.
    ///
    /// Note that this should not be used for physics, animations or other such
    /// localised timings.
    pub fn get_time_of_day(&self) -> f64 { self.ecs.read_resource::<TimeOfDay>().0 }

    /// Get the current in-game day period (period of the day/night cycle)
    pub fn get_day_period(&self) -> DayPeriod { self.get_time_of_day().into() }

    /// Get the current in-game time.
    ///
    /// Note that this does not correspond to the time of day.
    pub fn get_time(&self) -> f64 { self.ecs.read_resource::<Time>().0 }

    /// Get the current true in-game time, unaffected by time_scale.
    ///
    /// Note that this does not correspond to the time of day.
    pub fn get_program_time(&self) -> f64 { self.ecs.read_resource::<ProgramTime>().0 }

    /// Get the current delta time.
    pub fn get_delta_time(&self) -> f32 { self.ecs.read_resource::<DeltaTime>().0 }

    /// Get a reference to this state's terrain.
    pub fn terrain(&self) -> Fetch<TerrainGrid> { self.ecs.read_resource() }

    /// Get a reference to this state's terrain.
    pub fn slow_job_pool(&self) -> Fetch<SlowJobPool> { self.ecs.read_resource() }

    /// Get a writable reference to this state's terrain.
    pub fn terrain_mut(&self) -> FetchMut<TerrainGrid> { self.ecs.write_resource() }

    /// Get a block in this state's terrain.
    pub fn get_block(&self, pos: Vec3<i32>) -> Option<Block> {
        self.terrain().get(pos).ok().copied()
    }

    /// Set a block in this state's terrain.
    pub fn set_block(&self, pos: Vec3<i32>, block: Block) {
        self.ecs.write_resource::<BlockChange>().set(pos, block);
    }

    /// Set a block in this state's terrain (used to delete temporary summoned
    /// sprites after a timeout).
    pub fn schedule_set_block(
        &self,
        pos: Vec3<i32>,
        block: Block,
        sprite_block: Block,
        replace_time: f64,
    ) {
        self.ecs
            .write_resource::<ScheduledBlockChange>()
            .set(pos, block, replace_time);
        self.ecs
            .write_resource::<ScheduledBlockChange>()
            .outcome_set(pos, sprite_block, replace_time);
    }

    /// Check if the block at given position `pos` has already been modified
    /// this tick.
    pub fn can_set_block(&self, pos: Vec3<i32>) -> bool {
        self.ecs.read_resource::<BlockChange>().can_set_block(pos)
    }

    /// Removes every chunk of the terrain.
    pub fn clear_terrain(&mut self) {
        let removed_chunks = &mut self.ecs.write_resource::<TerrainChanges>().removed_chunks;

        self.terrain_mut().drain().for_each(|(key, _)| {
            removed_chunks.insert(key);
        });
    }

    /// Insert the provided chunk into this state's terrain.
    pub fn insert_chunk(&mut self, key: Vec2<i32>, chunk: Arc<TerrainChunk>) {
        if self
            .ecs
            .write_resource::<TerrainGrid>()
            .insert(key, chunk)
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

    // Apply terrain changes
    pub fn apply_terrain_changes(&self, block_update: impl FnMut(&specs::World, Vec<BlockDiff>)) {
        self.apply_terrain_changes_internal(false, block_update);
    }

    /// `during_tick` is true if and only if this is called from within
    /// [State::tick].
    ///
    /// This only happens if [State::tick] is asked to update terrain itself
    /// (using `update_terrain: true`).  [State::tick] is called from within
    /// both the client and the server ticks, right after handling terrain
    /// messages; currently, client sets it to true and server to false.
    fn apply_terrain_changes_internal(
        &self,
        during_tick: bool,
        mut block_update: impl FnMut(&specs::World, Vec<BlockDiff>),
    ) {
        span!(
            _guard,
            "apply_terrain_changes",
            "State::apply_terrain_changes"
        );
        let mut terrain = self.ecs.write_resource::<TerrainGrid>();
        let mut modified_blocks =
            std::mem::take(&mut self.ecs.write_resource::<BlockChange>().blocks);

        let mut scheduled_changes = self.ecs.write_resource::<ScheduledBlockChange>();
        let current_time: f64 = self.ecs.read_resource::<Time>().0 * SECONDS_TO_MILLISECONDS;
        let current_time = current_time as u64;
        // This is important as the poll function has a debug assert that the new poll
        // is at a more recent time than the old poll. As Time is synced between server
        // and client, there is a chance that client dt can get slightly ahead of a
        // server update, so we do not want to panic in that scenario.
        if scheduled_changes.last_poll_time < current_time {
            scheduled_changes.last_poll_time = current_time;
            while let Some(changes) = scheduled_changes.changes.poll(current_time) {
                modified_blocks.extend(changes.iter());
            }
            let outcome = self.ecs.read_resource::<EventBus<Outcome>>();
            while let Some(outcomes) = scheduled_changes.outcomes.poll(current_time) {
                for (pos, block) in outcomes.into_iter() {
                    if let Some(sprite) = block.get_sprite() {
                        outcome.emit_now(Outcome::SpriteDelete { pos, sprite });
                    }
                }
            }
        }
        // Apply block modifications
        // Only include in `TerrainChanges` if successful
        let mut updated_blocks = Vec::with_capacity(modified_blocks.len());
        modified_blocks.retain(|wpos, new| {
            let res = terrain.map(*wpos, |old| {
                updated_blocks.push(BlockDiff {
                    wpos: *wpos,
                    old,
                    new: *new,
                });
                *new
            });
            if let (&Ok(old), true) = (&res, during_tick) {
                // NOTE: If the changes are applied during the tick, we push the *old* value as
                // the modified block (since it otherwise can't be recovered after the tick).
                // Otherwise, the changes will be applied after the tick, so we push the *new*
                // value.
                *new = old;
            }
            res.is_ok()
        });

        if !updated_blocks.is_empty() {
            block_update(&self.ecs, updated_blocks);
        }

        self.ecs.write_resource::<TerrainChanges>().modified_blocks = modified_blocks;
    }

    /// Execute a single tick, simulating the game state by the given duration.
    pub fn tick(
        &mut self,
        dt: Duration,
        update_terrain: bool,
        mut metrics: Option<&mut StateTickMetrics>,
        server_constants: &ServerConstants,
        block_update: impl FnMut(&specs::World, Vec<BlockDiff>),
    ) {
        span!(_guard, "tick", "State::tick");

        // Timing code for server metrics
        macro_rules! section_span {
            ($guard:ident, $label:literal) => {
                span!(span_guard, $label);
                let metrics_guard = metrics.as_mut().map(|m| MetricsGuard::new($label, m));
                let $guard = (span_guard, metrics_guard);
            };
        }

        // Change the time accordingly.
        let time_scale = self.ecs.read_resource::<TimeScale>().0;
        self.ecs.write_resource::<TimeOfDay>().0 +=
            dt.as_secs_f64() * server_constants.day_cycle_coefficient * time_scale;
        self.ecs.write_resource::<Time>().0 += dt.as_secs_f64() * time_scale;
        self.ecs.write_resource::<ProgramTime>().0 += dt.as_secs_f64();

        // Update delta time.
        // Beyond a delta time of MAX_DELTA_TIME, start lagging to avoid skipping
        // important physics events.
        self.ecs.write_resource::<DeltaTime>().0 =
            (dt.as_secs_f32() * time_scale as f32).min(MAX_DELTA_TIME);

        section_span!(guard, "run systems");
        // This dispatches all the systems in parallel.
        self.dispatcher.dispatch(&self.ecs);
        drop(guard);

        self.maintain_ecs();

        if update_terrain {
            self.apply_terrain_changes_internal(true, block_update);
        }

        // Process local events
        section_span!(guard, "process local events");

        let outcomes = self.ecs.read_resource::<EventBus<Outcome>>();
        let mut outcomes_emitter = outcomes.emitter();

        let events = self.ecs.read_resource::<EventBus<LocalEvent>>().recv_all();
        for event in events {
            let mut velocities = self.ecs.write_storage::<comp::Vel>();
            let physics = self.ecs.read_storage::<comp::PhysicsState>();
            match event {
                LocalEvent::Jump(entity, impulse) => {
                    if let Some(vel) = velocities.get_mut(entity) {
                        vel.0.z = impulse + physics.get(entity).map_or(0.0, |ps| ps.ground_vel.z);
                    }
                },
                LocalEvent::ApplyImpulse { entity, impulse } => {
                    if let Some(vel) = velocities.get_mut(entity) {
                        vel.0 = impulse;
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
                LocalEvent::CreateOutcome(outcome) => {
                    outcomes_emitter.emit(outcome);
                },
            }
        }
        drop(guard);
    }

    pub fn maintain_ecs(&mut self) {
        span!(_guard, "maintain ecs");
        self.ecs.maintain();
    }

    /// Clean up the state after a tick.
    pub fn cleanup(&mut self) {
        span!(_guard, "cleanup", "State::cleanup");
        // Clean up data structures from the last tick.
        self.ecs.write_resource::<TerrainChanges>().clear();
    }
}

// Timing code for server metrics
#[derive(Default)]
pub struct StateTickMetrics {
    pub timings: Vec<(&'static str, Duration)>,
}

impl StateTickMetrics {
    fn add(&mut self, label: &'static str, dur: Duration) {
        // Check for duplicates!
        debug_assert!(
            self.timings.iter().all(|(l, _)| *l != label),
            "Duplicate label in state tick metrics {label}"
        );
        self.timings.push((label, dur));
    }
}

struct MetricsGuard<'a> {
    start: Instant,
    label: &'static str,
    metrics: &'a mut StateTickMetrics,
}

impl<'a> MetricsGuard<'a> {
    fn new(label: &'static str, metrics: &'a mut StateTickMetrics) -> Self {
        Self {
            start: Instant::now(),
            label,
            metrics,
        }
    }
}

impl Drop for MetricsGuard<'_> {
    fn drop(&mut self) { self.metrics.add(self.label, self.start.elapsed()); }
}
