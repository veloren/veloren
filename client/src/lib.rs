#![deny(unsafe_code)]
#![feature(label_break_value)]

pub mod error;

// Reexports
pub use crate::error::Error;
pub use authc::AuthClientError;
pub use specs::{
    join::Join,
    saveload::{Marker, MarkerAllocator},
    Builder, DispatcherBuilder, Entity as EcsEntity, ReadStorage, WorldExt,
};

use byteorder::{ByteOrder, LittleEndian};
use common::{
    comp::{
        self, ControlEvent, Controller, ControllerInputs, InventoryManip, InventoryUpdateEvent,
    },
    event::{EventBus, SfxEvent, SfxEventItem},
    msg::{
        validate_chat_msg, ChatMsgValidationError, ClientMsg, ClientState, PlayerListUpdate,
        RegisterError, RequestStateError, ServerInfo, ServerMsg, MAX_BYTES_CHAT_MSG,
    },
    net::PostBox,
    state::State,
    sync::{Uid, UidAllocator, WorldSyncExt},
    terrain::{block::Block, TerrainChunk, TerrainChunkSize},
    vol::RectVolSize,
    ChatType,
};
use hashbrown::HashMap;
use image::DynamicImage;
use log::warn;
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use uvth::{ThreadPool, ThreadPoolBuilder};
use vek::*;

// The duration of network inactivity until the player is kicked
// @TODO: in the future, this should be configurable on the server
// and be provided to the client
const SERVER_TIMEOUT: f64 = 20.0;

// After this duration has elapsed, the user will begin getting kick warnings in
// their chat window
const SERVER_TIMEOUT_GRACE_PERIOD: f64 = 14.0;

pub enum Event {
    Chat {
        chat_type: ChatType,
        message: String,
    },
    Disconnect,
    DisconnectionNotification(u64),
}

pub struct Client {
    client_state: ClientState,
    thread_pool: ThreadPool,
    pub server_info: ServerInfo,
    pub world_map: (Arc<DynamicImage>, Vec2<u32>),
    pub player_list: HashMap<u64, String>,

    postbox: PostBox<ClientMsg, ServerMsg>,

    last_server_ping: f64,
    last_server_pong: f64,
    last_ping_delta: f64,

    tick: u64,
    state: State,
    entity: EcsEntity,

    view_distance: Option<u32>,
    // TODO: move into voxygen
    loaded_distance: f32,

    pending_chunks: HashMap<Vec2<i32>, Instant>,
}

