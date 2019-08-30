#![deny(unsafe_code)]
#![feature(label_break_value, duration_float, euclidean_division)]

pub mod error;

// Reexports
pub use crate::error::Error;
pub use specs::{join::Join, saveload::Marker, Entity as EcsEntity, ReadStorage};

use common::{
    comp,
    msg::{ClientMsg, ClientState, RequestStateError, ServerError, ServerInfo, ServerMsg},
    net::PostBox,
    state::{State, Uid},
    terrain::{block::Block, chonk::ChonkMetrics, TerrainChunk, TerrainChunkSize},
    vol::VolSize,
    ChatType,
};
use hashbrown::HashMap;
use log::{info, log_enabled, warn};
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use uvth::{ThreadPool, ThreadPoolBuilder};
use vek::*;

const SERVER_TIMEOUT: Duration = Duration::from_secs(20);

pub enum Event {
    Chat {
        chat_type: ChatType,
        message: String,
    },
    Disconnect,
}

pub struct Client {
    client_state: ClientState,
    thread_pool: ThreadPool,
    pub server_info: ServerInfo,

    postbox: PostBox<ClientMsg, ServerMsg>,

    last_server_ping: Instant,
    last_ping_delta: f64,

    tick: u64,
    state: State,
    entity: EcsEntity,

    view_distance: Option<u32>,
    loaded_distance: Option<u32>,

    pending_chunks: HashMap<Vec2<i32>, Instant>,
}

impl Client {
    /// Create a new `Client`.
    #[allow(dead_code)]
    pub fn new<A: Into<SocketAddr>>(addr: A, view_distance: Option<u32>) -> Result<Self, Error> {
        let client_state = ClientState::Connected;
        let mut postbox = PostBox::to(addr)?;

        // Wait for initial sync
        let (mut state, entity, server_info) = match postbox.next_message() {
            Some(ServerMsg::InitialSync {
                ecs_state,
                entity_uid,
                server_info,
            }) => {
                // TODO: Voxygen should display this.
                if server_info.git_hash != common::util::GIT_HASH.to_string() {
                    log::warn!(
                        "Git hash mismatch between client and server: {} vs {}",
                        server_info.git_hash,
                        common::util::GIT_HASH
                    );
                }

                let state = State::from_state_package(ecs_state);
                let entity = state
                    .ecs()
                    .entity_from_uid(entity_uid)
                    .ok_or(Error::ServerWentMad)?;
                (state, entity, server_info)
            }
            Some(ServerMsg::Error(ServerError::TooManyPlayers)) => {
                return Err(Error::TooManyPlayers)
            }
            _ => return Err(Error::ServerWentMad),
        };

        postbox.send_message(ClientMsg::Ping);

        let mut thread_pool = ThreadPoolBuilder::new()
            .name("veloren-worker".into())
            .build();
        // We reduce the thread count by 1 to keep rendering smooth
        thread_pool.set_num_threads((num_cpus::get() - 1).max(1));

        // Set client-only components
        let _ = state
            .ecs_mut()
            .write_storage()
            .insert(entity, comp::AnimationInfo::default());

        Ok(Self {
            client_state,
            thread_pool,
            server_info,

            postbox,

            last_server_ping: Instant::now(),
            last_ping_delta: 0.0,

            tick: 0,
            state,
            entity,
            view_distance,
            loaded_distance: None,

            pending_chunks: HashMap::new(),
        })
    }

    #[allow(dead_code)]
    pub fn with_thread_pool(mut self, thread_pool: ThreadPool) -> Self {
        self.thread_pool = thread_pool;
        self
    }

    /// Request a state transition to `ClientState::Registered`.
    pub fn register(&mut self, player: comp::Player, password: String) -> Result<(), Error> {
        self.postbox
            .send_message(ClientMsg::Register { player, password });
        self.client_state = ClientState::Pending;
        loop {
            match self.postbox.next_message() {
                Some(ServerMsg::StateAnswer(Err((RequestStateError::Denied, _)))) => {
                    break Err(Error::InvalidAuth)
                }
                Some(ServerMsg::StateAnswer(Ok(ClientState::Registered))) => break Ok(()),
                _ => {}
            }
        }
    }

