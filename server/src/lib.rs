#![deny(unsafe_code)]
#![allow(clippy::option_map_unit_fn)]
#![deny(clippy::clone_on_ref_ptr)]
#![feature(bool_to_option, drain_filter, option_zip)]
#![cfg_attr(not(feature = "worldgen"), feature(const_panic))]

pub mod alias_validator;
mod character_creator;
pub mod chunk_generator;
pub mod client;
pub mod cmd;
pub mod connection_handler;
mod data_dir;
pub mod error;
pub mod events;
pub mod input;
pub mod login_provider;
pub mod metrics;
pub mod persistence;
pub mod presence;
pub mod settings;
pub mod state_ext;
pub mod sys;
#[cfg(not(feature = "worldgen"))] mod test_world;

// Reexports
pub use crate::{
    data_dir::DEFAULT_DATA_DIR_NAME,
    error::Error,
    events::Event,
    input::Input,
    settings::{EditableSettings, Settings},
};

use crate::{
    alias_validator::AliasValidator,
    chunk_generator::ChunkGenerator,
    client::Client,
    cmd::ChatCommandExt,
    connection_handler::ConnectionHandler,
    data_dir::DataDir,
    login_provider::LoginProvider,
    presence::{Presence, RegionSubscription},
    state_ext::StateExt,
    sys::sentinel::{DeletedEntities, TrackedComps},
};
use common::{
    assets::Asset,
    cmd::ChatCommand,
    comp::{self, ChatType},
    event::{EventBus, ServerEvent},
    msg::{
        ClientType, DisconnectReason, ServerGeneral, ServerInfo, ServerInit, ServerMsg, WorldMapMsg,
    },
    outcome::Outcome,
    recipe::default_recipe_book,
    spiral::Spiral2d,
    state::{State, TimeOfDay},
    sync::WorldSyncExt,
    terrain::TerrainChunkSize,
    vol::{ReadVol, RectVolSize},
};
use futures_executor::block_on;
use metrics::{PhysicsMetrics, ServerMetrics, StateTickMetrics, TickMetrics};
use network::{Network, Pid, ProtocolAddr};
use persistence::{
    character_loader::{CharacterLoader, CharacterLoaderResponseKind},
    character_updater::CharacterUpdater,
};
use specs::{join::Join, Builder, Entity as EcsEntity, RunNow, SystemData, WorldExt};
use std::{
    i32,
    ops::{Deref, DerefMut},
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};
#[cfg(not(feature = "worldgen"))]
use test_world::{IndexOwned, World};
use tracing::{debug, error, info, trace};
use uvth::{ThreadPool, ThreadPoolBuilder};
use vek::*;
#[cfg(feature = "worldgen")]
use world::{
    civ::SiteKind,
    sim::{FileOpts, WorldOpts, DEFAULT_WORLD_MAP},
    IndexOwned, World,
};

#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_migrations;

#[derive(Copy, Clone)]
struct SpawnPoint(Vec3<f32>);

// Tick count used for throttling network updates
// Note this doesn't account for dt (so update rate changes with tick rate)
#[derive(Copy, Clone, Default)]
pub struct Tick(u64);

pub struct Server {
    state: State,
    world: Arc<World>,
    index: IndexOwned,
    map: WorldMapMsg,

    connection_handler: ConnectionHandler,

    thread_pool: ThreadPool,

    metrics: ServerMetrics,
    tick_metrics: TickMetrics,
    state_tick_metrics: StateTickMetrics,
    physics_metrics: PhysicsMetrics,
}