impl Client {
    /// Create a new `Client`.
    pub fn new<A: Into<SocketAddr>>(addr: A, view_distance: Option<u32>) -> Result<Self, Error> {
        let client_state = ClientState::Connected;
        let mut postbox = PostBox::to(addr)?;

        // Wait for initial sync
        let (state, entity, server_info, world_map) = match postbox.next_message()? {
            ServerMsg::InitialSync {
                entity_package,
                server_info,
                time_of_day,
                world_map: (map_size, world_map),
            } => {
                // TODO: Display that versions don't match in Voxygen
                if server_info.git_hash != common::util::GIT_HASH.to_string() {
                    log::warn!(
                        "Server is running {}[{}], you are running {}[{}], versions might be \
                         incompatible!",
                        server_info.git_hash,
                        server_info.git_date,
                        common::util::GIT_HASH.to_string(),
                        common::util::GIT_DATE.to_string(),
                    );
                }

                log::debug!("Auth Server: {:?}", server_info.auth_provider);

                // Initialize `State`
                let mut state = State::default();
                let entity = state.ecs_mut().apply_entity_package(entity_package);
                *state.ecs_mut().write_resource() = time_of_day;

                assert_eq!(world_map.len(), (map_size.x * map_size.y) as usize);
                let mut world_map_raw = vec![0u8; 4 * world_map.len()/*map_size.x * map_size.y*/];
                LittleEndian::write_u32_into(&world_map, &mut world_map_raw);
                log::debug!("Preparing image...");
                let world_map = Arc::new(
                    image::DynamicImage::ImageRgba8({
                        // Should not fail if the dimensions are correct.
                        let world_map =
                            image::ImageBuffer::from_raw(map_size.x, map_size.y, world_map_raw);
                        world_map.ok_or(Error::Other("Server sent a bad world map image".into()))?
                    })
                    // Flip the image, since Voxygen uses an orientation where rotation from
                    // positive x axis to positive y axis is counterclockwise around the z axis.
                    .flipv(),
                );
                log::debug!("Done preparing image...");

                (state, entity, server_info, (world_map, map_size))
            },
            ServerMsg::TooManyPlayers => return Err(Error::TooManyPlayers),
            _ => return Err(Error::ServerWentMad),
        };

        postbox.send_message(ClientMsg::Ping);

        let mut thread_pool = ThreadPoolBuilder::new()
            .name("veloren-worker".into())
            .build();
        // We reduce the thread count by 1 to keep rendering smooth
        thread_pool.set_num_threads((num_cpus::get() - 1).max(1));

        Ok(Self {
            client_state,
            thread_pool,
            server_info,
            world_map,
            player_list: HashMap::new(),

            postbox,

            last_server_ping: 0.0,
            last_server_pong: 0.0,
            last_ping_delta: 0.0,

            tick: 0,
            state,
            entity,
            view_distance,
            loaded_distance: 0.0,

            pending_chunks: HashMap::new(),
        })
    }

    pub fn with_thread_pool(mut self, thread_pool: ThreadPool) -> Self {
        self.thread_pool = thread_pool;
        self
    }

    /// Request a state transition to `ClientState::Registered`.
    pub fn register(
        &mut self,
        username: String,
        password: String,
        mut auth_trusted: impl FnMut(&str) -> bool,
    ) -> Result<(), Error> {
        // Authentication
        let token_or_username = self.server_info.auth_provider.as_ref().map(|addr|
                // Query whether this is a trusted auth server
                if auth_trusted(&addr) {
                    Ok(authc::AuthClient::new(addr)
                        .sign_in(&username, &password)?
                        .serialize())
                } else {
                    Err(Error::AuthServerNotTrusted)
                }
        ).unwrap_or(Ok(username))?;

        self.postbox.send_message(ClientMsg::Register {
            view_distance: self.view_distance,
            token_or_username,
        });
        self.client_state = ClientState::Pending;

        loop {
            match self.postbox.next_message()? {
                ServerMsg::StateAnswer(Err((RequestStateError::RegisterDenied(err), state))) => {
                    self.client_state = state;
                    break Err(match err {
                        RegisterError::AlreadyLoggedIn => Error::AlreadyLoggedIn,
                        RegisterError::AuthError(err) => Error::AuthErr(err),
                    });
                },
                ServerMsg::StateAnswer(Ok(ClientState::Registered)) => break Ok(()),
                _ => {},
            }
        }
    }

    /// Request a state transition to `ClientState::Character`.
    pub fn request_character(&mut self, name: String, body: comp::Body, main: Option<String>) {
        self.postbox
            .send_message(ClientMsg::Character { name, body, main });
        self.client_state = ClientState::Pending;
    }

    /// Send disconnect message to the server
    pub fn request_logout(&mut self) { self.postbox.send_message(ClientMsg::Disconnect); }

    /// Request a state transition to `ClientState::Registered` from an ingame
    /// state.
    pub fn request_remove_character(&mut self) {
        self.postbox.send_message(ClientMsg::ExitIngame);
        self.client_state = ClientState::Pending;
    }

    pub fn set_view_distance(&mut self, view_distance: u32) {
        self.view_distance = Some(view_distance.max(1).min(65));
        self.postbox
            .send_message(ClientMsg::SetViewDistance(self.view_distance.unwrap()));
        // Can't fail
    }