    /// Request a state transition to `ClientState::Character`.
    pub fn request_character(
        &mut self,
        name: String,
        body: comp::Body,
        main: Option<comp::item::Tool>,
    ) {
        self.postbox
            .send_message(ClientMsg::Character { name, body, main });
        self.client_state = ClientState::Pending;
    }

    /// Request a state transition to `ClientState::Character`.
    pub fn request_logout(&mut self) {
        self.postbox
            .send_message(ClientMsg::RequestState(ClientState::Connected));
        self.client_state = ClientState::Pending;
    }

    /// Request a state transition to `ClientState::Character`.
    pub fn request_remove_character(&mut self) {
        self.postbox
            .send_message(ClientMsg::RequestState(ClientState::Registered));
        self.client_state = ClientState::Pending;
    }

    pub fn set_view_distance(&mut self, view_distance: u32) {
        self.view_distance = Some(view_distance.max(1).min(25));
        self.postbox
            .send_message(ClientMsg::SetViewDistance(self.view_distance.unwrap()));
        // Can't fail
    }

    pub fn use_inventory_slot(&mut self, x: usize) {
        self.postbox.send_message(ClientMsg::UseInventorySlot(x))
    }

    pub fn swap_inventory_slots(&mut self, a: usize, b: usize) {
        self.postbox
            .send_message(ClientMsg::SwapInventorySlots(a, b))
    }

    pub fn drop_inventory_slot(&mut self, x: usize) {
        self.postbox.send_message(ClientMsg::DropInventorySlot(x))
    }

    pub fn pick_up(&mut self, entity: EcsEntity) {
        if let Some(uid) = self.state.ecs().read_storage::<Uid>().get(entity).copied() {
            self.postbox.send_message(ClientMsg::PickUp(uid.id()));
        }
    }

    pub fn view_distance(&self) -> Option<u32> {
        self.view_distance
    }

    pub fn loaded_distance(&self) -> Option<u32> {
        self.loaded_distance
    }

    pub fn current_chunk(&self) -> Option<Arc<TerrainChunk>> {
        let chunk_pos = Vec2::from(
            self.state
                .read_storage::<comp::Pos>()
                .get(self.entity)
                .cloned()?
                .0,
        )
        .map2(Vec2::from(TerrainChunkSize::SIZE), |e: f32, sz| {
            (e as u32).div_euclid(sz) as i32
        });

        self.state.terrain().get_key_arc(chunk_pos).cloned()
    }

    pub fn inventories(&self) -> ReadStorage<comp::Inventory> {
        self.state.read_storage()
    }

    /// Send a chat message to the server.
    #[allow(dead_code)]
    pub fn send_chat(&mut self, msg: String) {
        self.postbox.send_message(ClientMsg::chat(msg))
    }

    /// Remove all cached terrain
    #[allow(dead_code)]
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