impl Server {
    /// Create a new `Server`
    #[allow(clippy::expect_fun_call)] // TODO: Pending review in #587
    #[allow(clippy::needless_update)] // TODO: Pending review in #587
    pub fn new(
        settings: Settings,
        editable_settings: EditableSettings,
        data_dir: &std::path::Path,
    ) -> Result<Self, Error> {
        info!("Server is data dir is: {}", data_dir.display());
        if settings.auth_server_address.is_none() {
            info!("Authentication is disabled");
        }

        // Relative to data_dir
        const PERSISTENCE_DB_DIR: &str = "saves";
        let persistence_db_dir = data_dir.join(PERSISTENCE_DB_DIR);

        // Run pending DB migrations (if any)
        debug!("Running DB migrations...");
        if let Some(e) = persistence::run_migrations(&persistence_db_dir).err() {
            panic!("Migration error: {:?}", e);
        }

        let (chunk_gen_metrics, registry_chunk) = metrics::ChunkGenMetrics::new().unwrap();
        let (network_request_metrics, registry_network) =
            metrics::NetworkRequestMetrics::new().unwrap();
        let (player_metrics, registry_player) = metrics::PlayerMetrics::new().unwrap();

        let mut state = State::default();
        state.ecs_mut().insert(settings.clone());
        state.ecs_mut().insert(editable_settings);
        state.ecs_mut().insert(DataDir {
            path: data_dir.to_owned(),
        });
        state.ecs_mut().insert(EventBus::<ServerEvent>::default());
        state
            .ecs_mut()
            .insert(LoginProvider::new(settings.auth_server_address.clone()));
        state.ecs_mut().insert(Tick(0));
        state.ecs_mut().insert(network_request_metrics);
        state.ecs_mut().insert(player_metrics);
        state
            .ecs_mut()
            .insert(ChunkGenerator::new(chunk_gen_metrics));
        state
            .ecs_mut()
            .insert(CharacterUpdater::new(&persistence_db_dir)?);

        let ability_map = comp::item::tool::AbilityMap::load_expect_cloned(
            "common.abilities.weapon_ability_manifest",
        );
        state
            .ecs_mut()
            .insert(CharacterLoader::new(&persistence_db_dir, &ability_map)?);
        state.ecs_mut().insert(ability_map);
        state.ecs_mut().insert(Vec::<Outcome>::new());

        // System timers for performance monitoring
        state.ecs_mut().insert(sys::EntitySyncTimer::default());
        state.ecs_mut().insert(sys::GeneralMsgTimer::default());
        state.ecs_mut().insert(sys::PingMsgTimer::default());
        state.ecs_mut().insert(sys::RegisterMsgTimer::default());
        state
            .ecs_mut()
            .insert(sys::CharacterScreenMsgTimer::default());
        state.ecs_mut().insert(sys::InGameMsgTimer::default());
        state.ecs_mut().insert(sys::SentinelTimer::default());
        state.ecs_mut().insert(sys::SubscriptionTimer::default());
        state.ecs_mut().insert(sys::TerrainSyncTimer::default());
        state.ecs_mut().insert(sys::TerrainTimer::default());
        state.ecs_mut().insert(sys::WaypointTimer::default());
        state.ecs_mut().insert(sys::InviteTimeoutTimer::default());
        state.ecs_mut().insert(sys::PersistenceTimer::default());

        // System schedulers to control execution of systems
        state
            .ecs_mut()
            .insert(sys::PersistenceScheduler::every(Duration::from_secs(10)));

        // Server-only components
        state.ecs_mut().register::<RegionSubscription>();
        state.ecs_mut().register::<Client>();
        state.ecs_mut().register::<Presence>();

        //Alias validator
        let banned_words_paths = &settings.banned_words_files;
        let mut banned_words = Vec::new();
        for path in banned_words_paths {
            let mut list = match std::fs::File::open(&path) {
                Ok(file) => match ron::de::from_reader(&file) {
                    Ok(vec) => vec,
                    Err(error) => {
                        tracing::warn!(?error, ?file, "Couldn't deserialize banned words file");
                        return Err(Error::Other(format!(
                            "Couldn't read banned words file \"{}\"",
                            path.to_string_lossy()
                        )));
                    },
                },
                Err(error) => {
                    tracing::warn!(?error, ?path, "Couldn't open banned words file");
                    return Err(Error::Other(format!(
                        "Couldn't open banned words file \"{}\". Error: {}",
                        path.to_string_lossy(),
                        error
                    )));
                },
            };
            banned_words.append(&mut list);
        }
        let banned_words_count = banned_words.len();
        tracing::debug!(?banned_words_count);
        tracing::trace!(?banned_words);
        state.ecs_mut().insert(AliasValidator::new(banned_words));

        #[cfg(feature = "worldgen")]
        let (world, index) = World::generate(settings.world_seed, WorldOpts {
            seed_elements: true,
            world_file: if let Some(ref opts) = settings.map_file {
                opts.clone()
            } else {
                // Load default map from assets.
                FileOpts::LoadAsset(DEFAULT_WORLD_MAP.into())
            },
            ..WorldOpts::default()
        });
        #[cfg(feature = "worldgen")]
        let map = world.get_map_data(index.as_index_ref());

        #[cfg(not(feature = "worldgen"))]
        let (world, index) = World::generate(settings.world_seed);
        #[cfg(not(feature = "worldgen"))]
        let map = WorldMapMsg {
            dimensions_lg: Vec2::zero(),
            max_height: 1.0,
            rgba: vec![0],
            horizons: [(vec![0], vec![0]), (vec![0], vec![0])],
            sea_level: 0.0,
            alt: vec![30],
        };

        #[cfg(feature = "worldgen")]
        let spawn_point = {
            let index = index.as_index_ref();
            // NOTE: all of these `.map(|e| e as [type])` calls should compile into no-ops,
            // but are needed to be explicit about casting (and to make the compiler stop
            // complaining)

            // spawn in the chunk, that is in the middle of the world
            let center_chunk: Vec2<i32> = world.sim().map_size_lg().chunks().map(i32::from) / 2;

            // Find a town to spawn in that's close to the centre of the world
            let spawn_chunk = world
                .civs()
                .sites()
                .filter(|site| matches!(site.kind, SiteKind::Settlement))
                .map(|site| site.center)
                .min_by_key(|site_pos| site_pos.distance_squared(center_chunk))
                .unwrap_or(center_chunk);

            // calculate the absolute position of the chunk in the world
            // (we could add TerrainChunkSize::RECT_SIZE / 2 here, to spawn in the middle of
            // the chunk)
            let spawn_location = spawn_chunk.map2(TerrainChunkSize::RECT_SIZE, |e, sz| {
                e as i32 * sz as i32 + sz as i32 / 2
            });

            // get a z cache for the column in which we want to spawn
            let mut block_sampler = world.sample_blocks();
            let z_cache = block_sampler
                .get_z_cache(spawn_location, index)
                .expect(&format!("no z_cache found for chunk: {}", spawn_chunk));

            // get the minimum and maximum z values at which there could be solid blocks
            let (min_z, max_z) = z_cache.get_z_limits();
            // round range outwards, so no potential air block is missed
            let min_z = min_z.floor() as i32;
            let max_z = max_z.ceil() as i32;

            // loop over all blocks from min_z to max_z + 1
            // until the first air block is found
            // (up to max_z + 1, because max_z could still be a solid block)
            // if no air block is found default to max_z + 1
            let z = (min_z..(max_z + 1) + 1)
                .find(|z| {
                    block_sampler
                        .get_with_z_cache(
                            Vec3::new(spawn_location.x, spawn_location.y, *z),
                            Some(&z_cache),
                        )
                        .map(|b| b.is_air())
                        .unwrap_or(false)
                })
                .unwrap_or(max_z + 1);

            // build the actual spawn point and
            // add 0.5, so that the player spawns in the middle of the block
            Vec3::new(spawn_location.x, spawn_location.y, z).map(|e| (e as f32)) + 0.5
        };

        #[cfg(not(feature = "worldgen"))]
        let spawn_point = Vec3::new(0.0, 0.0, 256.0);

        // set the spawn point we calculated above
        state.ecs_mut().insert(SpawnPoint(spawn_point));

        // Set starting time for the server.
        state.ecs_mut().write_resource::<TimeOfDay>().0 = settings.start_time;

        // Register trackers
        sys::sentinel::register_trackers(&mut state.ecs_mut());

        state.ecs_mut().insert(DeletedEntities::default());

        let mut metrics = ServerMetrics::new();
        // register all metrics submodules here
        let (tick_metrics, registry_tick) = TickMetrics::new(metrics.tick_clone())
            .expect("Failed to initialize server tick metrics submodule.");
        let (state_tick_metrics, registry_state) = StateTickMetrics::new().unwrap();
        let (physics_metrics, registry_physics) = PhysicsMetrics::new().unwrap();

        registry_chunk(&metrics.registry()).expect("failed to register chunk gen metrics");
        registry_network(&metrics.registry()).expect("failed to register network request metrics");
        registry_player(&metrics.registry()).expect("failed to register player metrics");
        registry_tick(&metrics.registry()).expect("failed to register tick metrics");
        registry_state(&metrics.registry()).expect("failed to register state metrics");
        registry_physics(&metrics.registry()).expect("failed to register state metrics");

        let thread_pool = ThreadPoolBuilder::new()
            .name("veloren-worker".to_string())
            .build();
        let (network, f) = Network::new_with_registry(Pid::new(), &metrics.registry());
        metrics
            .run(settings.metrics_address)
            .expect("Failed to initialize server metrics submodule.");
        thread_pool.execute(f);
        block_on(network.listen(ProtocolAddr::Tcp(settings.gameserver_address)))?;
        let connection_handler = ConnectionHandler::new(network);

        let this = Self {
            state,
            world: Arc::new(world),
            index,
            map,

            connection_handler,

            thread_pool,

            metrics,
            tick_metrics,
            state_tick_metrics,
            physics_metrics,
        };

        debug!(?settings, "created veloren server with");

        let git_hash = *common::util::GIT_HASH;
        let git_date = common::util::GIT_DATE.clone();
        let git_time = *common::util::GIT_TIME;
        let version = common::util::DISPLAY_VERSION_LONG.clone();
        info!(?version, "Server version");
        debug!(?git_hash, ?git_date, ?git_time, "detailed Server version");

        Ok(this)
    }

