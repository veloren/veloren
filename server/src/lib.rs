#![deny(unsafe_code)]
#![feature(drain_filter)]

pub mod auth_provider;
pub mod client;
pub mod cmd;
pub mod error;
pub mod input;
pub mod metrics;
pub mod settings;

// Reexports
pub use crate::{error::Error, input::Input, settings::ServerSettings};

use crate::{
    auth_provider::AuthProvider,
    client::{Client, Clients, RegionSubscription},
    cmd::CHAT_COMMANDS,
};
use common::{
    comp,
    effect::Effect,
    event::{EventBus, ServerEvent},
    msg::{validate_chat_msg, ChatMsgValidationError, MAX_BYTES_CHAT_MSG},
    msg::{ClientMsg, ClientState, RequestStateError, ServerError, ServerInfo, ServerMsg},
    net::PostOffice,
    state::{BlockChange, State, TimeOfDay, Uid},
    terrain::{block::Block, TerrainChunk, TerrainChunkSize, TerrainGrid},
    vol::{ReadVol, RectVolSize, Vox},
};
use crossbeam::channel;
use hashbrown::{hash_map::Entry, HashMap};
use log::{debug, trace};
use metrics::ServerMetrics;
use rand::Rng;
use specs::{join::Join, world::EntityBuilder as EcsEntityBuilder, Builder, Entity as EcsEntity};
use std::{
    i32,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use uvth::{ThreadPool, ThreadPoolBuilder};
use vek::*;
use world::{ChunkSupplement, World};

const CLIENT_TIMEOUT: f64 = 20.0; // Seconds

pub enum Event {
    ClientConnected {
        entity: EcsEntity,
    },
    ClientDisconnected {
        entity: EcsEntity,
    },
    Chat {
        entity: Option<EcsEntity>,
        msg: String,
    },
}

#[derive(Copy, Clone)]
struct SpawnPoint(Vec3<f32>);

pub struct Server {
    state: State,
    world: Arc<World>,

    postoffice: PostOffice<ServerMsg, ClientMsg>,
    clients: Clients,

    thread_pool: ThreadPool,
    chunk_tx: channel::Sender<(
        Vec2<i32>,
        Result<(TerrainChunk, ChunkSupplement), EcsEntity>,
    )>,
    chunk_rx: channel::Receiver<(
        Vec2<i32>,
        Result<(TerrainChunk, ChunkSupplement), EcsEntity>,
    )>,
    pending_chunks: HashMap<Vec2<i32>, Arc<AtomicBool>>,

    server_settings: ServerSettings,
    server_info: ServerInfo,
    metrics: ServerMetrics,

    // Tick count used for throttling network updates
    // Note this doesn't account for dt (so update rate changes with tick rate)
    tick: u64,

    // TODO: anything but this
    accounts: AuthProvider,
}

impl Server {
    /// Create a new `Server`
    pub fn new(settings: ServerSettings) -> Result<Self, Error> {
        let (chunk_tx, chunk_rx) = channel::unbounded();

        let mut state = State::default();
        state
            .ecs_mut()
            .add_resource(SpawnPoint(Vec3::new(16_384.0, 16_384.0, 600.0)));
        state
            .ecs_mut()
            .add_resource(EventBus::<ServerEvent>::default());
        state.ecs_mut().register::<RegionSubscription>();

        // Set starting time for the server.
        state.ecs_mut().write_resource::<TimeOfDay>().0 = settings.start_time;

        let this = Self {
            state,
            world: Arc::new(World::generate(settings.world_seed)),

            postoffice: PostOffice::bind(settings.gameserver_address)?,
            clients: Clients::empty(),

            thread_pool: ThreadPoolBuilder::new()
                .name("veloren-worker".into())
                .build(),
            chunk_tx,
            chunk_rx,
            pending_chunks: HashMap::new(),

            server_info: ServerInfo {
                name: settings.server_name.clone(),
                description: settings.server_description.clone(),
                git_hash: common::util::GIT_HASH.to_string(),
                git_date: common::util::GIT_DATE.to_string(),
            },
            metrics: ServerMetrics::new(settings.metrics_address)
                .expect("Failed to initialize server metrics submodule."),
            tick: 0,
            accounts: AuthProvider::new(),
            server_settings: settings.clone(),
        };
        debug!("created veloren server");
        trace!("server configuration: {:?}", &settings);

        Ok(this)
    }

    pub fn with_thread_pool(mut self, thread_pool: ThreadPool) -> Self {
        self.thread_pool = thread_pool;
        self
    }

    /// Get a reference to the server's game state.
    pub fn state(&self) -> &State {
        &self.state
    }
    /// Get a mutable reference to the server's game state.
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    /// Get a reference to the server's world.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Build a non-player character.
    pub fn create_npc(
        &mut self,
        pos: comp::Pos,
        stats: comp::Stats,
        body: comp::Body,
    ) -> EcsEntityBuilder {
        self.state
            .ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(comp::Vel(Vec3::zero()))
            .with(comp::Ori(Vec3::unit_y()))
            .with(comp::Controller::default())
            .with(body)
            .with(stats)
            .with(comp::Gravity(1.0))
            .with(comp::CharacterState::default())
    }

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
        client: &mut Client,
        name: String,
        body: comp::Body,
        main: Option<comp::Item>,
        server_settings: &ServerSettings,
    ) {
        let spawn_point = state.ecs().read_resource::<SpawnPoint>().0;

        state.write_component(entity, body);
        state.write_component(entity, comp::Stats::new(name, main));
        state.write_component(entity, comp::Controller::default());
        state.write_component(entity, comp::Pos(spawn_point));
        state.write_component(entity, comp::Vel(Vec3::zero()));
        state.write_component(entity, comp::Ori(Vec3::unit_y()));
        state.write_component(entity, comp::Gravity(1.0));
        state.write_component(entity, comp::CharacterState::default());
        state.write_component(entity, comp::Inventory::default());
        state.write_component(entity, comp::InventoryUpdate);
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
        client.allow_state(ClientState::Character);
    }

    /// Handle events coming through via the event bus
    fn handle_events(&mut self) {
        let events = self
            .state
            .ecs()
            .read_resource::<EventBus<ServerEvent>>()
            .recv_all();
        for event in events {
            let state = &mut self.state;
            let clients = &mut self.clients;

            let mut todo_remove = None;

            match event {
                ServerEvent::Explosion { pos, radius } => {
                    const RAYS: usize = 500;

                    for _ in 0..RAYS {
                        let dir = Vec3::new(
                            rand::random::<f32>() - 0.5,
                            rand::random::<f32>() - 0.5,
                            rand::random::<f32>() - 0.5,
                        )
                        .normalized();

                        let ecs = state.ecs_mut();
                        let mut block_change = ecs.write_resource::<BlockChange>();

                        let _ = ecs
                            .read_resource::<TerrainGrid>()
                            .ray(pos, pos + dir * radius)
                            .until(|_| rand::random::<f32>() < 0.05)
                            .for_each(|pos| block_change.set(pos, Block::empty()))
                            .cast();
                    }
                }

                ServerEvent::Shoot {
                    entity,
                    dir,
                    body,
                    light,
                    projectile,
                    gravity,
                } => {
                    let mut pos = state
                        .ecs()
                        .read_storage::<comp::Pos>()
                        .get(entity)
                        .expect("Failed to fetch entity")
                        .0;

                    // TODO: Player height
                    pos.z += 1.2;

                    let mut builder = Self::create_projectile(
                        state,
                        comp::Pos(pos),
                        comp::Vel(dir * 100.0),
                        body,
                        projectile,
                    );
                    if let Some(light) = light {
                        builder = builder.with(light)
                    }
                    if let Some(gravity) = gravity {
                        builder = builder.with(gravity)
                    }

                    builder.build();
                }

                ServerEvent::Damage { uid, change } => {
                    let ecs = state.ecs_mut();
                    if let Some(entity) = ecs.entity_from_uid(uid.into()) {
                        if let Some(stats) = ecs.write_storage::<comp::Stats>().get_mut(entity) {
                            stats.health.change_by(change);
                        }
                    }
                }

                ServerEvent::Destroy { entity, cause } => {
                    let ecs = state.ecs_mut();
                    // Chat message
                    if let Some(player) = ecs.read_storage::<comp::Player>().get(entity) {
                        let msg = if let comp::HealthSource::Attack { by } = cause {
                            ecs.entity_from_uid(by.into()).and_then(|attacker| {
                                ecs.read_storage::<comp::Player>().get(attacker).map(
                                    |attacker_alias| {
                                        format!(
                                            "{} was killed by {}",
                                            &player.alias, &attacker_alias.alias
                                        )
                                    },
                                )
                            })
                        } else {
                            None
                        }
                        .unwrap_or(format!("{} died", &player.alias));

                        clients.notify_registered(ServerMsg::kill(msg));
                    }

                    // Give EXP to the killer if entity had stats
                    let mut stats = ecs.write_storage::<comp::Stats>();

                    if let Some(entity_stats) = stats.get(entity).cloned() {
                        if let comp::HealthSource::Attack { by } = cause {
                            ecs.entity_from_uid(by.into()).map(|attacker| {
                                if let Some(attacker_stats) = stats.get_mut(attacker) {
                                    // TODO: Discuss whether we should give EXP by Player Killing or not.
                                    attacker_stats
                                        .exp
                                        .change_by((entity_stats.level.level() * 10) as i64);
                                }
                            });
                        }
                    }

                    if let Some(client) = clients.get_mut(&entity) {
                        let _ = ecs.write_storage().insert(entity, comp::Vel(Vec3::zero()));
                        let _ = ecs.write_storage().insert(entity, comp::ForceUpdate);
                        client.force_state(ClientState::Dead);
                    } else {
                        todo_remove = Some(entity.clone());
                    }
                }

                ServerEvent::Respawn(entity) => {
                    // Only clients can respawn
                    if let Some(client) = clients.get_mut(&entity) {
                        let respawn_point = state
                            .read_component_cloned::<comp::Waypoint>(entity)
                            .map(|wp| wp.get_pos())
                            .unwrap_or(state.ecs().read_resource::<SpawnPoint>().0);

                        client.allow_state(ClientState::Character);
                        state
                            .ecs_mut()
                            .write_storage::<comp::Stats>()
                            .get_mut(entity)
                            .map(|stats| stats.revive());
                        state
                            .ecs_mut()
                            .write_storage::<comp::Pos>()
                            .get_mut(entity)
                            .map(|pos| pos.0 = respawn_point);
                        let _ = state
                            .ecs_mut()
                            .write_storage()
                            .insert(entity, comp::ForceUpdate);
                    }
                }

                ServerEvent::LandOnGround { entity, vel } => {
                    if vel.z <= -25.0 {
                        if let Some(stats) = state
                            .ecs_mut()
                            .write_storage::<comp::Stats>()
                            .get_mut(entity)
                        {
                            let falldmg = (vel.z / 5.0) as i32;
                            if falldmg < 0 {
                                stats.health.change_by(comp::HealthChange {
                                    amount: falldmg,
                                    cause: comp::HealthSource::World,
                                });
                            }
                        }
                    }
                }

                ServerEvent::Mount(mounter, mountee) => {
                    if state
                        .ecs()
                        .read_storage::<comp::Mounting>()
                        .get(mounter)
                        .is_none()
                    {
                        let not_mounting_yet = if let Some(comp::MountState::Unmounted) = state
                            .ecs()
                            .write_storage::<comp::MountState>()
                            .get_mut(mountee)
                            .cloned()
                        {
                            true
                        } else {
                            false
                        };

                        if not_mounting_yet {
                            if let (Some(mounter_uid), Some(mountee_uid)) = (
                                state.ecs().uid_from_entity(mounter),
                                state.ecs().uid_from_entity(mountee),
                            ) {
                                state.write_component(
                                    mountee,
                                    comp::MountState::MountedBy(mounter_uid.into()),
                                );
                                state.write_component(mounter, comp::Mounting(mountee_uid.into()));
                            }
                        }
                    }
                }
                ServerEvent::Unmount(mounter) => {
                    let mountee_entity = state
                        .ecs()
                        .write_storage::<comp::Mounting>()
                        .get(mounter)
                        .and_then(|mountee| state.ecs().entity_from_uid(mountee.0.into()));
                    if let Some(mountee_entity) = mountee_entity {
                        state
                            .ecs_mut()
                            .write_storage::<comp::MountState>()
                            .get_mut(mountee_entity)
                            .map(|ms| *ms = comp::MountState::Unmounted);
                    }
                    state.delete_component::<comp::Mounting>(mounter);
                }
                ServerEvent::Possess(possessor_uid, possesse_uid) => {
                    if let (Some(possessor), Some(possesse)) = (
                        state.ecs().entity_from_uid(possessor_uid.into()),
                        state.ecs().entity_from_uid(possesse_uid.into()),
                    ) {
                        // You can't possess other players
                        if clients.get(&possesse).is_none() {
                            if let Some(mut client) = clients.remove(&possessor) {
                                client.notify(ServerMsg::SetPlayerEntity(possesse_uid.into()));
                                clients.add(possesse, client);
                                // Create inventory if it doesn't exist
                                {
                                    let mut inventories =
                                        state.ecs_mut().write_storage::<comp::Inventory>();
                                    if let Some(inventory) = inventories.get_mut(possesse) {
                                        inventory
                                            .push(comp::Item::Debug(comp::item::Debug::Possess));
                                    } else {
                                        let _ = inventories.insert(
                                            possesse,
                                            comp::Inventory {
                                                slots: vec![Some(comp::Item::Debug(
                                                    comp::item::Debug::Possess,
                                                ))],
                                            },
                                        );
                                    }
                                }
                                let _ = state
                                    .ecs_mut()
                                    .write_storage::<comp::InventoryUpdate>()
                                    .insert(possesse, comp::InventoryUpdate);
                                // Move player component
                                {
                                    let mut players =
                                        state.ecs_mut().write_storage::<comp::Player>();
                                    if let Some(player) = players.get(possessor).cloned() {
                                        let _ = players.insert(possesse, player);
                                    }
                                }
                                // Remove will of the entity
                                let _ = state
                                    .ecs_mut()
                                    .write_storage::<comp::Agent>()
                                    .remove(possesse);
                                // Transfer admin powers
                                {
                                    let mut admins = state.ecs_mut().write_storage::<comp::Admin>();
                                    if let Some(admin) = admins.remove(possessor) {
                                        let _ = admins.insert(possesse, admin);
                                    }
                                }
                                // Transfer waypoint
                                {
                                    let mut waypoints =
                                        state.ecs_mut().write_storage::<comp::Waypoint>();
                                    if let Some(waypoint) = waypoints.remove(possessor) {
                                        let _ = waypoints.insert(possesse, waypoint);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(entity) = todo_remove {
                let _ = state.ecs_mut().delete_entity_synced(entity);
            }
        }
    }

    /// Execute a single server tick, handle input and update the game state by the given duration.
    pub fn tick(&mut self, _input: Input, dt: Duration) -> Result<Vec<Event>, Error> {
        self.tick += 1;
        // This tick function is the centre of the Veloren universe. Most server-side things are
        // managed from here, and as such it's important that it stays organised. Please consult
        // the core developers before making significant changes to this code. Here is the
        // approximate order of things. Please update it as this code changes.
        //
        // 1) Collect input from the frontend, apply input effects to the state of the game
        // 2) Go through any events (timer-driven or otherwise) that need handling and apply them
        //    to the state of the game
        // 3) Go through all incoming client network communications, apply them to the game state
        // 4) Perform a single LocalState tick (i.e: update the world and entities in the world)
        // 5) Go through the terrain update queue and apply all changes to the terrain
        // 6) Send relevant state updates to all clients
        // 7) Update Metrics with current data
        // 8) Finish the tick, passing control of the main thread back to the frontend

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
        frontend_events.append(&mut self.handle_new_messages()?);

        // Handle game events
        self.handle_events();

        let before_tick_4 = Instant::now();
        // 4) Tick the client's LocalState.
        self.state.tick(dt);

        // Tick the world
        self.world.tick(dt);

        let before_tick_5 = Instant::now();
        // 5) Fetch any generated `TerrainChunk`s and insert them into the terrain.
        // Also, send the chunk data to anybody that is close by.
        'insert_terrain_chunks: while let Ok((key, res)) = self.chunk_rx.try_recv() {
            let (chunk, supplement) = match res {
                Ok((chunk, supplement)) => (chunk, supplement),
                Err(entity) => {
                    self.clients.notify(
                        entity,
                        ServerMsg::TerrainChunkUpdate {
                            key,
                            chunk: Err(()),
                        },
                    );
                    continue 'insert_terrain_chunks;
                }
            };
            // Send the chunk to all nearby players.
            for (entity, view_distance, pos) in (
                &self.state.ecs().entities(),
                &self.state.ecs().read_storage::<comp::Player>(),
                &self.state.ecs().read_storage::<comp::Pos>(),
            )
                .join()
                .filter_map(|(entity, player, pos)| {
                    player.view_distance.map(|vd| (entity, vd, pos))
                })
            {
                let chunk_pos = self.state.terrain().pos_key(pos.0.map(|e| e as i32));
                let adjusted_dist_sqr = (Vec2::from(chunk_pos) - Vec2::from(key))
                    .map(|e: i32| (e.abs() as u32).checked_sub(2).unwrap_or(0))
                    .magnitude_squared();

                if adjusted_dist_sqr <= view_distance.pow(2) {
                    self.clients.notify(
                        entity,
                        ServerMsg::TerrainChunkUpdate {
                            key,
                            chunk: Ok(Box::new(chunk.clone())),
                        },
                    );
                }
            }

            self.state.insert_chunk(key, chunk);
            self.pending_chunks.remove(&key);

            // Handle chunk supplement
            for npc in supplement.npcs {
                let (mut stats, mut body) = if rand::random() {
                    let stats = comp::Stats::new(
                        "Humanoid".to_string(),
                        Some(comp::Item::Tool {
                            kind: comp::item::Tool::Sword,
                            power: 5,
                            stamina: 0,
                            strength: 0,
                            dexterity: 0,
                            intelligence: 0,
                        }),
                    );
                    let body = comp::Body::Humanoid(comp::humanoid::Body::random());
                    (stats, body)
                } else {
                    let stats = comp::Stats::new("Wolf".to_string(), None);
                    let body = comp::Body::QuadrupedMedium(comp::quadruped_medium::Body::random());
                    (stats, body)
                };
                let mut scale = 1.0;

                // TODO: Remove this and implement scaling or level depending on stuff like species instead
                stats.level.set_level(rand::thread_rng().gen_range(1, 3));

                if npc.boss {
                    if rand::random::<f32>() < 0.8 {
                        stats = comp::Stats::new(
                            "Humanoid".to_string(),
                            Some(comp::Item::Tool {
                                kind: comp::item::Tool::Sword,
                                power: 10,
                                stamina: 0,
                                strength: 0,
                                dexterity: 0,
                                intelligence: 0,
                            }),
                        );
                        body = comp::Body::Humanoid(comp::humanoid::Body::random());
                    }
                    stats.level.set_level(rand::thread_rng().gen_range(10, 50));
                    scale = 2.5 + rand::random::<f32>();
                }

                stats.update_max_hp();
                stats
                    .health
                    .set_to(stats.health.maximum(), comp::HealthSource::Revive);
                self.create_npc(comp::Pos(npc.pos), stats, body)
                    .with(comp::Agent::enemy())
                    .with(comp::Scale(scale))
                    .build();
            }
        }

        fn chunk_in_vd(
            player_pos: Vec3<f32>,
            chunk_pos: Vec2<i32>,
            terrain: &TerrainGrid,
            vd: u32,
        ) -> bool {
            let player_chunk_pos = terrain.pos_key(player_pos.map(|e| e as i32));

            let adjusted_dist_sqr = Vec2::from(player_chunk_pos - chunk_pos)
                .map(|e: i32| (e.abs() as u32).checked_sub(2).unwrap_or(0))
                .magnitude_squared();

            adjusted_dist_sqr <= vd.pow(2)
        }

        // Remove chunks that are too far from players.
        let mut chunks_to_remove = Vec::new();
        self.state
            .terrain()
            .iter()
            .map(|(k, _)| k)
            .chain(self.pending_chunks.keys().cloned())
            .for_each(|chunk_key| {
                let mut should_drop = true;

                // For each player with a position, calculate the distance.
                for (player, pos) in (
                    &self.state.ecs().read_storage::<comp::Player>(),
                    &self.state.ecs().read_storage::<comp::Pos>(),
                )
                    .join()
                {
                    if player
                        .view_distance
                        .map(|vd| chunk_in_vd(pos.0, chunk_key, &self.state.terrain(), vd))
                        .unwrap_or(false)
                    {
                        should_drop = false;
                        break;
                    }
                }

                if should_drop {
                    chunks_to_remove.push(chunk_key);
                }
            });
        for key in chunks_to_remove {
            self.state.remove_chunk(key);
            if let Some(cancel) = self.pending_chunks.remove(&key) {
                cancel.store(true, Ordering::Relaxed);
            }
        }

        let before_tick_6 = Instant::now();
        // 6) Synchronise clients with the new state of the world.
        self.sync_clients();

        // Sync changed chunks
        'chunk: for chunk_key in &self.state.terrain_changes().modified_chunks {
            let terrain = self.state.terrain();

            for (entity, player, pos) in (
                &self.state.ecs().entities(),
                &self.state.ecs().read_storage::<comp::Player>(),
                &self.state.ecs().read_storage::<comp::Pos>(),
            )
                .join()
            {
                if player
                    .view_distance
                    .map(|vd| chunk_in_vd(pos.0, *chunk_key, &terrain, vd))
                    .unwrap_or(false)
                {
                    self.clients.notify(
                        entity,
                        ServerMsg::TerrainChunkUpdate {
                            key: *chunk_key,
                            chunk: Ok(Box::new(match self.state.terrain().get_key(*chunk_key) {
                                Some(chunk) => chunk.clone(),
                                None => break 'chunk,
                            })),
                        },
                    );
                }
            }
        }

        // Sync changed blocks
        let msg =
            ServerMsg::TerrainBlockUpdates(self.state.terrain_changes().modified_blocks.clone());
        for (entity, player) in (
            &self.state.ecs().entities(),
            &self.state.ecs().read_storage::<comp::Player>(),
        )
            .join()
        {
            if player.view_distance.is_some() {
                self.clients.notify(entity, msg.clone());
            }
        }

        // Remove NPCs that are outside the view distances of all players
        // This is done by removing NPCs in unloaded chunks
        let to_delete = {
            let terrain = self.state.terrain();
            (
                &self.state.ecs().entities(),
                &self.state.ecs().read_storage::<comp::Pos>(),
                &self.state.ecs().read_storage::<comp::Agent>(),
            )
                .join()
                .filter(|(_, pos, _)| terrain.get(pos.0.map(|e| e.floor() as i32)).is_err())
                .map(|(entity, _, _)| entity)
                .collect::<Vec<_>>()
        };
        for entity in to_delete {
            let _ = self.state.ecs_mut().delete_entity_synced(entity);
        }

        let before_tick_7 = Instant::now();
        // 7) Update Metrics
        self.metrics
            .tick_time
            .with_label_values(&["input"])
            .set((before_tick_4 - before_tick_1).as_nanos() as i64);
        self.metrics
            .tick_time
            .with_label_values(&["world"])
            .set((before_tick_5 - before_tick_4).as_nanos() as i64);
        self.metrics
            .tick_time
            .with_label_values(&["terrain"])
            .set((before_tick_6 - before_tick_5).as_nanos() as i64);
        self.metrics
            .tick_time
            .with_label_values(&["sync"])
            .set((before_tick_7 - before_tick_6).as_nanos() as i64);
        self.metrics.player_online.set(self.clients.len() as i64);
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
            let entity = self.state.ecs_mut().create_entity_synced().build();
            let mut client = Client {
                client_state: ClientState::Connected,
                postbox,
                last_ping: self.state.get_time(),
            };

            if self.server_settings.max_players <= self.clients.len() {
                client.notify(ServerMsg::Error(ServerError::TooManyPlayers));
            } else {
                // Return the state of the current world (all of the components that Sphynx tracks).
                log::info!("Starting initial sync with client.");
                client.notify(ServerMsg::InitialSync {
                    ecs_state: self.state.ecs().gen_state_package(),
                    entity_uid: self.state.ecs().uid_from_entity(entity).unwrap().into(), // Can't fail.
                    server_info: self.server_info.clone(),
                    // world_map: (WORLD_SIZE/*, self.world.sim().get_map()*/),
                });
                log::info!("Done initial sync with client.");

                frontend_events.push(Event::ClientConnected { entity });
            }

            self.clients.add(entity, client);
        }

        Ok(frontend_events)
    }

    /// Handle new client messages.
    fn handle_new_messages(&mut self) -> Result<Vec<Event>, Error> {
        let mut frontend_events = Vec::new();

        let accounts = &mut self.accounts;
        let server_settings = &self.server_settings;

        let state = &mut self.state;
        let mut new_chat_msgs = Vec::new();
        let mut disconnected_clients = Vec::new();
        let mut requested_chunks = Vec::new();
        let mut dropped_items = Vec::new();

        self.clients.remove_if(|entity, client| {
            let mut disconnect = false;
            let new_msgs = client.postbox.new_messages();

            // Update client ping.
            if new_msgs.len() > 0 {
                client.last_ping = state.get_time();

                // Process incoming messages.
                for msg in new_msgs {
                    match msg {
                        ClientMsg::RequestState(requested_state) => match requested_state {
                            ClientState::Connected => disconnect = true, // Default state
                            ClientState::Registered => match client.client_state {
                                // Use ClientMsg::Register instead.
                                ClientState::Connected => {
                                    client.error_state(RequestStateError::WrongMessage)
                                }
                                ClientState::Registered => {
                                    client.error_state(RequestStateError::Already)
                                }
                                ClientState::Spectator
                                | ClientState::Character
                                | ClientState::Dead => client.allow_state(ClientState::Registered),
                                ClientState::Pending => {}
                            },
                            ClientState::Spectator => match requested_state {
                                // Become Registered first.
                                ClientState::Connected => {
                                    client.error_state(RequestStateError::Impossible)
                                }
                                ClientState::Spectator => {
                                    client.error_state(RequestStateError::Already)
                                }
                                ClientState::Registered
                                | ClientState::Character
                                | ClientState::Dead => client.allow_state(ClientState::Spectator),
                                ClientState::Pending => {}
                            },
                            // Use ClientMsg::Character instead.
                            ClientState::Character => {
                                client.error_state(RequestStateError::WrongMessage)
                            }
                            ClientState::Dead => client.error_state(RequestStateError::Impossible),
                            ClientState::Pending => {}
                        },
                        // Valid player
                        ClientMsg::Register { player, password } if player.is_valid() => {
                            if !accounts.query(player.alias.clone(), password) {
                                client.error_state(RequestStateError::Denied);
                                break;
                            }
                            match client.client_state {
                                ClientState::Connected => {
                                    Self::initialize_player(state, entity, client, player);
                                }
                                // Use RequestState instead (No need to send `player` again).
                                _ => client.error_state(RequestStateError::Impossible),
                            }
                            //client.allow_state(ClientState::Registered);
                        }
                        // Invalid player
                        ClientMsg::Register { .. } => {
                            client.error_state(RequestStateError::Impossible)
                        }
                        ClientMsg::SetViewDistance(view_distance) => match client.client_state {
                            ClientState::Character { .. } => {
                                state
                                    .ecs_mut()
                                    .write_storage::<comp::Player>()
                                    .get_mut(entity)
                                    .map(|player| player.view_distance = Some(view_distance));
                            }
                            _ => {}
                        },
                        ClientMsg::UseInventorySlot(x) => {
                            let item = state
                                .ecs()
                                .write_storage::<comp::Inventory>()
                                .get_mut(entity)
                                .and_then(|inv| inv.remove(x));

                            match item {
                                Some(comp::Item::Tool { .. }) | Some(comp::Item::Debug(_)) => {
                                    if let Some(stats) =
                                        state.ecs().write_storage::<comp::Stats>().get_mut(entity)
                                    {
                                        // Insert old item into inventory
                                        if let Some(old_item) = stats.equipment.main.take() {
                                            state
                                                .ecs()
                                                .write_storage::<comp::Inventory>()
                                                .get_mut(entity)
                                                .map(|inv| inv.insert(x, old_item));
                                        }

                                        stats.equipment.main = item;
                                    }
                                }
                                Some(comp::Item::Consumable { effect, .. }) => {
                                    state.apply_effect(entity, effect);
                                }
                                Some(item) => {
                                    // Re-insert it if unused
                                    let _ = state
                                        .ecs()
                                        .write_storage::<comp::Inventory>()
                                        .get_mut(entity)
                                        .map(|inv| inv.insert(x, item));
                                }
                                _ => {}
                            }
                            state.write_component(entity, comp::InventoryUpdate);
                        }
                        ClientMsg::SwapInventorySlots(a, b) => {
                            state
                                .ecs()
                                .write_storage::<comp::Inventory>()
                                .get_mut(entity)
                                .map(|inv| inv.swap_slots(a, b));
                            state.write_component(entity, comp::InventoryUpdate);
                        }
                        ClientMsg::DropInventorySlot(x) => {
                            let item = state
                                .ecs()
                                .write_storage::<comp::Inventory>()
                                .get_mut(entity)
                                .and_then(|inv| inv.remove(x));

                            state.write_component(entity, comp::InventoryUpdate);

                            if let (Some(item), Some(pos)) =
                                (item, state.ecs().read_storage::<comp::Pos>().get(entity))
                            {
                                dropped_items.push((
                                    *pos,
                                    state
                                        .ecs()
                                        .read_storage::<comp::Ori>()
                                        .get(entity)
                                        .copied()
                                        .unwrap_or(comp::Ori(Vec3::unit_y())),
                                    item,
                                ));
                            }
                        }
                        ClientMsg::PickUp(uid) => {
                            let item_entity = state.ecs_mut().entity_from_uid(uid);

                            let ecs = state.ecs_mut();

                            let item_entity = if let (Some((item, item_entity)), Some(inv)) = (
                                item_entity.and_then(|item_entity| {
                                    ecs.write_storage::<comp::Item>()
                                        .get_mut(item_entity)
                                        .map(|item| (item.clone(), item_entity))
                                }),
                                ecs.write_storage::<comp::Inventory>().get_mut(entity),
                            ) {
                                if inv.push(item).is_none() {
                                    Some(item_entity)
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            if let Some(item_entity) = item_entity {
                                let _ = ecs.delete_entity_synced(item_entity);
                            }

                            state.write_component(entity, comp::InventoryUpdate);
                        }
                        ClientMsg::Character { name, body, main } => match client.client_state {
                            // Become Registered first.
                            ClientState::Connected => {
                                client.error_state(RequestStateError::Impossible)
                            }
                            ClientState::Registered
                            | ClientState::Spectator
                            | ClientState::Dead => {
                                if let (Some(player), None) = (
                                    state.ecs().read_storage::<comp::Player>().get(entity),
                                    // Only send login message if the player didn't have a body
                                    // previously
                                    state.ecs().read_storage::<comp::Body>().get(entity),
                                ) {
                                    new_chat_msgs.push((
                                        None,
                                        ServerMsg::broadcast(format!(
                                            "[{}] is now online.",
                                            &player.alias
                                        )),
                                    ));
                                }

                                Self::create_player_character(
                                    state,
                                    entity,
                                    client,
                                    name,
                                    body,
                                    main.map(|t| comp::Item::Tool {
                                        kind: t,
                                        power: 10,
                                        stamina: 0,
                                        strength: 0,
                                        dexterity: 0,
                                        intelligence: 0,
                                    }),
                                    &server_settings,
                                );
                                Self::initialize_region_subscription(state, client, entity);
                            }
                            ClientState::Character => {
                                client.error_state(RequestStateError::Already)
                            }
                            ClientState::Pending => {}
                        },
                        ClientMsg::Controller(controller) => match client.client_state {
                            ClientState::Connected
                            | ClientState::Registered
                            | ClientState::Spectator => {
                                client.error_state(RequestStateError::Impossible)
                            }
                            ClientState::Dead | ClientState::Character => {
                                state.write_component(entity, controller);
                            }
                            ClientState::Pending => {}
                        },
                        ClientMsg::ChatMsg { chat_type, message } => match client.client_state {
                            ClientState::Connected => {
                                client.error_state(RequestStateError::Impossible)
                            }
                            ClientState::Registered
                            | ClientState::Spectator
                            | ClientState::Dead
                            | ClientState::Character => match validate_chat_msg(&message) {
                                Ok(()) => new_chat_msgs.push((
                                    Some(entity),
                                    ServerMsg::ChatMsg { chat_type, message },
                                )),
                                Err(ChatMsgValidationError::TooLong) => log::warn!(
                                    "Recieved a chat message that's too long (max:{} len:{})",
                                    MAX_BYTES_CHAT_MSG,
                                    message.len()
                                ),
                            },
                            ClientState::Pending => {}
                        },
                        ClientMsg::PlayerPhysics { pos, vel, ori } => match client.client_state {
                            ClientState::Character => {
                                state.write_component(entity, pos);
                                state.write_component(entity, vel);
                                state.write_component(entity, ori);
                            }
                            // Only characters can send positions.
                            _ => client.error_state(RequestStateError::Impossible),
                        },
                        ClientMsg::BreakBlock(pos) => {
                            if state
                                .ecs_mut()
                                .read_storage::<comp::CanBuild>()
                                .get(entity)
                                .is_some()
                            {
                                state.set_block(pos, Block::empty());
                            }
                        }
                        ClientMsg::PlaceBlock(pos, block) => {
                            if state
                                .ecs_mut()
                                .read_storage::<comp::CanBuild>()
                                .get(entity)
                                .is_some()
                            {
                                state.try_set_block(pos, block);
                            }
                        }
                        ClientMsg::CollectBlock(pos) => {
                            let block = state.terrain().get(pos).ok().copied();
                            if let Some(block) = block {
                                if block.is_collectible()
                                    && state
                                        .ecs()
                                        .read_storage::<comp::Inventory>()
                                        .get(entity)
                                        .map(|inv| !inv.is_full())
                                        .unwrap_or(false)
                                {
                                    if state.try_set_block(pos, Block::empty()).is_some() {
                                        comp::Item::try_reclaim_from_block(block)
                                            .map(|item| state.give_item(entity, item));
                                    }
                                }
                            }
                        }
                        ClientMsg::TerrainChunkRequest { key } => match client.client_state {
                            ClientState::Connected
                            | ClientState::Registered
                            | ClientState::Dead => {
                                client.error_state(RequestStateError::Impossible);
                            }
                            ClientState::Spectator | ClientState::Character => {
                                match state.terrain().get_key(key) {
                                    Some(chunk) => {
                                        client.postbox.send_message(ServerMsg::TerrainChunkUpdate {
                                            key,
                                            chunk: Ok(Box::new(chunk.clone())),
                                        })
                                    }
                                    None => requested_chunks.push((entity, key)),
                                }
                            }
                            ClientState::Pending => {}
                        },
                        // Always possible.
                        ClientMsg::Ping => client.postbox.send_message(ServerMsg::Pong),
                        ClientMsg::Pong => {}
                        ClientMsg::Disconnect => {
                            disconnect = true;
                        }
                    }
                }
            } else if state.get_time() - client.last_ping > CLIENT_TIMEOUT || // Timeout
                client.postbox.error().is_some()
            // Postbox error
            {
                disconnect = true;
            } else if state.get_time() - client.last_ping > CLIENT_TIMEOUT * 0.5 {
                // Try pinging the client if the timeout is nearing.
                client.postbox.send_message(ServerMsg::Ping);
            }

            if disconnect {
                if let (Some(player), Some(_)) = (
                    state.ecs().read_storage::<comp::Player>().get(entity),
                    // It only shows a message if you had a body (not in char selection)
                    state.ecs().read_storage::<comp::Body>().get(entity),
                ) {
                    new_chat_msgs.push((
                        None,
                        ServerMsg::broadcast(format!("{} went offline.", &player.alias)),
                    ));
                }
                disconnected_clients.push(entity);
                client.postbox.send_message(ServerMsg::Disconnect);
                true
            } else {
                false
            }
        });

        // Handle new chat messages.
        for (entity, msg) in new_chat_msgs {
            match msg {
                ServerMsg::ChatMsg { chat_type, message } => {
                    if let Some(entity) = entity {
                        // Handle chat commands.
                        if message.starts_with("/") && message.len() > 1 {
                            let argv = String::from(&message[1..]);
                            self.process_chat_cmd(entity, argv);
                        } else {
                            let message =
                                match self.state.ecs().read_storage::<comp::Player>().get(entity) {
                                    Some(player) => {
                                        if self.entity_is_admin(entity) {
                                            format!("[ADMIN][{}] {}", &player.alias, message)
                                        } else {
                                            format!("[{}] {}", &player.alias, message)
                                        }
                                    }
                                    None => format!("[<Unknown>] {}", message),
                                };
                            self.clients
                                .notify_registered(ServerMsg::ChatMsg { chat_type, message });
                        }
                    } else {
                        self.clients
                            .notify_registered(ServerMsg::ChatMsg { chat_type, message });
                    }
                }
                _ => {
                    panic!("Invalid message type.");
                }
            }
        }

        // Handle client disconnects.
        for entity in disconnected_clients {
            if let Err(err) = self.state.ecs_mut().delete_entity_synced(entity) {
                debug!("Failed to delete disconnected client: {:?}", err);
            }

            frontend_events.push(Event::ClientDisconnected { entity });
        }

        // Generate requested chunks.
        for (entity, key) in requested_chunks {
            self.generate_chunk(entity, key);
        }

        for (pos, ori, item) in dropped_items {
            let vel = ori.0.normalized() * 5.0
                + Vec3::unit_z() * 10.0
                + Vec3::<f32>::zero().map(|_| rand::thread_rng().gen::<f32>() - 0.5) * 4.0;
            self.create_object(Default::default(), comp::object::Body::Pouch)
                .with(comp::Pos(pos.0 + Vec3::unit_z() * 0.25))
                .with(item)
                .with(comp::Vel(vel))
                .build();
        }

        Ok(frontend_events)
    }

    /// Initialize a new client states with important information.
    fn initialize_player(
        state: &mut State,
        entity: specs::Entity,
        client: &mut Client,
        player: comp::Player,
    ) {
        // Save player metadata (for example the username).
        state.write_component(entity, player);

        // Tell the client its request was successful.
        client.allow_state(ClientState::Registered);
    }

    /// Initialize region subscription, entity should be the client's entity
    fn initialize_region_subscription(
        state: &mut State,
        client: &mut Client,
        entity: specs::Entity,
    ) {
        let mut subscription = None;

        if let (Some(client_pos), Some(client_vd)) = (
            state.ecs().read_storage::<comp::Pos>().get(entity),
            state
                .ecs()
                .read_storage::<comp::Player>()
                .get(entity)
                .map(|pl| pl.view_distance)
                .and_then(|v| v),
        ) {
            use common::region::RegionMap;

            let fuzzy_chunk = (Vec2::<f32>::from(client_pos.0))
                .map2(TerrainChunkSize::RECT_SIZE, |e, sz| e as i32 / sz as i32);
            let chunk_size = TerrainChunkSize::RECT_SIZE.reduce_max() as f32;
            let regions = common::region::regions_in_vd(
                client_pos.0,
                (client_vd as f32 * chunk_size) as f32
                    + (client::CHUNK_FUZZ as f32 + chunk_size) * 2.0f32.sqrt(),
            );

            for (_, region) in state
                .ecs()
                .read_resource::<RegionMap>()
                .iter()
                .filter(|(key, _)| regions.contains(key))
            {
                // Sync physics of all entities in this region
                for (&uid, &pos, vel, ori, character_state, _) in (
                    &state.ecs().read_storage::<Uid>(),
                    &state.ecs().read_storage::<comp::Pos>(), // We assume all these entities have a position
                    state.ecs().read_storage::<comp::Vel>().maybe(),
                    state.ecs().read_storage::<comp::Ori>().maybe(),
                    state.ecs().read_storage::<comp::CharacterState>().maybe(),
                    region.entities(),
                )
                    .join()
                {
                    client.notify(ServerMsg::EntityPos {
                        entity: uid.into(),
                        pos,
                    });
                    if let Some(vel) = vel.copied() {
                        client.notify(ServerMsg::EntityVel {
                            entity: uid.into(),
                            vel,
                        });
                    }
                    if let Some(ori) = ori.copied() {
                        client.notify(ServerMsg::EntityOri {
                            entity: uid.into(),
                            ori,
                        });
                    }
                    if let Some(character_state) = character_state.copied() {
                        client.notify(ServerMsg::EntityCharacterState {
                            entity: uid.into(),
                            character_state,
                        });
                    }
                }
            }

            subscription = Some(RegionSubscription {
                fuzzy_chunk,
                regions,
            });
        }
        if let Some(subscription) = subscription {
            state.write_component(entity, subscription);
        }
    }

    /// Sync client states with the most up to date information.
    fn sync_clients(&mut self) {
        use common::region::{region_in_vd, regions_in_vd, Event as RegionEvent, RegionMap};
        //use hibitset::BitSetLike;

        let ecs = self.state.ecs_mut();
        let clients = &mut self.clients;

        // Sync 'logical' state using Sphynx.
        clients.notify_registered(ServerMsg::EcsSync(ecs.next_sync_package()));

        // To update subscriptions
        // 1. Iterate through clients
        // 2. Calculate current chunk position
        // 3. If chunk is the same return, otherwise continue (use fuzzyiness)
        // 4. Iterate through subscribed regions
        // 5. Check if region is still in range (use fuzzyiness)
        // 6. If not in range
        //     - remove from hashset
        //     - inform client of which entities to remove
        // 7. Determine list of regions that are in range and iterate through it
        //    - check if in hashset (hash calc) if not add it
        let mut regions_to_remove = Vec::new();
        for (entity, subscription, pos, vd) in (
            &ecs.entities(),
            &mut ecs.write_storage::<RegionSubscription>(),
            &ecs.read_storage::<comp::Pos>(),
            &ecs.read_storage::<comp::Player>(),
        )
            .join()
            .filter_map(|(e, s, pos, player)| player.view_distance.map(|v| (e, s, pos, v)))
        {
            let chunk = (Vec2::<f32>::from(pos.0))
                .map2(TerrainChunkSize::RECT_SIZE, |e, sz| e as i32 / sz as i32);
            // Only update regions when moving to a new chunk
            // uses a fuzzy border to prevent rapid triggering when moving along chunk boundaries
            if chunk != subscription.fuzzy_chunk
                && (subscription
                    .fuzzy_chunk
                    .map2(TerrainChunkSize::RECT_SIZE, |e, sz| {
                        (e as f32 + 0.5) * sz as f32
                    })
                    - Vec2::from(pos.0))
                .map2(TerrainChunkSize::RECT_SIZE, |e, sz| {
                    e.abs() > (sz / 2 + client::CHUNK_FUZZ) as f32
                })
                .reduce_or()
            {
                // Update current chunk
                subscription.fuzzy_chunk = (Vec2::<f32>::from(pos.0))
                    .map2(TerrainChunkSize::RECT_SIZE, |e, sz| e as i32 / sz as i32);
                // Use the largest side length as our chunk size
                let chunk_size = TerrainChunkSize::RECT_SIZE.reduce_max() as f32;
                // Iterate through currently subscribed regions
                for key in &subscription.regions {
                    // Check if the region is not within range anymore
                    if !region_in_vd(
                        *key,
                        pos.0,
                        (vd as f32 * chunk_size)
                            + (client::CHUNK_FUZZ as f32 + client::REGION_FUZZ as f32 + chunk_size)
                                * 2.0f32.sqrt(),
                    ) {
                        // Add to the list of regions to remove
                        regions_to_remove.push(*key);
                    }
                }

                let mut client = clients.get_mut(&entity);
                // Iterate through regions to remove
                for key in regions_to_remove.drain(..) {
                    // Remove region from this clients set of subscribed regions
                    subscription.regions.remove(&key);
                    // Tell the client to delete the entities in that region if it exists in the RegionMap
                    if let (Some(ref mut client), Some(region)) =
                        (&mut client, ecs.read_resource::<RegionMap>().get(key))
                    {
                        // Process entity left events since they won't be processed below because this region is no longer subscribed to
                        for event in region.events() {
                            match event {
                                RegionEvent::Entered(_, _) => {} // These don't need to be processed because this region is being thrown out anyway
                                RegionEvent::Left(id, maybe_key) => {
                                    // Lookup UID for entity
                                    if let Some(&uid) =
                                        ecs.read_storage::<Uid>().get(ecs.entities().entity(*id))
                                    {
                                        if !maybe_key
                                            .as_ref()
                                            .map(|key| subscription.regions.contains(key))
                                            .unwrap_or(false)
                                        {
                                            client.notify(ServerMsg::DeleteEntity(uid.into()));
                                        }
                                    }
                                }
                            }
                        }
                        for (&uid, _) in (&ecs.read_storage::<Uid>(), region.entities()).join() {
                            client.notify(ServerMsg::DeleteEntity(uid.into()))
                        }
                    }
                }

                for key in regions_in_vd(
                    pos.0,
                    (vd as f32 * chunk_size)
                        + (client::CHUNK_FUZZ as f32 + chunk_size) * 2.0f32.sqrt(),
                ) {
                    if subscription.regions.insert(key) {
                        // TODO: send the client initial infromation for all the entities in this region
                    }
                }
            }
        }

        // To send entity updates
        // 1. Iterate through regions
        // 2. Iterate through region subscribers (ie clients)
        //     - Collect a list of entity ids for clients who are subscribed to this region (hash calc to check each)
        // 3. Iterate through events from that region
        //     - For each entity left event, iterate through the client list and check if they are subscribed to the destination (hash calc per subscribed client per entity left event)
        //     - Do something with entity entered events when sphynx is removed??
        // 4. Iterate through entities in that region
        // 5. Inform clients of the component changes for that entity
        //     - Throttle update rate base on distance to each client

        // Sync physics
        // via iterating through regions
        for (key, region) in ecs.read_resource::<RegionMap>().iter() {
            let subscriptions = ecs.read_storage::<RegionSubscription>();
            let subscribers = (
                &ecs.entities(),
                &subscriptions,
                &ecs.read_storage::<comp::Pos>(),
            )
                .join()
                .filter_map(|(entity, subscription, pos)| {
                    if subscription.regions.contains(&key) {
                        clients
                            .get_client_index_ingame(&entity)
                            .map(|index| (index, &subscription.regions, entity, *pos))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            for event in region.events() {
                match event {
                    RegionEvent::Entered(_, _) => {} // TODO use this
                    RegionEvent::Left(id, maybe_key) => {
                        // Lookup UID for entity
                        if let Some(&uid) =
                            ecs.read_storage::<Uid>().get(ecs.entities().entity(*id))
                        {
                            for (client_index, regions, _, _) in &subscribers {
                                if !maybe_key
                                    .as_ref()
                                    .map(|key| regions.contains(key))
                                    .unwrap_or(false)
                                {
                                    clients.notify_index(
                                        *client_index,
                                        ServerMsg::DeleteEntity(uid.into()),
                                    );
                                }
                            }
                        }
                    }
                }
            }

            let tick = self.tick;
            let send_msg = |msg: ServerMsg,
                            entity: EcsEntity,
                            pos: comp::Pos,
                            force_update,
                            clients: &mut Clients,
                            throttle: bool| {
                for (index, _, client_entity, client_pos) in &subscribers {
                    match force_update {
                        None if client_entity == &entity => {}
                        _ => {
                            let distance_sq = client_pos.0.distance_squared(pos.0);

                            // Throttle update rate based on distance to player
                            let update = if !throttle || distance_sq < 100.0f32.powi(2) {
                                true // Closer than 100.0 blocks
                            } else if distance_sq < 150.0f32.powi(2) {
                                (tick + entity.id() as u64) % 2 == 0
                            } else if distance_sq < 200.0f32.powi(2) {
                                (tick + entity.id() as u64) % 4 == 0
                            } else if distance_sq < 250.0f32.powi(2) {
                                (tick + entity.id() as u64) % 8 == 0
                            } else if distance_sq < 300.0f32.powi(2) {
                                (tick + entity.id() as u64) % 16 == 0
                            } else {
                                (tick + entity.id() as u64) % 32 == 0
                            };

                            if update {
                                clients.notify_index(*index, msg.clone());
                            }
                        }
                    }
                }
            };

            for (_, entity, &uid, &pos, maybe_vel, maybe_ori, character_state, force_update) in (
                region.entities(),
                &ecs.entities(),
                &ecs.read_storage::<Uid>(),
                &ecs.read_storage::<comp::Pos>(),
                ecs.read_storage::<comp::Vel>().maybe(),
                ecs.read_storage::<comp::Ori>().maybe(),
                ecs.read_storage::<comp::CharacterState>().maybe(),
                ecs.read_storage::<comp::ForceUpdate>().maybe(),
            )
                .join()
            {
                let mut last_pos = ecs.write_storage::<comp::Last<comp::Pos>>();
                let mut last_vel = ecs.write_storage::<comp::Last<comp::Vel>>();
                let mut last_ori = ecs.write_storage::<comp::Last<comp::Ori>>();
                let mut last_character_state =
                    ecs.write_storage::<comp::Last<comp::CharacterState>>();

                if last_pos.get(entity).map(|&l| l.0 != pos).unwrap_or(true) {
                    let _ = last_pos.insert(entity, comp::Last(pos));
                    send_msg(
                        ServerMsg::EntityPos {
                            entity: uid.into(),
                            pos,
                        },
                        entity,
                        pos,
                        force_update,
                        clients,
                        true,
                    );
                }

                if let Some(&vel) = maybe_vel {
                    if last_vel.get(entity).map(|&l| l.0 != vel).unwrap_or(true) {
                        let _ = last_vel.insert(entity, comp::Last(vel));
                        send_msg(
                            ServerMsg::EntityVel {
                                entity: uid.into(),
                                vel,
                            },
                            entity,
                            pos,
                            force_update,
                            clients,
                            true,
                        );
                    }
                }

                if let Some(&ori) = maybe_ori {
                    if last_ori.get(entity).map(|&l| l.0 != ori).unwrap_or(true) {
                        let _ = last_ori.insert(entity, comp::Last(ori));
                        send_msg(
                            ServerMsg::EntityOri {
                                entity: uid.into(),
                                ori,
                            },
                            entity,
                            pos,
                            force_update,
                            clients,
                            true,
                        );
                    }
                }

                if let Some(&character_state) = character_state {
                    if last_character_state
                        .get(entity)
                        .map(|&l| !character_state.is_same_state(&l.0))
                        .unwrap_or(true)
                    {
                        let _ = last_character_state.insert(entity, comp::Last(character_state));
                        send_msg(
                            ServerMsg::EntityCharacterState {
                                entity: uid.into(),
                                character_state,
                            },
                            entity,
                            pos,
                            force_update,
                            clients,
                            false,
                        );
                    }
                }
            }
        }

        // Sync inventories
        for (entity, inventory, _) in (
            &self.state.ecs().entities(),
            &self.state.ecs().read_storage::<comp::Inventory>(),
            &self.state.ecs().read_storage::<comp::InventoryUpdate>(),
        )
            .join()
        {
            self.clients
                .notify(entity, ServerMsg::InventoryUpdate(inventory.clone()));
        }

        // Remove all force flags.
        self.state
            .ecs_mut()
            .write_storage::<comp::ForceUpdate>()
            .clear();
        self.state
            .ecs_mut()
            .write_storage::<comp::InventoryUpdate>()
            .clear();
    }

    pub fn generate_chunk(&mut self, entity: EcsEntity, key: Vec2<i32>) {
        let v = if let Entry::Vacant(v) = self.pending_chunks.entry(key) {
            v
        } else {
            return;
        };
        let cancel = Arc::new(AtomicBool::new(false));
        v.insert(Arc::clone(&cancel));
        let chunk_tx = self.chunk_tx.clone();
        let world = self.world.clone();
        self.thread_pool.execute(move || {
            let payload = world
                .generate_chunk(key, || cancel.load(Ordering::Relaxed))
                .map_err(|_| entity);
            let _ = chunk_tx.send((key, payload));
        });
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
                self.clients.notify(
                    entity,
                    ServerMsg::private(format!(
                        "Unknown command '/{}'.\nType '/help' for available commands",
                        kwd
                    )),
                );
            }
        }
    }

    fn entity_is_admin(&self, entity: EcsEntity) -> bool {
        self.state
            .read_storage::<comp::Admin>()
            .get(entity)
            .is_some()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.clients.notify_registered(ServerMsg::Shutdown);
    }
}

trait StateExt {
    fn give_item(&mut self, entity: EcsEntity, item: comp::Item) -> bool;
    fn apply_effect(&mut self, entity: EcsEntity, effect: Effect);
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
            self.write_component(entity, comp::InventoryUpdate);
        }
        success
    }

    fn apply_effect(&mut self, entity: EcsEntity, effect: Effect) {
        match effect {
            Effect::Health(change) => {
                self.ecs_mut()
                    .write_storage::<comp::Stats>()
                    .get_mut(entity)
                    .map(|stats| stats.health.change_by(change));
            }
            Effect::Xp(xp) => {
                self.ecs_mut()
                    .write_storage::<comp::Stats>()
                    .get_mut(entity)
                    .map(|stats| stats.exp.change_by(xp));
            }
        }
    }
}