    /// Execute a single client tick, handle input and update the game state by the given duration.
    #[allow(dead_code)]
    pub fn tick(
        &mut self,
        controller: comp::Controller,
        dt: Duration,
    ) -> Result<Vec<Event>, Error> {
        // This tick function is the centre of the Veloren universe. Most client-side things are
        // managed from here, and as such it's important that it stays organised. Please consult
        // the core developers before making significant changes to this code. Here is the
        // approximate order of things. Please update it as this code changes.
        //
        // 1) Collect input from the frontend, apply input effects to the state of the game
        // 2) Handle messages from the server
        // 3) Go through any events (timer-driven or otherwise) that need handling and apply them
        //    to the state of the game
        // 4) Perform a single LocalState tick (i.e: update the world and entities in the world)
        // 5) Go through the terrain update queue and apply all changes to the terrain
        // 6) Sync information to the server
        // 7) Finish the tick, passing actions of the main thread back to the frontend

        // 1) Handle input from frontend.
        // Pass character actions from frontend input to the player's entity.
        if let ClientState::Character | ClientState::Dead = self.client_state {
            self.state.write_component(self.entity, controller.clone());
            self.postbox.send_message(ClientMsg::Controller(controller));
        }

        // 2) Build up a list of events for this frame, to be passed to the frontend.
        let mut frontend_events = Vec::new();

        // Handle new messages from the server.
        frontend_events.append(&mut self.handle_new_messages()?);

        // 3)

        // 4) Tick the client's LocalState
        self.state.tick(dt);

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
            let mut all_loaded = true;
            'outer: for dist in 0..=view_distance as i32 {
                // Only iterate through chunks that need to be loaded for circular vd
                // The (dist - 2) explained:
                // -0.5 because a chunk is visible if its corner is within the view distance
                // -0.5 for being able to move to the corner of the current chunk
                // -1 because chunks are not meshed if they don't have all their neighbors
                //     (notice also that view_distance is decreased by 1)
                //     (this subtraction on vd is ommitted elsewhere in order to provide a buffer layer of loaded chunks)
                let top = if 2 * (dist - 2).max(0).pow(2) > (view_distance - 1).pow(2) as i32 {
                    ((view_distance - 1).pow(2) as f32 - (dist - 2).pow(2) as f32)
                        .sqrt()
                        .round() as i32
                        + 1
                } else {
                    dist
                };

                for i in -top..=top {
                    let keys = [
                        chunk_pos + Vec2::new(dist, i),
                        chunk_pos + Vec2::new(i, dist),
                        chunk_pos + Vec2::new(-dist, i),
                        chunk_pos + Vec2::new(i, -dist),
                    ];

                    for key in keys.iter() {
                        if self.state.terrain().get_key(*key).is_none() {
                            if !self.pending_chunks.contains_key(key) {
                                if self.pending_chunks.len() < 4 {
                                    self.postbox
                                        .send_message(ClientMsg::TerrainChunkRequest { key: *key });
                                    self.pending_chunks.insert(*key, Instant::now());
                                } else {
                                    break 'outer;
                                }
                            }

                            all_loaded = false;
                        }
                    }
                }

                if all_loaded {
                    self.loaded_distance = Some((dist - 1).max(0) as u32);
                }
            }