    pub fn get_server_info(&self) -> ServerInfo {
        let settings = self.state.ecs().fetch::<Settings>();
        let editable_settings = self.state.ecs().fetch::<EditableSettings>();
        ServerInfo {
            name: settings.server_name.clone(),
            description: (&*editable_settings.server_description).clone(),
            git_hash: common::util::GIT_HASH.to_string(),
            git_date: common::util::GIT_DATE.to_string(),
            auth_provider: settings.auth_server_address.clone(),
        }
    }

    pub fn with_thread_pool(mut self, thread_pool: ThreadPool) -> Self {
        self.thread_pool = thread_pool;
        self
    }

    /// Get a reference to the server's settings
    pub fn settings(&self) -> impl Deref<Target = Settings> + '_ {
        self.state.ecs().fetch::<Settings>()
    }

    /// Get a mutable reference to the server's settings
    pub fn settings_mut(&self) -> impl DerefMut<Target = Settings> + '_ {
        self.state.ecs().fetch_mut::<Settings>()
    }

    /// Get a mutable reference to the server's editable settings
    pub fn editable_settings_mut(&self) -> impl DerefMut<Target = EditableSettings> + '_ {
        self.state.ecs().fetch_mut::<EditableSettings>()
    }

    /// Get a reference to the server's editable settings
    pub fn editable_settings(&self) -> impl Deref<Target = EditableSettings> + '_ {
        self.state.ecs().fetch::<EditableSettings>()
    }

    /// Get path to the directory that the server info into
    pub fn data_dir(&self) -> impl Deref<Target = DataDir> + '_ {
        self.state.ecs().fetch::<DataDir>()
    }

    /// Get a reference to the server's game state.
    pub fn state(&self) -> &State { &self.state }

    /// Get a mutable reference to the server's game state.
    pub fn state_mut(&mut self) -> &mut State { &mut self.state }

    /// Get a reference to the server's world.
    pub fn world(&self) -> &World { &self.world }

    /// Get a reference to the server's world map.
    pub fn map(&self) -> &WorldMapMsg { &self.map }

    /// Execute a single server tick, handle input and update the game state by
    /// the given duration.
    pub fn tick(&mut self, _input: Input, dt: Duration) -> Result<Vec<Event>, Error> {
        self.state.ecs().write_resource::<Tick>().0 += 1;
        // This tick function is the centre of the Veloren universe. Most server-side
        // things are managed from here, and as such it's important that it
        // stays organised. Please consult the core developers before making
        // significant changes to this code. Here is the approximate order of
        // things. Please update it as this code changes.
        //
        // 1) Collect input from the frontend, apply input effects to the
        //    state of the game
        // 2) Go through any events (timer-driven or otherwise) that need handling
        //    and apply them to the state of the game
        // 3) Go through all incoming client network communications, apply them to
        //    the game state
        // 4) Perform a single LocalState tick (i.e: update the world and entities
        //    in the world)
        // 5) Go through the terrain update queue and apply all changes to
        //    the terrain
        // 6) Send relevant state updates to all clients
        // 7) Check for persistence updates related to character data, and message the
        //    relevant entities
        // 8) Update Metrics with current data
        // 9) Finish the tick, passing control of the main thread back
        //    to the frontend

        // 1) Build up a list of events for this frame, to be passed to the frontend.
        let mut frontend_events = Vec::new();

        // 2)

        let before_new_connections = Instant::now();

        // 3) Handle inputs from clients
        self.handle_new_connections(&mut frontend_events);

        let before_message_system = Instant::now();

        // Run message receiving sys before the systems in common for decreased latency
        // (e.g. run before controller system)
        //TODO: run in parallel
        sys::msg::general::Sys.run_now(&self.state.ecs());
        sys::msg::register::Sys.run_now(&self.state.ecs());
        sys::msg::character_screen::Sys.run_now(&self.state.ecs());
        sys::msg::in_game::Sys.run_now(&self.state.ecs());
        sys::msg::ping::Sys.run_now(&self.state.ecs());

        let before_state_tick = Instant::now();

        // 4) Tick the server's LocalState.
        // 5) Fetch any generated `TerrainChunk`s and insert them into the terrain.
        // in sys/terrain.rs
        self.state.tick(dt, sys::add_server_systems, false);

        let before_handle_events = Instant::now();

        // Handle game events
        frontend_events.append(&mut self.handle_events());

        let before_update_terrain_and_regions = Instant::now();

        // Apply terrain changes and update the region map after processing server
        // events so that changes made by server events will be immediately
        // visible to client synchronization systems, minimizing the latency of
        // `ServerEvent` mediated effects
        self.state.update_region_map();
        self.state.apply_terrain_changes();

        let before_sync = Instant::now();

        // 6) Synchronise clients with the new state of the world.
        sys::run_sync_systems(self.state.ecs_mut());

        let before_world_tick = Instant::now();

        // Tick the world
        self.world.tick(dt);

        let before_entity_cleanup = Instant::now();

        // Remove NPCs that are outside the view distances of all players
        // This is done by removing NPCs in unloaded chunks
        let to_delete = {
            let terrain = self.state.terrain();
            (
                &self.state.ecs().entities(),
                &self.state.ecs().read_storage::<comp::Pos>(),
                !&self.state.ecs().read_storage::<comp::Player>(),
            )
                .join()
                .filter(|(_, pos, _)| {
                    let chunk_key = terrain.pos_key(pos.0.map(|e| e.floor() as i32));
                    // Check not only this chunk, but also all neighbours to avoid immediate
                    // despawning if the entity walks outside of a valid chunk
                    // briefly. If the entity isn't even near a loaded chunk then we get
                    // rid of it.
                    Spiral2d::new().all(|offs| terrain.get_key(chunk_key + offs).is_none())
                })
                .map(|(entity, _, _)| entity)
                .collect::<Vec<_>>()
        };

        for entity in to_delete {
            if let Err(e) = self.state.delete_entity_recorded(entity) {
                error!(?e, "Failed to delete agent outside the terrain");
            }
        }

        // 7 Persistence updates
        let before_persistence_updates = Instant::now();

        // Get character-related database responses and notify the requesting client
        self.state
            .ecs()
            .read_resource::<persistence::character_loader::CharacterLoader>()
            .messages()
            .for_each(|query_result| match query_result.result {
                CharacterLoaderResponseKind::CharacterList(result) => match result {
                    Ok(character_list_data) => self.notify_client(
                        query_result.entity,
                        ServerGeneral::CharacterListUpdate(character_list_data),
                    ),
                    Err(error) => self.notify_client(
                        query_result.entity,
                        ServerGeneral::CharacterActionError(error.to_string()),
                    ),
                },
                CharacterLoaderResponseKind::CharacterCreation(result) => match result {
                    Ok((character_id, list)) => {
                        self.notify_client(
                            query_result.entity,
                            ServerGeneral::CharacterListUpdate(list),
                        );
                        self.notify_client(
                            query_result.entity,
                            ServerGeneral::CharacterCreated(character_id),
                        );
                    },
                    Err(error) => self.notify_client(
                        query_result.entity,
                        ServerGeneral::CharacterActionError(error.to_string()),
                    ),
                },
                CharacterLoaderResponseKind::CharacterData(result) => {
                    let message = match *result {
                        Ok(character_data) => ServerEvent::UpdateCharacterData {
                            entity: query_result.entity,
                            components: character_data,
                        },
                        Err(error) => {
                            // We failed to load data for the character from the DB. Notify the
                            // client to push the state back to character selection, with the error
                            // to display
                            self.notify_client(
                                query_result.entity,
                                ServerGeneral::CharacterDataLoadError(error.to_string()),
                            );

                            // Clean up the entity data on the server
                            ServerEvent::ExitIngame {
                                entity: query_result.entity,
                            }
                        },
                    };

                    self.state
                        .ecs()
                        .read_resource::<EventBus<ServerEvent>>()
                        .emit_now(message);
                },
            });

        {
            // Check for new chunks; cancel and regenerate all chunks if the asset has been
            // reloaded. Note that all of these assignments are no-ops, so the
            // only work we do here on the fast path is perform a relaxed read on an atomic.
            // boolean.
            let index = &mut self.index;
            let thread_pool = &mut self.thread_pool;
            let world = &mut self.world;
            let ecs = self.state.ecs_mut();

            index.reload_colors_if_changed(|index| {
                let mut chunk_generator = ecs.write_resource::<ChunkGenerator>();
                let client = ecs.read_storage::<Client>();
                let mut terrain = ecs.write_resource::<common::terrain::TerrainGrid>();

                // Cancel all pending chunks.
                chunk_generator.cancel_all();

                if client.is_empty() {
                    // No clients, so just clear all terrain.
                    terrain.clear();
                } else {
                    // There's at least one client, so regenerate all chunks.
                    terrain.iter().for_each(|(pos, _)| {
                        chunk_generator.generate_chunk(
                            None,
                            pos,
                            thread_pool,
                            Arc::clone(&world),
                            index.clone(),
                        );
                    });
                }
            });
        }

        let end_of_server_tick = Instant::now();

        // 8) Update Metrics
        // Get system timing info
        let entity_sync_nanos = self
            .state
            .ecs()
            .read_resource::<sys::EntitySyncTimer>()
            .nanos as i64;
        let message_nanos = {
            let state = self.state.ecs();
            (state.read_resource::<sys::GeneralMsgTimer>().nanos
                + state.read_resource::<sys::PingMsgTimer>().nanos
                + state.read_resource::<sys::RegisterMsgTimer>().nanos
                + state.read_resource::<sys::CharacterScreenMsgTimer>().nanos
                + state.read_resource::<sys::InGameMsgTimer>().nanos) as i64
        };
        let sentinel_nanos = self.state.ecs().read_resource::<sys::SentinelTimer>().nanos as i64;
        let subscription_nanos = self
            .state
            .ecs()
            .read_resource::<sys::SubscriptionTimer>()
            .nanos as i64;
        let terrain_sync_nanos = self
            .state
            .ecs()
            .read_resource::<sys::TerrainSyncTimer>()
            .nanos as i64;
        let terrain_nanos = self.state.ecs().read_resource::<sys::TerrainTimer>().nanos as i64;
        let waypoint_nanos = self.state.ecs().read_resource::<sys::WaypointTimer>().nanos as i64;
        let invite_timeout_nanos = self
            .state
            .ecs()
            .read_resource::<sys::InviteTimeoutTimer>()
            .nanos as i64;
        let stats_persistence_nanos = self
            .state
            .ecs()
            .read_resource::<sys::PersistenceTimer>()
            .nanos as i64;
        let total_sys_ran_in_dispatcher_nanos =
            terrain_nanos + waypoint_nanos + invite_timeout_nanos;

        // Report timing info
        self.tick_metrics
            .tick_time
            .with_label_values(&["new connections"])
            .set((before_message_system - before_new_connections).as_nanos() as i64);
        self.tick_metrics
            .tick_time
            .with_label_values(&["state tick"])
            .set(
                (before_handle_events - before_state_tick).as_nanos() as i64
                    - total_sys_ran_in_dispatcher_nanos,
            );
        self.tick_metrics
            .tick_time
            .with_label_values(&["handle server events"])
            .set((before_update_terrain_and_regions - before_handle_events).as_nanos() as i64);
        self.tick_metrics
            .tick_time
            .with_label_values(&["update terrain and region map"])
            .set((before_sync - before_update_terrain_and_regions).as_nanos() as i64);
        self.tick_metrics
            .tick_time
            .with_label_values(&["world tick"])
            .set((before_entity_cleanup - before_world_tick).as_nanos() as i64);
        self.tick_metrics
            .tick_time
            .with_label_values(&["entity cleanup"])
            .set((before_persistence_updates - before_entity_cleanup).as_nanos() as i64);
        self.tick_metrics
            .tick_time
            .with_label_values(&["persistence_updates"])
            .set((end_of_server_tick - before_persistence_updates).as_nanos() as i64);
        self.tick_metrics
            .tick_time
            .with_label_values(&["entity sync"])
            .set(entity_sync_nanos);
        self.tick_metrics
            .tick_time
            .with_label_values(&["message"])
            .set(message_nanos);
        self.tick_metrics
            .tick_time
            .with_label_values(&["sentinel"])
            .set(sentinel_nanos);
        self.tick_metrics
            .tick_time
            .with_label_values(&["subscription"])
            .set(subscription_nanos);
        self.tick_metrics
            .tick_time
            .with_label_values(&["terrain sync"])
            .set(terrain_sync_nanos);
        self.tick_metrics
            .tick_time
            .with_label_values(&["terrain"])
            .set(terrain_nanos);
        self.tick_metrics
            .tick_time
            .with_label_values(&["waypoint"])
            .set(waypoint_nanos);
        self.tick_metrics
            .tick_time
            .with_label_values(&["invite timeout"])
            .set(invite_timeout_nanos);
        self.tick_metrics
            .tick_time
            .with_label_values(&["persistence:stats"])
            .set(stats_persistence_nanos);

        //detailed state metrics
        {
            let res = self
                .state
                .ecs()
                .read_resource::<common::metrics::SysMetrics>();
            let c = &self.state_tick_metrics.state_tick_time_count;
            let agent_ns = res.agent_ns.load(Ordering::Relaxed);
            let mount_ns = res.mount_ns.load(Ordering::Relaxed);
            let controller_ns = res.controller_ns.load(Ordering::Relaxed);
            let character_behavior_ns = res.character_behavior_ns.load(Ordering::Relaxed);
            let stats_ns = res.stats_ns.load(Ordering::Relaxed);
            let phys_ns = res.phys_ns.load(Ordering::Relaxed);
            let projectile_ns = res.projectile_ns.load(Ordering::Relaxed);
            let melee_ns = res.melee_ns.load(Ordering::Relaxed);

            c.with_label_values(&[common::sys::AGENT_SYS])
                .inc_by(agent_ns);
            c.with_label_values(&[common::sys::MOUNT_SYS])
                .inc_by(mount_ns);
            c.with_label_values(&[common::sys::CONTROLLER_SYS])
                .inc_by(controller_ns);
            c.with_label_values(&[common::sys::CHARACTER_BEHAVIOR_SYS])
                .inc_by(character_behavior_ns);
            c.with_label_values(&[common::sys::STATS_SYS])
                .inc_by(stats_ns);
            c.with_label_values(&[common::sys::PHYS_SYS])
                .inc_by(phys_ns);
            c.with_label_values(&[common::sys::PROJECTILE_SYS])
                .inc_by(projectile_ns);
            c.with_label_values(&[common::sys::MELEE_SYS])
                .inc_by(melee_ns);

            const NANOSEC_PER_SEC: f64 = Duration::from_secs(1).as_nanos() as f64;
            let h = &self.state_tick_metrics.state_tick_time_hist;
            h.with_label_values(&[common::sys::AGENT_SYS])
                .observe(agent_ns as f64 / NANOSEC_PER_SEC);
            h.with_label_values(&[common::sys::MOUNT_SYS])
                .observe(mount_ns as f64 / NANOSEC_PER_SEC);
            h.with_label_values(&[common::sys::CONTROLLER_SYS])
                .observe(controller_ns as f64 / NANOSEC_PER_SEC);
            h.with_label_values(&[common::sys::CHARACTER_BEHAVIOR_SYS])
                .observe(character_behavior_ns as f64 / NANOSEC_PER_SEC);
            h.with_label_values(&[common::sys::STATS_SYS])
                .observe(stats_ns as f64 / NANOSEC_PER_SEC);
            h.with_label_values(&[common::sys::PHYS_SYS])
                .observe(phys_ns as f64 / NANOSEC_PER_SEC);
            h.with_label_values(&[common::sys::PROJECTILE_SYS])
                .observe(projectile_ns as f64 / NANOSEC_PER_SEC);
            h.with_label_values(&[common::sys::MELEE_SYS])
                .observe(melee_ns as f64 / NANOSEC_PER_SEC);
        }

        //detailed physics metrics
        {
            let res = self
                .state
                .ecs()
                .read_resource::<common::metrics::PhysicsMetrics>();

            self.physics_metrics
                .entity_entity_collision_checks_count
                .inc_by(res.entity_entity_collision_checks);
            self.physics_metrics
                .entity_entity_collisions_count
                .inc_by(res.entity_entity_collisions);
        }

        // Report other info
        self.tick_metrics
            .time_of_day
            .set(self.state.ecs().read_resource::<TimeOfDay>().0);
        if self.tick_metrics.is_100th_tick() {
            let mut chonk_cnt = 0;
            let mut group_cnt = 0;
            let chunk_cnt = self.state.terrain().iter().fold(0, |a, (_, c)| {
                chonk_cnt += 1;
                group_cnt += c.sub_chunk_groups();
                a + c.sub_chunks_len()
            });
            self.tick_metrics.chonks_count.set(chonk_cnt as i64);
            self.tick_metrics.chunks_count.set(chunk_cnt as i64);
            self.tick_metrics.chunk_groups_count.set(group_cnt as i64);

            let entity_count = self.state.ecs().entities().join().count();
            self.tick_metrics.entity_count.set(entity_count as i64);
        }
        //self.metrics.entity_count.set(self.state.);
        self.tick_metrics
            .tick_time
            .with_label_values(&["metrics"])
            .set(end_of_server_tick.elapsed().as_nanos() as i64);
        self.metrics.tick();

        // 9) Finish the tick, pass control back to the frontend.

        Ok(frontend_events)
    }

    /// Clean up the server after a tick.
    pub fn cleanup(&mut self) {
        // Cleanup the local state
        self.state.cleanup();
    }

    fn initialize_client(
        &mut self,
        client: crate::connection_handler::IncomingClient,
    ) -> Result<Option<specs::Entity>, Error> {
        if self.settings().max_players <= self.state.ecs().read_storage::<Client>().join().count() {
            trace!(
                ?client.participant,
                "to many players, wont allow participant to connect"
            );
            client.send(ServerInit::TooManyPlayers)?;
            return Ok(None);
        }

        let entity = self
            .state
            .ecs_mut()
            .create_entity_synced()
            .with(client)
            .build();
        self.state
            .ecs()
            .read_resource::<metrics::PlayerMetrics>()
            .clients_connected
            .inc();
        // Send client all the tracked components currently attached to its entity as
        // well as synced resources (currently only `TimeOfDay`)
        debug!("Starting initial sync with client.");
        self.state
            .ecs()
            .read_storage::<Client>()
            .get(entity)
            .unwrap()
            .send(ServerInit::GameSync {
                // Send client their entity
                entity_package: TrackedComps::fetch(&self.state.ecs())
                    .create_entity_package(entity, None, None, None),
                time_of_day: *self.state.ecs().read_resource(),
                max_group_size: self.settings().max_player_group_size,
                client_timeout: self.settings().client_timeout,
                world_map: self.map.clone(),
                recipe_book: (&*default_recipe_book()).clone(),
                ability_map: (&*self
                    .state
                    .ecs()
                    .read_resource::<comp::item::tool::AbilityMap>())
                    .clone(),
            })?;
        Ok(Some(entity))
    }

    /// Handle new client connections.
    fn handle_new_connections(&mut self, frontend_events: &mut Vec<Event>) {
        while let Ok(sender) = self.connection_handler.info_requester_receiver.try_recv() {
            // can fail, e.g. due to timeout or network prob.
            trace!("sending info to connection_handler");
            let _ = sender.send(crate::connection_handler::ServerInfoPacket {
                info: self.get_server_info(),
                time: self.state.get_time(),
            });
        }

        while let Ok(incoming) = self.connection_handler.client_receiver.try_recv() {
            match self.initialize_client(incoming) {
                Ok(None) => (),
                Ok(Some(entity)) => {
                    frontend_events.push(Event::ClientConnected { entity });
                    debug!("Done initial sync with client.");
                },
                Err(e) => {
                    debug!(?e, "failed initializing a new client");
                },
            }
        }
    }

    pub fn notify_client<S>(&self, entity: EcsEntity, msg: S)
    where
        S: Into<ServerMsg>,
    {
        self.state
            .ecs()
            .read_storage::<Client>()
            .get(entity)
            .map(|c| c.send(msg));
    }

    pub fn notify_players(&mut self, msg: ServerGeneral) { self.state.notify_players(msg); }

    pub fn generate_chunk(&mut self, entity: EcsEntity, key: Vec2<i32>) {
        self.state
            .ecs()
            .write_resource::<ChunkGenerator>()
            .generate_chunk(
                Some(entity),
                key,
                &mut self.thread_pool,
                Arc::clone(&self.world),
                self.index.clone(),
            );
    }

    fn process_chat_cmd(&mut self, entity: EcsEntity, cmd: String) {
        // Separate string into keyword and arguments.
        let sep = cmd.find(' ');
        let (kwd, args) = match sep {
            Some(i) => (cmd[..i].to_string(), cmd[(i + 1)..].to_string()),
            None => (cmd, "".to_string()),
        };

        // Find the command object and run its handler.
        if let Ok(command) = kwd.parse::<ChatCommand>() {
            command.execute(self, entity, args);
        } else {
            self.notify_client(
                entity,
                ChatType::CommandError.server_msg(format!(
                    "Unknown command '/{}'.\nType '/help' for available commands",
                    kwd
                )),
            );
        }
    }

    fn entity_is_admin(&self, entity: EcsEntity) -> bool {
        self.state
            .read_storage::<comp::Admin>()
            .get(entity)
            .is_some()
    }

    pub fn number_of_players(&self) -> i64 {
        self.state.ecs().read_storage::<Client>().join().count() as i64
    }

    // TODO: add Admin comp if ingame
    pub fn add_admin(&self, username: &str) {
        let mut editable_settings = self.editable_settings_mut();
        let login_provider = self.state.ecs().fetch::<LoginProvider>();
        let data_dir = self.data_dir();
        add_admin(
            username,
            &login_provider,
            &mut editable_settings,
            &data_dir.path,
        );
    }

    // TODO: remove Admin comp if ingame
    pub fn remove_admin(&self, username: &str) {
        let mut editable_settings = self.editable_settings_mut();
        let login_provider = self.state.ecs().fetch::<LoginProvider>();
        let data_dir = self.data_dir();
        remove_admin(
            username,
            &login_provider,
            &mut editable_settings,
            &data_dir.path,
        );
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.state
            .notify_players(ServerGeneral::Disconnect(DisconnectReason::Shutdown));
    }
}