    pub fn use_inventory_slot(&mut self, slot: usize) {
        self.postbox
            .send_message(ClientMsg::ControlEvent(ControlEvent::InventoryManip(
                InventoryManip::Use(slot),
            )));
    }

    pub fn swap_inventory_slots(&mut self, a: usize, b: usize) {
        self.postbox
            .send_message(ClientMsg::ControlEvent(ControlEvent::InventoryManip(
                InventoryManip::Swap(a, b),
            )));
    }

    pub fn drop_inventory_slot(&mut self, slot: usize) {
        self.postbox
            .send_message(ClientMsg::ControlEvent(ControlEvent::InventoryManip(
                InventoryManip::Drop(slot),
            )));
    }

    pub fn pick_up(&mut self, entity: EcsEntity) {
        if let Some(uid) = self.state.ecs().read_storage::<Uid>().get(entity).copied() {
            self.postbox
                .send_message(ClientMsg::ControlEvent(ControlEvent::InventoryManip(
                    InventoryManip::Pickup(uid),
                )));
        }
    }

    pub fn is_mounted(&self) -> bool {
        self.state
            .ecs()
            .read_storage::<comp::Mounting>()
            .get(self.entity)
            .is_some()
    }

    pub fn mount(&mut self, entity: EcsEntity) {
        if let Some(uid) = self.state.ecs().read_storage::<Uid>().get(entity).copied() {
            self.postbox
                .send_message(ClientMsg::ControlEvent(ControlEvent::Mount(uid)));
        }
    }

    pub fn unmount(&mut self) {
        self.postbox
            .send_message(ClientMsg::ControlEvent(ControlEvent::Unmount));
    }

    pub fn view_distance(&self) -> Option<u32> { self.view_distance }

    pub fn loaded_distance(&self) -> f32 { self.loaded_distance }

    pub fn current_chunk(&self) -> Option<Arc<TerrainChunk>> {
        let chunk_pos = Vec2::from(
            self.state
                .read_storage::<comp::Pos>()
                .get(self.entity)
                .cloned()?
                .0,
        )
        .map2(TerrainChunkSize::RECT_SIZE, |e: f32, sz| {
            (e as u32).div_euclid(sz) as i32
        });

        self.state.terrain().get_key_arc(chunk_pos).cloned()
    }

    pub fn inventories(&self) -> ReadStorage<comp::Inventory> { self.state.read_storage() }

    /// Send a chat message to the server.
    pub fn send_chat(&mut self, message: String) {
        match validate_chat_msg(&message) {
            Ok(()) => self.postbox.send_message(ClientMsg::ChatMsg { message }),
            Err(ChatMsgValidationError::TooLong) => log::warn!(
                "Attempted to send a message that's too long (Over {} bytes)",
                MAX_BYTES_CHAT_MSG
            ),
        }
    }

    /// Remove all cached terrain
    pub fn clear_terrain(&mut self) {
        self.state.clear_terrain();
        self.pending_chunks.clear();
    }

    pub fn place_block(&mut self, pos: Vec3<i32>, block: Block) {
        self.postbox.send_message(ClientMsg::PlaceBlock(pos, block));
    }

    pub fn remove_block(&mut self, pos: Vec3<i32>) {
        self.postbox.send_message(ClientMsg::BreakBlock(pos));
    }

    pub fn collect_block(&mut self, pos: Vec3<i32>) {
        self.postbox
            .send_message(ClientMsg::ControlEvent(ControlEvent::InventoryManip(
                InventoryManip::Collect(pos),
            )));
    }