            // If chunks are taking too long, assume they're no longer pending.
            let now = Instant::now();
            self.pending_chunks
                .retain(|_, created| now.duration_since(*created) < Duration::from_secs(3));
        }

        // Send a ping to the server once every second
        if Instant::now().duration_since(self.last_server_ping) > Duration::from_secs(1) {
            self.postbox.send_message(ClientMsg::Ping);
            self.last_server_ping = Instant::now();
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

        // Output debug metrics
        if log_enabled!(log::Level::Info) && self.tick % 600 == 0 {
            let metrics = self
                .state
                .terrain()
                .iter()
                .fold(ChonkMetrics::default(), |a, (_, c)| a + c.get_metrics());
            info!("{:?}", metrics);
        }

        // 7) Finish the tick, pass control back to the frontend.

        self.tick += 1;
        Ok(frontend_events)
    }

    /// Clean up the client after a tick.
    #[allow(dead_code)]
    pub fn cleanup(&mut self) {
        // Cleanup the local state
        self.state.cleanup();
    }

    /// Handle new server messages.
    fn handle_new_messages(&mut self) -> Result<Vec<Event>, Error> {
        let mut frontend_events = Vec::new();

        let new_msgs = self.postbox.new_messages();

        if new_msgs.len() > 0 {
            for msg in new_msgs {
                match msg {
                    ServerMsg::Error(e) => match e {
                        ServerError::TooManyPlayers => return Err(Error::ServerWentMad),
                        ServerError::InvalidAuth => return Err(Error::InvalidAuth),
                        //TODO: ServerError::InvalidAlias => return Err(Error::InvalidAlias),
                    },
                    ServerMsg::Shutdown => return Err(Error::ServerShutdown),
                    ServerMsg::InitialSync { .. } => return Err(Error::ServerWentMad),
                    ServerMsg::Ping => self.postbox.send_message(ClientMsg::Pong),
                    ServerMsg::Pong => {
                        self.last_ping_delta = Instant::now()
                            .duration_since(self.last_server_ping)
                            .as_secs_f64()
                    }
                    ServerMsg::ChatMsg { chat_type, message } => {
                        frontend_events.push(Event::Chat { chat_type, message })
                    }
                    ServerMsg::SetPlayerEntity(uid) => {
                        self.entity = self.state.ecs().entity_from_uid(uid).unwrap()
                    } // TODO: Don't unwrap here!
                    ServerMsg::EcsSync(sync_package) => {
                        self.state.ecs_mut().sync_with_package(sync_package)
                    }
                    ServerMsg::EntityPos { entity, pos } => {
                        if let Some(entity) = self.state.ecs().entity_from_uid(entity) {
                            self.state.write_component(entity, pos);
                        }
                    }
                    ServerMsg::EntityVel { entity, vel } => {
                        if let Some(entity) = self.state.ecs().entity_from_uid(entity) {
                            self.state.write_component(entity, vel);
                        }
                    }
                    ServerMsg::EntityOri { entity, ori } => {
                        if let Some(entity) = self.state.ecs().entity_from_uid(entity) {
                            self.state.write_component(entity, ori);
                        }
                    }
                    ServerMsg::EntityCharacterState {
                        entity,
                        character_state,
                    } => {
                        if let Some(entity) = self.state.ecs().entity_from_uid(entity) {
                            self.state.write_component(entity, character_state);
                        }
                    }
                    ServerMsg::InventoryUpdate(inventory) => {
                        self.state.write_component(self.entity, inventory)
                    }
                    ServerMsg::TerrainChunkUpdate { key, chunk } => {
                        self.state.insert_chunk(key, *chunk);
                        self.pending_chunks.remove(&key);
                    }
                    ServerMsg::TerrainBlockUpdates(mut blocks) => blocks
                        .drain()
                        .for_each(|(pos, block)| self.state.set_block(pos, block)),
                    ServerMsg::StateAnswer(Ok(state)) => {
                        self.client_state = state;
                    }
                    ServerMsg::StateAnswer(Err((error, state))) => {
                        if error == RequestStateError::Denied {
                            warn!("Connection denied!");
                            return Err(Error::InvalidAuth);
                        }
                        warn!(
                            "StateAnswer: {:?}. Server thinks client is in state {:?}.",
                            error, state
                        );
                    }
                    ServerMsg::ForceState(state) => {
                        self.client_state = state;
                    }
                    ServerMsg::Disconnect => {
                        frontend_events.push(Event::Disconnect);
                    }
                }
            }
        } else if let Some(err) = self.postbox.error() {
            return Err(err.into());
        // We regularily ping in the tick method
        } else if Instant::now().duration_since(self.last_server_ping) > SERVER_TIMEOUT {
            return Err(Error::ServerTimeout);
        }
        Ok(frontend_events)
    }

    /// Get the player's entity.
    #[allow(dead_code)]
    pub fn entity(&self) -> EcsEntity {
        self.entity
    }

    /// Get the client state
    #[allow(dead_code)]
    pub fn get_client_state(&self) -> ClientState {
        self.client_state
    }

    /// Get the current tick number.
    #[allow(dead_code)]
    pub fn get_tick(&self) -> u64 {
        self.tick
    }

    #[allow(dead_code)]
    pub fn get_ping_ms(&self) -> f64 {
        self.last_ping_delta * 1000.0
    }

    /// Get a reference to the client's worker thread pool. This pool should be used for any
    /// computationally expensive operations that run outside of the main thread (i.e., threads that
    /// block on I/O operations are exempt).
    #[allow(dead_code)]
    pub fn thread_pool(&self) -> &ThreadPool {
        &self.thread_pool
    }

    /// Get a reference to the client's game state.
    #[allow(dead_code)]
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Get a mutable reference to the client's game state.
    #[allow(dead_code)]
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

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
    fn drop(&mut self) {
        self.postbox.send_message(ClientMsg::Disconnect);
    }
}