pub fn add_admin(
    username: &str,
    login_provider: &LoginProvider,
    editable_settings: &mut EditableSettings,
    data_dir: &std::path::Path,
) {
    use crate::settings::EditableSetting;
    match login_provider.username_to_uuid(username) {
        Ok(uuid) => editable_settings.admins.edit(data_dir, |admins| {
            if admins.insert(uuid) {
                info!("Successfully added {} ({}) as an admin!", username, uuid);
            } else {
                info!("{} ({}) is already an admin!", username, uuid);
            }
        }),
        Err(err) => error!(
            ?err,
            "Could not find uuid for this name either the user does not exist or there was an \
             error communicating with the auth server."
        ),
    }
}

pub fn remove_admin(
    username: &str,
    login_provider: &LoginProvider,
    editable_settings: &mut EditableSettings,
    data_dir: &std::path::Path,
) {
    use crate::settings::EditableSetting;
    match login_provider.username_to_uuid(username) {
        Ok(uuid) => editable_settings.admins.edit(data_dir, |admins| {
            if admins.remove(&uuid) {
                info!(
                    "Successfully removed {} ({}) from the admins",
                    username, uuid
                );
            } else {
                info!("{} ({}) is not an admin!", username, uuid);
            }
        }),
        Err(err) => error!(
            ?err,
            "Could not find uuid for this name either the user does not exist or there was an \
             error communicating with the auth server."
        ),
    }
}