    /// Execute a single client tick, handle input and update the game state by
    /// the given duration.
    pub fn tick(
        &mut self,
        inputs: ControllerInputs,
        dt: Duration,
        add_foreign_systems: impl Fn(&mut DispatcherBuilder),
    ) -> Result<Vec<Event>, Error> {
        // This tick function is the centre of the Veloren universe. Most client-side
        // things are managed from here, and as such it's important that it
        // stays organised. Please consult the core developers before making
        // significant changes to this code. Here is the approximate order of
        // things. Please update it as this code changes.
        //
        // 1) Collect input from the frontend, apply input effects to the state
        //    of the game
        // 2) Handle messages from the server
        // 3) Go through any events (timer-driven or otherwise) that need handling
        //    and apply them to the state of the game
        // 4) Perform a single LocalState tick (i.e: update the world and entities
        //    in the world)
        // 5) Go through the terrain update queue and apply all changes
        //    to the terrain
        // 6) Sync information to the server
        // 7) Finish the tick, passing actions of the main thread back
        //    to the frontend

        // 1) Handle input from frontend.
        // Pass character actions from frontend input to the player's entity.
        if let ClientState::Character = self.client_state {
            self.state.write_component(self.entity, Controller {
                inputs: inputs.clone(),
                events: Vec::new(),
            });
            self.postbox
                .send_message(ClientMsg::ControllerInputs(inputs));
        }

        // 2) Build up a list of events for this frame, to be passed to the frontend.
        let mut frontend_events = Vec::new();

        // Prepare for new events
        {
            let ecs = self.state.ecs();
            for (entity, _) in (&ecs.entities(), &ecs.read_storage::<comp::Body>()).join() {
                let mut last_character_states =
                    ecs.write_storage::<comp::Last<comp::CharacterState>>();
                if let Some(client_character_state) =
                    ecs.read_storage::<comp::CharacterState>().get(entity)
                {
                    if last_character_states
                        .get(entity)
                        .map(|&l| !client_character_state.is_same_state(&l.0))
                        .unwrap_or(true)
                    {
                        let _ = last_character_states
                            .insert(entity, comp::Last(*client_character_state));
                    }
                }
            }
        }

        // Handle new messages from the server.
        frontend_events.append(&mut self.handle_new_messages()?);

        // 3) Update client local data

        // 4) Tick the client's LocalState
        self.state.tick(dt, add_foreign_systems, true);

        // 5) Terrain
        let pos = self
            .state
            .read_storage::<comp::Pos>()
            .get(self.entity)
            .cloned();
        if let (Some(pos), Some(view_distance)) = (pos, self.view_distance) {
            let chunk_pos = self.state.terrain().pos_key(pos.0.map(|e| e as i32));

            // Remove chunks that are too far from the player.
            let mut chunks_to_remove = Vec::new();
            self.state.terrain().iter().for_each(|(key, _)| {
                // Subtract 2 from the offset before computing squared magnitude
                // 1 for the chunks needed bordering other chunks for meshing
                // 1 as a buffer so that if the player moves back in that direction the chunks
                //   don't need to be reloaded
                if (chunk_pos - key)
                    .map(|e: i32| (e.abs() as u32).checked_sub(2).unwrap_or(0))
                    .magnitude_squared()
                    > view_distance.pow(2)
                {
                    chunks_to_remove.push(key);
                }
            });
            for key in chunks_to_remove {
                self.state.remove_chunk(key);
            }

            // Request chunks from the server.
            self.loaded_distance = ((view_distance * TerrainChunkSize::RECT_SIZE.x) as f32).powi(2);
            // +1 so we can find a chunk that's outside the vd for better fog
            for dist in 0..view_distance as i32 + 1 {
                // Only iterate through chunks that need to be loaded for circular vd
                // The (dist - 2) explained:
                // -0.5 because a chunk is visible if its corner is within the view distance
                // -0.5 for being able to move to the corner of the current chunk
                // -1 because chunks are not meshed if they don't have all their neighbors
                //     (notice also that view_distance is decreased by 1)
                //     (this subtraction on vd is ommitted elsewhere in order to provide
                //     a buffer layer of loaded chunks)
                let top = if 2 * (dist - 2).max(0).pow(2) > (view_distance - 1).pow(2) as i32 {
                    ((view_distance - 1).pow(2) as f32 - (dist - 2).pow(2) as f32)
                        .sqrt()
                        .round() as i32
                        + 1
                } else {
                    dist
                };

                let mut skip_mode = false;
                for i in -top..top + 1 {
                    let keys = [
                        chunk_pos + Vec2::new(dist, i),
                        chunk_pos + Vec2::new(i, dist),
                        chunk_pos + Vec2::new(-dist, i),
                        chunk_pos + Vec2::new(i, -dist),
                    ];

                    for key in keys.iter() {
                        if self.state.terrain().get_key(*key).is_none() {
                            if !skip_mode && !self.pending_chunks.contains_key(key) {
                                if self.pending_chunks.len() < 4 {
                                    self.postbox
                                        .send_message(ClientMsg::TerrainChunkRequest { key: *key });
                                    self.pending_chunks.insert(*key, Instant::now());
                                } else {
                                    skip_mode = true;
                                }
                            }

                            let dist_to_player =
                                (self.state.terrain().key_pos(*key).map(|x| x as f32)
                                    + TerrainChunkSize::RECT_SIZE.map(|x| x as f32) / 2.0)
                                    .distance_squared(pos.0.into());

                            if dist_to_player < self.loaded_distance {
                                self.loaded_distance = dist_to_player;
                            }
                        }
                    }
                }
            }
            self.loaded_distance = self.loaded_distance.sqrt()
                - ((TerrainChunkSize::RECT_SIZE.x as f32 / 2.0).powi(2)
                    + (TerrainChunkSize::RECT_SIZE.y as f32 / 2.0).powi(2))
                .sqrt();

            // If chunks are taking too long, assume they're no longer pending.
            let now = Instant::now();
            self.pending_chunks
                .retain(|_, created| now.duration_since(*created) < Duration::from_secs(3));
        }

        // Send a ping to the server once every second
        if self.state.get_time() - self.last_server_ping > 1. {
            self.postbox.send_message(ClientMsg::Ping);
            self.last_server_ping = self.state.get_time();
        }

        // 6) Update the server about the player's physics attributes.
        if let ClientState::Character = self.client_state {
            if let (Some(pos), Some(vel), Some(ori)) = (
                self.state.read_storage().get(self.entity).cloned(),
                self.state.read_storage().get(self.entity).cloned(),
                self.state.read_storage().get(self.entity).cloned(),
            ) {
                self.postbox
                    .send_message(ClientMsg::PlayerPhysics { pos, vel, ori });
            }
        }

        /*
        // Output debug metrics
        if log_enabled!(log::Level::Info) && self.tick % 600 == 0 {
            let metrics = self
                .state
                .terrain()
                .iter()
                .fold(ChonkMetrics::default(), |a, (_, c)| a + c.get_metrics());
            info!("{:?}", metrics);
        }
        */

        // 7) Finish the tick, pass control back to the frontend.
        self.tick += 1;
        Ok(frontend_events)
    }

