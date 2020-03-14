#![deny(unsafe_code)]
#![feature(drain_filter)]

pub mod auth_provider;
pub mod chunk_generator;
pub mod client;
pub mod cmd;
pub mod error;
pub mod events;
pub mod input;
pub mod metrics;
pub mod settings;
pub mod sys;
#[cfg(not(feature = "worldgen"))] mod test_world;

// Reexports
pub use crate::{error::Error, events::Event, input::Input, settings::ServerSettings};

use crate::{
    auth_provider::AuthProvider,
    chunk_generator::ChunkGenerator,
    client::{Client, RegionSubscription},
    cmd::CHAT_COMMANDS,
    sys::sentinel::{DeletedEntities, TrackedComps},
};
use common::{
    assets, comp,
    effect::Effect,
    event::{EventBus, ServerEvent},
    msg::{ClientMsg, ClientState, ServerError, ServerInfo, ServerMsg},
    net::PostOffice,
    state::{State, TimeOfDay},
    sync::{Uid, WorldSyncExt},
    terrain::TerrainChunkSize,
    vol::{ReadVol, RectVolSize},
};
use log::{debug, error, warn};
use metrics::ServerMetrics;
use specs::{
    join::Join, world::EntityBuilder as EcsEntityBuilder, Builder, Entity as EcsEntity, RunNow,
    SystemData, WorldExt,
};
use std::{
    i32,
    sync::Arc,
    time::{Duration, Instant},
};
#[cfg(not(feature = "worldgen"))]
use test_world::{World, WORLD_SIZE};
use uvth::{ThreadPool, ThreadPoolBuilder};
use vek::*;
#[cfg(feature = "worldgen")]
use world::{
    sim::{FileOpts, WorldOpts, DEFAULT_WORLD_MAP, WORLD_SIZE},
    World,
};

const CLIENT_TIMEOUT: f64 = 20.0; // Seconds

#[derive(Copy, Clone)]
struct SpawnPoint(Vec3<f32>);

// Tick count used for throttling network updates
// Note this doesn't account for dt (so update rate changes with tick rate)
#[derive(Copy, Clone, Default)]
pub struct Tick(u64);

pub struct Server {
    state: State,
    world: Arc<World>,
    map: Vec<u32>,

    postoffice: PostOffice<ServerMsg, ClientMsg>,

    thread_pool: ThreadPool,

    server_info: ServerInfo,
    metrics: ServerMetrics,

    server_settings: ServerSettings,
}

impl Server {
    /// Create a new `Server`
    pub fn new(settings: ServerSettings) -> Result<Self, Error> {
        let mut state = State::default();
        state.ecs_mut().insert(EventBus::<ServerEvent>::default());
        // TODO: anything but this
        state.ecs_mut().insert(AuthProvider::new());
        state.ecs_mut().insert(Tick(0));
        state.ecs_mut().insert(ChunkGenerator::new());
        // System timers for performance monitoring
        state.ecs_mut().insert(sys::EntitySyncTimer::default());
        state.ecs_mut().insert(sys::MessageTimer::default());
        state.ecs_mut().insert(sys::SentinelTimer::default());
        state.ecs_mut().insert(sys::SubscriptionTimer::default());
        state.ecs_mut().insert(sys::TerrainSyncTimer::default());
        state.ecs_mut().insert(sys::TerrainTimer::default());
        // Server-only components
        state.ecs_mut().register::<RegionSubscription>();
        state.ecs_mut().register::<Client>();

        #[cfg(feature = "worldgen")]
        let world = World::generate(settings.world_seed, WorldOpts {
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
        let map = world.sim().get_map();

        #[cfg(not(feature = "worldgen"))]
        let world = World::generate(settings.world_seed);
        #[cfg(not(feature = "worldgen"))]
        let map = vec![0];

        #[cfg(feature = "worldgen")]
        let spawn_point = {
            // NOTE: all of these `.map(|e| e as [type])` calls should compile into no-ops,
            // but are needed to be explicit about casting (and to make the compiler stop
            // complaining)

            // spawn in the chunk, that is in the middle of the world
            let spawn_chunk: Vec2<i32> = WORLD_SIZE.map(|e| e as i32) / 2;
            // calculate the absolute position of the chunk in the world
            // (we could add TerrainChunkSize::RECT_SIZE / 2 here, to spawn in the midde of
            // the chunk)
            let spawn_location = spawn_chunk * TerrainChunkSize::RECT_SIZE.map(|e| e as i32);

            // get a z cache for the collumn in which we want to spawn
            let mut block_sampler = world.sample_blocks();
            let z_cache = block_sampler
                .get_z_cache(spawn_location)
                .expect(&format!("no z_cache found for chunk: {}", spawn_chunk));

            // get the minimum and maximum z values at which there could be soild blocks
            let (min_z, _, max_z) = z_cache.get_z_limits(&mut block_sampler);
            // round range outwards, so no potential air block is missed
            let min_z = min_z.floor() as i32;
            let max_z = max_z.ceil() as i32;

            // loop over all blocks from min_z to max_z + 1
            // until the first air block is found
            // (up to max_z + 1, because max_z could still be a soild block)
            // if no air block is found default to max_z + 1
            let z = (min_z..(max_z + 1) + 1)
                .find(|z| {
                    block_sampler
                        .get_with_z_cache(
                            Vec3::new(spawn_location.x, spawn_location.y, *z),
                            Some(&z_cache),
                            false,
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

        let this = Self {
            state,
            world: Arc::new(world),
            map,

            postoffice: PostOffice::bind(settings.gameserver_address)?,

            thread_pool: ThreadPoolBuilder::new()
                .name("veloren-worker".into())
                .build(),

            server_info: ServerInfo {
                name: settings.server_name.clone(),
                description: settings.server_description.clone(),
                git_hash: common::util::GIT_HASH.to_string(),
                git_date: common::util::GIT_DATE.to_string(),
            },
            metrics: ServerMetrics::new(settings.metrics_address)
                .expect("Failed to initialize server metrics submodule."),
            server_settings: settings.clone(),
        };
        debug!("created veloren server with: {:?}", &settings);

        Ok(this)
    }

    pub fn with_thread_pool(mut self, thread_pool: ThreadPool) -> Self {
        self.thread_pool = thread_pool;
        self
    }

    /// Get a reference to the server's game state.
    pub fn state(&self) -> &State { &self.state }

    /// Get a mutable reference to the server's game state.
    pub fn state_mut(&mut self) -> &mut State { &mut self.state }

    /// Get a reference to the server's world.
    pub fn world(&self) -> &World { &self.world }

    /// Build a static object entity
    pub fn create_object(
        &mut self,
        pos: comp::Pos,
        object: comp::object::Body,
    ) -> EcsEntityBuilder {
        self.state
            .ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(comp::Vel(Vec3::zero()))
            .with(comp::Ori(Vec3::unit_y()))
            .with(comp::Body::Object(object))
            .with(comp::Mass(100.0))
            .with(comp::Gravity(1.0))
        //.with(comp::LightEmitter::default())
    }

    /// Build a projectile
    pub fn create_projectile(
        state: &mut State,
        pos: comp::Pos,
        vel: comp::Vel,
        body: comp::Body,
        projectile: comp::Projectile,
    ) -> EcsEntityBuilder {
        state
            .ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(vel)
            .with(comp::Ori(vel.0.normalized()))
            .with(comp::Mass(0.0))
            .with(body)
            .with(projectile)
            .with(comp::Sticky)
    }

    pub fn create_player_character(
        state: &mut State,
        entity: EcsEntity,
        name: String,
        body: comp::Body,
        main: Option<String>,
        server_settings: &ServerSettings,
    ) {
        // Give no item when an invalid specifier is given
        let main = main.and_then(|specifier| assets::load_cloned(&specifier).ok());

        let spawn_point = state.ecs().read_resource::<SpawnPoint>().0;

        state.write_component(entity, body);
        state.write_component(entity, comp::Stats::new(name, body, main.clone()));
        state.write_component(entity, comp::Energy::new(1000));
        state.write_component(entity, comp::Controller::default());
        state.write_component(entity, comp::Pos(spawn_point));
        state.write_component(entity, comp::Vel(Vec3::zero()));
        state.write_component(entity, comp::Ori(Vec3::unit_y()));
        state.write_component(entity, comp::Gravity(1.0));
        state.write_component(entity, comp::CharacterState::default());
        state.write_component(entity, comp::Alignment::Owned(entity));
        state.write_component(entity, comp::Inventory::default());
        state.write_component(
            entity,
            comp::InventoryUpdate::new(comp::InventoryUpdateEvent::default()),
        );

        state.write_component(
            entity,
            if let Some(comp::ItemKind::Tool(tool)) = main.as_ref().map(|i| i.kind) {
                let mut abilities = tool.get_abilities();
                let mut ability_drain = abilities.drain(..);
                comp::Loadout {
                    active_item: main.map(|item| comp::ItemConfig {
                        item,
                        primary_ability: ability_drain.next(),
                        secondary_ability: ability_drain.next(),
                        block_ability: Some(comp::CharacterAbility::BasicBlock),
                        dodge_ability: Some(comp::CharacterAbility::Roll),
                    }),
                    second_item: None,
                }
            } else {
                comp::Loadout::default()
            },
        );

        // Make sure physics are accepted.
        state.write_component(entity, comp::ForceUpdate);

        // Give the Admin component to the player if their name exists in admin list
        if server_settings.admins.contains(
            &state
                .ecs()
                .read_storage::<comp::Player>()
                .get(entity)
                .expect("Failed to fetch entity.")
                .alias,
        ) {
            state.write_component(entity, comp::Admin);
        }
        // Tell the client its request was successful.
        if let Some(client) = state.ecs().write_storage::<Client>().get_mut(entity) {
            client.allow_state(ClientState::Character);
        }
    }

    /// Handle events coming through via the event bus

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
        // 7) Update Metrics with current data
        // 8) Finish the tick, passing control of the main thread back
        //    to the frontend

        let before_tick_1 = Instant::now();
        // 1) Build up a list of events for this frame, to be passed to the frontend.
        let mut frontend_events = Vec::new();

        // If networking has problems, handle them.
        if let Some(err) = self.postoffice.error() {
            return Err(err.into());
        }

        // 2)

        // 3) Handle inputs from clients
        frontend_events.append(&mut self.handle_new_connections()?);

        // Run message recieving sys before the systems in common for decreased latency
        // (e.g. run before controller system)
        sys::message::Sys.run_now(&self.state.ecs());

        let before_tick_4 = Instant::now();

        // 4) Tick the server's LocalState.
        self.state.tick(dt, sys::add_server_systems);

        let before_handle_events = Instant::now();
        // Handle game events
        frontend_events.append(&mut self.handle_events());

        // Tick the world
        self.world.tick(dt);

        // 5) Fetch any generated `TerrainChunk`s and insert them into the terrain.
        // in sys/terrain.rs

        let before_tick_6 = Instant::now();
        // 6) Synchronise clients with the new state of the world.

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
                .filter(|(_, pos, _)| terrain.get(pos.0.map(|e| e.floor() as i32)).is_err())
                .map(|(entity, _, _)| entity)
                .collect::<Vec<_>>()
        };
        for entity in to_delete {
            if let Err(err) = self.state.delete_entity_recorded(entity) {
                error!("Failed to delete agent outside the terrain: {:?}", err);
            }
        }

        let before_tick_7 = Instant::now();
        // 7) Update Metrics
        let entity_sync_nanos = self
            .state
            .ecs()
            .read_resource::<sys::EntitySyncTimer>()
            .nanos as i64;
        let message_nanos = self.state.ecs().read_resource::<sys::MessageTimer>().nanos as i64;
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
        let total_sys_nanos = entity_sync_nanos
            + message_nanos
            + sentinel_nanos
            + subscription_nanos
            + terrain_sync_nanos
            + terrain_nanos;
        self.metrics
            .tick_time
            .with_label_values(&["input"])
            .set((before_tick_4 - before_tick_1).as_nanos() as i64 - message_nanos);
        self.metrics
            .tick_time
            .with_label_values(&["state tick"])
            .set(
                (before_handle_events - before_tick_4).as_nanos() as i64
                    - (total_sys_nanos - message_nanos),
            );
        self.metrics
            .tick_time
            .with_label_values(&["handle server events"])
            .set((before_tick_6 - before_handle_events).as_nanos() as i64);
        self.metrics
            .tick_time
            .with_label_values(&["entity deletion"])
            .set((before_tick_7 - before_tick_6).as_nanos() as i64);
        self.metrics
            .tick_time
            .with_label_values(&["entity sync"])
            .set(entity_sync_nanos);
        self.metrics
            .tick_time
            .with_label_values(&["message"])
            .set(message_nanos);
        self.metrics
            .tick_time
            .with_label_values(&["subscription"])
            .set(subscription_nanos);
        self.metrics
            .tick_time
            .with_label_values(&["terrain sync"])
            .set(terrain_sync_nanos);
        self.metrics
            .tick_time
            .with_label_values(&["terrain"])
            .set(terrain_nanos);
        self.metrics
            .player_online
            .set(self.state.ecs().read_storage::<Client>().join().count() as i64);
        self.metrics
            .time_of_day
            .set(self.state.ecs().read_resource::<TimeOfDay>().0);
        if self.metrics.is_100th_tick() {
            let mut chonk_cnt = 0;
            let chunk_cnt = self.state.terrain().iter().fold(0, |a, (_, c)| {
                chonk_cnt += 1;
                a + c.sub_chunks_len()
            });
            self.metrics.chonks_count.set(chonk_cnt as i64);
            self.metrics.chunks_count.set(chunk_cnt as i64);
        }
        //self.metrics.entity_count.set(self.state.);
        self.metrics
            .tick_time
            .with_label_values(&["metrics"])
            .set(before_tick_7.elapsed().as_nanos() as i64);

        // 8) Finish the tick, pass control back to the frontend.

        Ok(frontend_events)
    }

    /// Clean up the server after a tick.
    pub fn cleanup(&mut self) {
        // Cleanup the local state
        self.state.cleanup();
    }

    /// Handle new client connections.
    fn handle_new_connections(&mut self) -> Result<Vec<Event>, Error> {
        let mut frontend_events = Vec::new();

        for postbox in self.postoffice.new_postboxes() {
            let mut client = Client {
                client_state: ClientState::Connected,
                postbox,
                last_ping: self.state.get_time(),
                login_msg_sent: false,
            };

            if self.server_settings.max_players
                <= self.state.ecs().read_storage::<Client>().join().count()
            {
                // Note: in this case the client is dropped
                client.notify(ServerMsg::Error(ServerError::TooManyPlayers));
            } else {
                let entity = self
                    .state
                    .ecs_mut()
                    .create_entity_synced()
                    .with(client)
                    .build();
                // Send client all the tracked components currently attached to its entity as
                // well as synced resources (currently only `TimeOfDay`)
                log::debug!("Starting initial sync with client.");
                self.state
                    .ecs()
                    .write_storage::<Client>()
                    .get_mut(entity)
                    .unwrap()
                    .notify(ServerMsg::InitialSync {
                        // Send client their entity
                        entity_package: TrackedComps::fetch(&self.state.ecs())
                            .create_entity_package(entity),
                        server_info: self.server_info.clone(),
                        time_of_day: *self.state.ecs().read_resource(),
                        world_map: (WORLD_SIZE.map(|e| e as u32), self.map.clone()),
                    });
                log::debug!("Done initial sync with client.");

                frontend_events.push(Event::ClientConnected { entity });
            }
        }

        Ok(frontend_events)
    }

    pub fn notify_client(&self, entity: EcsEntity, msg: ServerMsg) {
        if let Some(client) = self.state.ecs().write_storage::<Client>().get_mut(entity) {
            client.notify(msg)
        }
    }

    pub fn generate_chunk(&mut self, entity: EcsEntity, key: Vec2<i32>) {
        self.state
            .ecs()
            .write_resource::<ChunkGenerator>()
            .generate_chunk(entity, key, &mut self.thread_pool, self.world.clone());
    }

    fn process_chat_cmd(&mut self, entity: EcsEntity, cmd: String) {
        // Separate string into keyword and arguments.
        let sep = cmd.find(' ');
        let (kwd, args) = match sep {
            Some(i) => (cmd[..i].to_string(), cmd[(i + 1)..].to_string()),
            None => (cmd, "".to_string()),
        };

        // Find the command object and run its handler.
        let action_opt = CHAT_COMMANDS.iter().find(|x| x.keyword == kwd);
        match action_opt {
            Some(action) => action.execute(self, entity, args),
            // Unknown command
            None => {
                if let Some(client) = self.state.ecs().write_storage::<Client>().get_mut(entity) {
                    client.notify(ServerMsg::private(format!(
                        "Unknown command '/{}'.\nType '/help' for available commands",
                        kwd
                    )));
                }
            },
        }
    }

    fn entity_is_admin(&self, entity: EcsEntity) -> bool {
        self.state
            .read_storage::<comp::Admin>()
            .get(entity)
            .is_some()
    }

    pub fn number_of_players(&self) -> i64 { self.metrics.player_online.get() }
}

impl Drop for Server {
    fn drop(&mut self) { self.state.notify_registered_clients(ServerMsg::Shutdown); }
}

trait StateExt {
    fn give_item(&mut self, entity: EcsEntity, item: comp::Item) -> bool;
    fn apply_effect(&mut self, entity: EcsEntity, effect: Effect);
    fn notify_registered_clients(&self, msg: ServerMsg);
    fn create_npc(
        &mut self,
        pos: comp::Pos,
        stats: comp::Stats,
        body: comp::Body,
    ) -> EcsEntityBuilder;
    fn delete_entity_recorded(
        &mut self,
        entity: EcsEntity,
    ) -> Result<(), specs::error::WrongGeneration>;
}

impl StateExt for State {
    fn give_item(&mut self, entity: EcsEntity, item: comp::Item) -> bool {
        let success = self
            .ecs()
            .write_storage::<comp::Inventory>()
            .get_mut(entity)
            .map(|inv| inv.push(item).is_none())
            .unwrap_or(false);
        if success {
            self.write_component(
                entity,
                comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Collected),
            );
        }
        success
    }

    fn apply_effect(&mut self, entity: EcsEntity, effect: Effect) {
        match effect {
            Effect::Health(change) => {
                self.ecs()
                    .write_storage::<comp::Stats>()
                    .get_mut(entity)
                    .map(|stats| stats.health.change_by(change));
            },
            Effect::Xp(xp) => {
                self.ecs()
                    .write_storage::<comp::Stats>()
                    .get_mut(entity)
                    .map(|stats| stats.exp.change_by(xp));
            },
        }
    }

    /// Build a non-player character.
    fn create_npc(
        &mut self,
        pos: comp::Pos,
        stats: comp::Stats,
        body: comp::Body,
    ) -> EcsEntityBuilder {
        self.ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(comp::Vel(Vec3::zero()))
            .with(comp::Ori(Vec3::unit_y()))
            .with(comp::Controller::default())
            .with(body)
            .with(stats)
            .with(comp::Alignment::Npc)
            .with(comp::Energy::new(500))
            .with(comp::Gravity(1.0))
            .with(comp::CharacterState::default())
            .with(comp::Loadout::default()) // TODO Give the poor npc something to do
    }

    fn notify_registered_clients(&self, msg: ServerMsg) {
        for client in (&mut self.ecs().write_storage::<Client>())
            .join()
            .filter(|c| c.is_registered())
        {
            client.notify(msg.clone())
        }
    }

    fn delete_entity_recorded(
        &mut self,
        entity: EcsEntity,
    ) -> Result<(), specs::error::WrongGeneration> {
        let (maybe_uid, maybe_pos) = (
            self.ecs().read_storage::<Uid>().get(entity).copied(),
            self.ecs().read_storage::<comp::Pos>().get(entity).copied(),
        );
        let res = self.ecs_mut().delete_entity(entity);
        if res.is_ok() {
            if let (Some(uid), Some(pos)) = (maybe_uid, maybe_pos) {
                if let Some(region_key) = self
                    .ecs()
                    .read_resource::<common::region::RegionMap>()
                    .find_region(entity, pos.0)
                {
                    self.ecs()
                        .write_resource::<DeletedEntities>()
                        .record_deleted_entity(uid, region_key);
                } else {
                    // Don't panic if the entity wasn't found in a region maybe it was just created
                    // and then deleted before the region manager had a chance to assign it a
                    // region
                    warn!(
                        "Failed to find region containing entity during entity deletion, assuming \
                         it wasn't sent to any clients and so deletion doesn't need to be \
                         recorded for sync purposes"
                    );
                }
            }
        }
        res
    }
}