    /// Clean up the client after a tick.
    pub fn cleanup(&mut self) {
        // Cleanup the local state
        self.state.cleanup();
    }

    /// Handle new server messages.
    fn handle_new_messages(&mut self) -> Result<Vec<Event>, Error> {
        let mut frontend_events = Vec::new();

        // Check that we have an valid connection.
        // Use the last ping time as a 1s rate limiter, we only notify the user once per
        // second
        if self.state.get_time() - self.last_server_ping > 1. {
            let duration_since_last_pong = self.state.get_time() - self.last_server_pong;

            // Dispatch a notification to the HUD warning they will be kicked in {n} seconds
            if duration_since_last_pong >= SERVER_TIMEOUT_GRACE_PERIOD {
                if self.state.get_time() - duration_since_last_pong > 0. {
                    frontend_events.push(Event::DisconnectionNotification(
                        (self.state.get_time() - duration_since_last_pong).round() as u64,
                    ));
                }
            }
        }

        let new_msgs = self.postbox.new_messages();

        if new_msgs.len() > 0 {
            for msg in new_msgs {
                match msg {
                    ServerMsg::TooManyPlayers => {
                        return Err(Error::ServerWentMad);
                    },
                    ServerMsg::Shutdown => return Err(Error::ServerShutdown),
                    ServerMsg::InitialSync { .. } => return Err(Error::ServerWentMad),
                    ServerMsg::PlayerListUpdate(PlayerListUpdate::Init(list)) => {
                        self.player_list = list
                    },
                    ServerMsg::PlayerListUpdate(PlayerListUpdate::Add(uid, name)) => {
                        if let Some(old_name) = self.player_list.insert(uid, name.clone()) {
                            warn!(
                                "Received msg to insert {} with uid {} into the player list but \
                                 there was already an entry for {} with the same uid that was \
                                 overwritten!",
                                name, uid, old_name
                            );
                        }
                    },
                    ServerMsg::PlayerListUpdate(PlayerListUpdate::Remove(uid)) => {
                        if self.player_list.remove(&uid).is_none() {
                            warn!(
                                "Received msg to remove uid {} from the player list by they \
                                 weren't in the list!",
                                uid
                            );
                        }
                    },
                    ServerMsg::PlayerListUpdate(PlayerListUpdate::Alias(uid, new_name)) => {
                        if let Some(name) = self.player_list.get_mut(&uid) {
                            *name = new_name;
                        } else {
                            warn!(
                                "Received msg to alias player with uid {} to {} but this uid is \
                                 not in the player list",
                                uid, new_name
                            );
                        }
                    },

                    ServerMsg::Ping => self.postbox.send_message(ClientMsg::Pong),
                    ServerMsg::Pong => {
                        self.last_server_pong = self.state.get_time();

                        self.last_ping_delta =
                            (self.state.get_time() - self.last_server_ping).round();
                    },
                    ServerMsg::ChatMsg { message, chat_type } => {
                        frontend_events.push(Event::Chat { message, chat_type })
                    },
                    ServerMsg::SetPlayerEntity(uid) => {
                        if let Some(entity) = self.state.ecs().entity_from_uid(uid) {
                            self.entity = entity;
                        } else {
                            return Err(Error::Other("Failed to find entity from uid.".to_owned()));
                        }
                    },
                    ServerMsg::TimeOfDay(time_of_day) => {
                        *self.state.ecs_mut().write_resource() = time_of_day;
                    },
                    ServerMsg::EcsSync(sync_package) => {
                        self.state.ecs_mut().apply_sync_package(sync_package);
                    },
                    ServerMsg::CreateEntity(entity_package) => {
                        self.state.ecs_mut().apply_entity_package(entity_package);
                    },
                    ServerMsg::DeleteEntity(entity) => {
                        if self
                            .state
                            .read_component_cloned::<Uid>(self.entity)
                            .map(|u| u.into())
                            != Some(entity)
                        {
                            self.state
                                .ecs_mut()
                                .delete_entity_and_clear_from_uid_allocator(entity);
                        }
                    },
                    // Cleanup for when the client goes back to the `Registered` state
                    ServerMsg::ExitIngameCleanup => {
                        // Get client entity Uid
                        let client_uid = self
                            .state
                            .read_component_cloned::<Uid>(self.entity)
                            .map(|u| u.into())
                            .expect("Client doesn't have a Uid!!!");
                        // Clear ecs of all entities
                        self.state.ecs_mut().delete_all();
                        self.state.ecs_mut().maintain();
                        self.state.ecs_mut().insert(UidAllocator::default());
                        // Recreate client entity with Uid
                        let entity_builder = self.state.ecs_mut().create_entity();
                        let uid = entity_builder
                            .world
                            .write_resource::<UidAllocator>()
                            .allocate(entity_builder.entity, Some(client_uid));
                        self.entity = entity_builder.with(uid).build();
                    },
                    ServerMsg::EntityPos { entity, pos } => {
                        if let Some(entity) = self.state.ecs().entity_from_uid(entity) {
                            self.state.write_component(entity, pos);
                        }
                    },
                    ServerMsg::EntityVel { entity, vel } => {
                        if let Some(entity) = self.state.ecs().entity_from_uid(entity) {
                            self.state.write_component(entity, vel);
                        }
                    },
                    ServerMsg::EntityOri { entity, ori } => {
                        if let Some(entity) = self.state.ecs().entity_from_uid(entity) {
                            self.state.write_component(entity, ori);
                        }
                    },
                    ServerMsg::EntityCharacterState {
                        entity,
                        character_state,
                    } => {
                        if let Some(entity) = self.state.ecs().entity_from_uid(entity) {
                            self.state.write_component(entity, character_state);
                        }
                    },
                    ServerMsg::InventoryUpdate(inventory, event) => {
                        match event {
                            InventoryUpdateEvent::CollectFailed => {
                                frontend_events.push(Event::Chat {
                                    message: String::from(
                                        "Failed to collect item. Your inventory may be full!",
                                    ),
                                    chat_type: ChatType::Meta,
                                })
                            },
                            _ => {
                                self.state.write_component(self.entity, inventory);
                            },
                        }

                        self.state
                            .ecs()
                            .read_resource::<EventBus<SfxEventItem>>()
                            .emitter()
                            .emit(SfxEventItem::at_player_position(SfxEvent::Inventory(event)));
                    },
                    ServerMsg::TerrainChunkUpdate { key, chunk } => {
                        if let Ok(chunk) = chunk {
                            self.state.insert_chunk(key, *chunk);
                        }
                        self.pending_chunks.remove(&key);
                    },
                    ServerMsg::TerrainBlockUpdates(mut blocks) => {
                        blocks.drain().for_each(|(pos, block)| {
                            self.state.set_block(pos, block);
                        });
                    },
                    ServerMsg::StateAnswer(Ok(state)) => {
                        self.client_state = state;
                    },
                    ServerMsg::StateAnswer(Err((error, state))) => {
                        warn!(
                            "StateAnswer: {:?}. Server thinks client is in state {:?}.",
                            error, state
                        );
                    },
                    ServerMsg::Disconnect => {
                        frontend_events.push(Event::Disconnect);
                        self.postbox.send_message(ClientMsg::Terminate);
                    },
                }
            }
        } else if let Some(err) = self.postbox.error() {
            return Err(err.into());
        // We regularily ping in the tick method
        } else if self.state.get_time() - self.last_server_pong > SERVER_TIMEOUT {
            return Err(Error::ServerTimeout);
        }
        Ok(frontend_events)
    }

    /// Get the player's entity.
    pub fn entity(&self) -> EcsEntity { self.entity }

    /// Get the client state
    pub fn get_client_state(&self) -> ClientState { self.client_state }

    /// Get the current tick number.
    pub fn get_tick(&self) -> u64 { self.tick }

    pub fn get_ping_ms(&self) -> f64 { self.last_ping_delta * 1000.0 }

    /// Get a reference to the client's worker thread pool. This pool should be
    /// used for any computationally expensive operations that run outside
    /// of the main thread (i.e., threads that block on I/O operations are
    /// exempt).
    pub fn thread_pool(&self) -> &ThreadPool { &self.thread_pool }

    /// Get a reference to the client's game state.
    pub fn state(&self) -> &State { &self.state }

    /// Get a mutable reference to the client's game state.
    pub fn state_mut(&mut self) -> &mut State { &mut self.state }

    /// Get a vector of all the players on the server
    pub fn get_players(&mut self) -> Vec<comp::Player> {
        // TODO: Don't clone players.
        self.state
            .ecs()
            .read_storage::<comp::Player>()
            .join()
            .cloned()
            .collect()
    }
}

impl Drop for Client {
    fn drop(&mut self) { self.postbox.send_message(ClientMsg::Disconnect); }
}
