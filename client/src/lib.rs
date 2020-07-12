#![deny(unsafe_code)]
#![feature(label_break_value, option_zip)]

pub mod cmd;
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
    character::CharacterItem,
    comp::{
        self, ControlAction, ControlEvent, Controller, ControllerInputs, GroupManip,
        InventoryManip, InventoryUpdateEvent,
    },
    msg::{
        validate_chat_msg, ChatMsgValidationError, ClientMsg, ClientState, Notification,
        PlayerInfo, PlayerListUpdate, RegisterError, RequestStateError, ServerInfo, ServerMsg,
        MAX_BYTES_CHAT_MSG,
    },
    recipe::RecipeBook,
    state::State,
    sync::{Uid, UidAllocator, WorldSyncExt},
    terrain::{block::Block, TerrainChunk, TerrainChunkSize},
    vol::RectVolSize,
};
use futures_executor::block_on;
use futures_timer::Delay;
use futures_util::{select, FutureExt};
use hashbrown::{HashMap, HashSet};
use image::DynamicImage;
use network::{
    Network, Participant, Pid, ProtocolAddr, Stream, PROMISES_CONSISTENCY, PROMISES_ORDERED,
};
use std::{
    collections::VecDeque,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::{debug, error, trace, warn};
use uvth::{ThreadPool, ThreadPoolBuilder};
use vek::*;

// The duration of network inactivity until the player is kicked
// @TODO: in the future, this should be configurable on the server
// and be provided to the client
const SERVER_TIMEOUT: f64 = 20.0;

// After this duration has elapsed, the user will begin getting kick warnings in
// their chat window
const SERVER_TIMEOUT_GRACE_PERIOD: f64 = 14.0;
const PING_ROLLING_AVERAGE_SECS: usize = 10;

pub enum Event {
    Chat(comp::ChatMsg),
    Disconnect,
    DisconnectionNotification(u64),
    InventoryUpdated(InventoryUpdateEvent),
    Notification(Notification),
    SetViewDistance(u32),
}

pub struct Client {
    client_state: ClientState,
    thread_pool: ThreadPool,
    pub server_info: ServerInfo,
    pub world_map: (Arc<DynamicImage>, Vec2<u32>),
    pub player_list: HashMap<Uid, PlayerInfo>,
    pub group_members: HashSet<Uid>,
    pub character_list: CharacterList,
    pub active_character_id: Option<i32>,
    recipe_book: RecipeBook,
    available_recipes: HashSet<String>,

    group_invite: Option<Uid>,
    group_leader: Option<Uid>,

    _network: Network,
    participant: Option<Participant>,
    singleton_stream: Stream,

    last_server_ping: f64,
    last_server_pong: f64,
    last_ping_delta: f64,
    ping_deltas: VecDeque<f64>,

    tick: u64,
    state: State,
    entity: EcsEntity,

    view_distance: Option<u32>,
    // TODO: move into voxygen
    loaded_distance: f32,

    pending_chunks: HashMap<Vec2<i32>, Instant>,
}

/// Holds data related to the current players characters, as well as some
/// additional state to handle UI.
#[derive(Default)]
pub struct CharacterList {
    pub characters: Vec<CharacterItem>,
    pub loading: bool,
    pub error: Option<String>,
}

impl Client {
    /// Create a new `Client`.
    pub fn new<A: Into<SocketAddr>>(addr: A, view_distance: Option<u32>) -> Result<Self, Error> {
        let client_state = ClientState::Connected;

        let mut thread_pool = ThreadPoolBuilder::new()
            .name("veloren-worker".into())
            .build();
        // We reduce the thread count by 1 to keep rendering smooth
        thread_pool.set_num_threads((num_cpus::get() - 1).max(1));

        let (network, f) = Network::new(Pid::new());
        thread_pool.execute(f);

        let participant = block_on(network.connect(ProtocolAddr::Tcp(addr.into())))?;
        let mut stream = block_on(participant.open(10, PROMISES_ORDERED | PROMISES_CONSISTENCY))?;

        // Wait for initial sync
        let (state, entity, server_info, world_map, recipe_book) = block_on(async {
            loop {
                match stream.recv().await? {
                    ServerMsg::InitialSync {
                        entity_package,
                        server_info,
                        time_of_day,
                        world_map: (map_size, world_map),
                        recipe_book,
                    } => {
                        // TODO: Display that versions don't match in Voxygen
                        if &server_info.git_hash != *common::util::GIT_HASH {
                            warn!(
                                "Server is running {}[{}], you are running {}[{}], versions might \
                                 be incompatible!",
                                server_info.git_hash,
                                server_info.git_date,
                                common::util::GIT_HASH.to_string(),
                                common::util::GIT_DATE.to_string(),
                            );
                        }

                        debug!("Auth Server: {:?}", server_info.auth_provider);

                        // Initialize `State`
                        let mut state = State::default();
                        // Client-only components
                        state
                            .ecs_mut()
                            .register::<comp::Last<comp::CharacterState>>();

                        let entity = state.ecs_mut().apply_entity_package(entity_package);
                        *state.ecs_mut().write_resource() = time_of_day;

                        assert_eq!(world_map.len(), (map_size.x * map_size.y) as usize);
                        let mut world_map_raw =
                            vec![0u8; 4 * world_map.len()/*map_size.x * map_size.y*/];
                        LittleEndian::write_u32_into(&world_map, &mut world_map_raw);
                        debug!("Preparing image...");
                        let world_map = Arc::new(
                            image::DynamicImage::ImageRgba8({
                                // Should not fail if the dimensions are correct.
                                let world_map =
                                    image::ImageBuffer::from_raw(map_size.x, map_size.y, world_map_raw);
                                world_map.ok_or_else(|| Error::Other("Server sent a bad world map image".into()))?
                            })
                                // Flip the image, since Voxygen uses an orientation where rotation from
                                // positive x axis to positive y axis is counterclockwise around the z axis.
                                .flipv(),
                        );
                        debug!("Done preparing image...");

                        break Ok((
                            state,
                            entity,
                            server_info,
                            (world_map, map_size),
                            recipe_book,
                        ));
                    },
                    ServerMsg::TooManyPlayers => break Err(Error::TooManyPlayers),
                    err => {
                        warn!("whoops, server mad {:?}, ignoring", err);
                    },
                }
            }
        })?;

        stream.send(ClientMsg::Ping)?;

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
            group_members: HashSet::new(),
            character_list: CharacterList::default(),
            active_character_id: None,
            recipe_book,
            available_recipes: HashSet::default(),

            group_invite: None,
            group_leader: None,

            _network: network,
            participant: Some(participant),
            singleton_stream: stream,

            last_server_ping: 0.0,
            last_server_pong: 0.0,
            last_ping_delta: 0.0,
            ping_deltas: VecDeque::new(),

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

        self.singleton_stream.send(ClientMsg::Register {
            view_distance: self.view_distance,
            token_or_username,
        })?;
        self.client_state = ClientState::Pending;

        block_on(async {
            loop {
                match self.singleton_stream.recv().await? {
                    ServerMsg::StateAnswer(Err((
                        RequestStateError::RegisterDenied(err),
                        state,
                    ))) => {
                        self.client_state = state;
                        break Err(match err {
                            RegisterError::AlreadyLoggedIn => Error::AlreadyLoggedIn,
                            RegisterError::AuthError(err) => Error::AuthErr(err),
                            RegisterError::InvalidCharacter => Error::InvalidCharacter,
                            RegisterError::NotOnWhitelist => Error::NotOnWhitelist,
                        });
                    },
                    ServerMsg::StateAnswer(Ok(ClientState::Registered)) => break Ok(()),
                    ignore => {
                        warn!(
                            "Ignoring what the server send till registered: {:? }",
                            ignore
                        );
                        //return Err(Error::ServerWentMad)
                    },
                }
            }
        })
    }

    /// Request a state transition to `ClientState::Character`.
    pub fn request_character(&mut self, character_id: i32) {
        self.singleton_stream
            .send(ClientMsg::Character(character_id))
            .unwrap();

        self.active_character_id = Some(character_id);
        self.client_state = ClientState::Pending;
    }

    /// Load the current players character list
    pub fn load_character_list(&mut self) {
        self.character_list.loading = true;
        self.singleton_stream
            .send(ClientMsg::RequestCharacterList)
            .unwrap();
    }

    /// New character creation
    pub fn create_character(&mut self, alias: String, tool: Option<String>, body: comp::Body) {
        self.character_list.loading = true;
        self.singleton_stream
            .send(ClientMsg::CreateCharacter { alias, tool, body })
            .unwrap();
    }

    /// Character deletion
    pub fn delete_character(&mut self, character_id: i32) {
        self.character_list.loading = true;
        self.singleton_stream
            .send(ClientMsg::DeleteCharacter(character_id))
            .unwrap();
    }

    /// Send disconnect message to the server
    pub fn request_logout(&mut self) {
        debug!("Requesting logout from server");
        if let Err(e) = self.singleton_stream.send(ClientMsg::Disconnect) {
            error!(
                ?e,
                "Couldn't send disconnect package to server, did server close already?"
            );
        }
    }

    /// Request a state transition to `ClientState::Registered` from an ingame
    /// state.
    pub fn request_remove_character(&mut self) {
        self.singleton_stream.send(ClientMsg::ExitIngame).unwrap();
        self.client_state = ClientState::Pending;
    }

    pub fn set_view_distance(&mut self, view_distance: u32) {
        self.view_distance = Some(view_distance.max(1).min(65));
        self.singleton_stream
            .send(ClientMsg::SetViewDistance(self.view_distance.unwrap()))
            .unwrap();
        // Can't fail
    }

    pub fn use_slot(&mut self, slot: comp::slot::Slot) {
        self.singleton_stream
            .send(ClientMsg::ControlEvent(ControlEvent::InventoryManip(
                InventoryManip::Use(slot),
            )))
            .unwrap();
    }

    pub fn swap_slots(&mut self, a: comp::slot::Slot, b: comp::slot::Slot) {
        self.singleton_stream
            .send(ClientMsg::ControlEvent(ControlEvent::InventoryManip(
                InventoryManip::Swap(a, b),
            )))
            .unwrap();
    }

    pub fn drop_slot(&mut self, slot: comp::slot::Slot) {
        self.singleton_stream
            .send(ClientMsg::ControlEvent(ControlEvent::InventoryManip(
                InventoryManip::Drop(slot),
            )))
            .unwrap();
    }

    pub fn pick_up(&mut self, entity: EcsEntity) {
        if let Some(uid) = self.state.read_component_copied(entity) {
            self.singleton_stream
                .send(ClientMsg::ControlEvent(ControlEvent::InventoryManip(
                    InventoryManip::Pickup(uid),
                )))
                .unwrap();
        }
    }

    pub fn recipe_book(&self) -> &RecipeBook { &self.recipe_book }

    pub fn available_recipes(&self) -> &HashSet<String> { &self.available_recipes }

    pub fn can_craft_recipe(&self, recipe: &str) -> bool {
        self.recipe_book
            .get(recipe)
            .zip(self.inventories().get(self.entity))
            .map(|(recipe, inv)| inv.contains_ingredients(&*recipe).is_ok())
            .unwrap_or(false)
    }

    pub fn craft_recipe(&mut self, recipe: &str) -> bool {
        if self.can_craft_recipe(recipe) {
            self.singleton_stream
                .send(ClientMsg::ControlEvent(ControlEvent::InventoryManip(
                    InventoryManip::CraftRecipe(recipe.to_string()),
                )))
                .unwrap();
            true
        } else {
            false
        }
    }

    fn update_available_recipes(&mut self) {
        self.available_recipes = self
            .recipe_book
            .iter()
            .map(|(name, _)| name.clone())
            .filter(|name| self.can_craft_recipe(name))
            .collect();
    }

    pub fn toggle_lantern(&mut self) {
        self.singleton_stream
            .send(ClientMsg::ControlEvent(ControlEvent::ToggleLantern))
            .unwrap();
    }

    pub fn group_invite(&self) -> Option<Uid> { self.group_invite }

    pub fn group_leader(&self) -> Option<Uid> { self.group_leader }

    pub fn send_group_invite(&mut self, invitee: Uid) {
        self.singleton_stream
            .send(ClientMsg::ControlEvent(ControlEvent::GroupManip( GroupManip::Invite(invitee) )))
            .unwrap()
    }

    pub fn accept_group_invite(&mut self) {
        // Clear invite
        self.group_invite.take();
        self.singleton_stream
            .send(ClientMsg::ControlEvent(ControlEvent::GroupManip(
                GroupManip::Accept,
            ))).unwrap();
    }

    pub fn reject_group_invite(&mut self) {
        // Clear invite
        self.group_invite.take();
        self.singleton_stream
            .send(ClientMsg::ControlEvent(ControlEvent::GroupManip(
                GroupManip::Reject,
            ))).unwrap();
    }

    pub fn leave_group(&mut self) {
        self.singleton_stream
            .send(ClientMsg::ControlEvent(ControlEvent::GroupManip(
                GroupManip::Leave,
            ))).unwrap();
    }

    pub fn kick_from_group(&mut self, uid: Uid) {
        self.singleton_stream
            .send(ClientMsg::ControlEvent(ControlEvent::GroupManip(
                GroupManip::Kick(uid),
            ))).unwrap();
    }

    pub fn assign_group_leader(&mut self, uid: Uid) {
        self.singleton_stream
            .send(ClientMsg::ControlEvent(ControlEvent::GroupManip(
                GroupManip::AssignLeader(uid),
            ))).unwrap();
    }

    pub fn is_mounted(&self) -> bool {
        self.state
            .ecs()
            .read_storage::<comp::Mounting>()
            .get(self.entity)
            .is_some()
    }

    pub fn mount(&mut self, entity: EcsEntity) {
        if let Some(uid) = self.state.read_component_copied(entity) {
            self.singleton_stream
                .send(ClientMsg::ControlEvent(ControlEvent::Mount(uid)))
                .unwrap();
        }
    }

    pub fn unmount(&mut self) {
        self.singleton_stream
            .send(ClientMsg::ControlEvent(ControlEvent::Unmount))
            .unwrap();
    }

    pub fn respawn(&mut self) {
        if self
            .state
            .ecs()
            .read_storage::<comp::Stats>()
            .get(self.entity)
            .map_or(false, |s| s.is_dead)
        {
            self.singleton_stream
                .send(ClientMsg::ControlEvent(ControlEvent::Respawn))
                .unwrap();
        }
    }

    /// Checks whether a player can swap their weapon+ability `Loadout` settings
    /// and sends the `ControlAction` event that signals to do the swap.
    pub fn swap_loadout(&mut self) { self.control_action(ControlAction::SwapLoadout) }

    pub fn toggle_wield(&mut self) {
        let is_wielding = self
            .state
            .ecs()
            .read_storage::<comp::CharacterState>()
            .get(self.entity)
            .map(|cs| cs.is_wield());

        match is_wielding {
            Some(true) => self.control_action(ControlAction::Unwield),
            Some(false) => self.control_action(ControlAction::Wield),
            None => warn!("Can't toggle wield, client entity doesn't have a `CharacterState`"),
        }
    }

    pub fn toggle_sit(&mut self) {
        let is_sitting = self
            .state
            .ecs()
            .read_storage::<comp::CharacterState>()
            .get(self.entity)
            .map(|cs| matches!(cs, comp::CharacterState::Sit));

        match is_sitting {
            Some(true) => self.control_action(ControlAction::Stand),
            Some(false) => self.control_action(ControlAction::Sit),
            None => warn!("Can't toggle sit, client entity doesn't have a `CharacterState`"),
        }
    }

    pub fn toggle_dance(&mut self) {
        let is_dancing = self
            .state
            .ecs()
            .read_storage::<comp::CharacterState>()
            .get(self.entity)
            .map(|cs| matches!(cs, comp::CharacterState::Dance));

        match is_dancing {
            Some(true) => self.control_action(ControlAction::Stand),
            Some(false) => self.control_action(ControlAction::Dance),
            None => warn!("Can't toggle dance, client entity doesn't have a `CharacterState`"),
        }
    }

    pub fn toggle_glide(&mut self) {
        let is_gliding = self
            .state
            .ecs()
            .read_storage::<comp::CharacterState>()
            .get(self.entity)
            .map(|cs| {
                matches!(
                    cs,
                    comp::CharacterState::GlideWield | comp::CharacterState::Glide
                )
            });

        match is_gliding {
            Some(true) => self.control_action(ControlAction::Unwield),
            Some(false) => self.control_action(ControlAction::GlideWield),
            None => warn!("Can't toggle glide, client entity doesn't have a `CharacterState`"),
        }
    }

    fn control_action(&mut self, control_action: ControlAction) {
        if let Some(controller) = self
            .state
            .ecs()
            .write_storage::<Controller>()
            .get_mut(self.entity)
        {
            controller.actions.push(control_action);
        }
        self.singleton_stream
            .send(ClientMsg::ControlAction(control_action))
            .unwrap();
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

    pub fn loadouts(&self) -> ReadStorage<comp::Loadout> { self.state.read_storage() }

    /// Send a chat message to the server.
    pub fn send_chat(&mut self, message: String) {
        match validate_chat_msg(&message) {
            Ok(()) => self
                .singleton_stream
                .send(ClientMsg::ChatMsg(message))
                .unwrap(),
            Err(ChatMsgValidationError::TooLong) => tracing::warn!(
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
        self.singleton_stream
            .send(ClientMsg::PlaceBlock(pos, block))
            .unwrap();
    }

    pub fn remove_block(&mut self, pos: Vec3<i32>) {
        self.singleton_stream
            .send(ClientMsg::BreakBlock(pos))
            .unwrap();
    }

    pub fn collect_block(&mut self, pos: Vec3<i32>) {
        self.singleton_stream
            .send(ClientMsg::ControlEvent(ControlEvent::InventoryManip(
                InventoryManip::Collect(pos),
            )))
            .unwrap();
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
            if let Err(e) = self
                .state
                .ecs()
                .write_storage::<Controller>()
                .entry(self.entity)
                .map(|entry| {
                    entry
                        .or_insert_with(|| Controller {
                            inputs: inputs.clone(),
                            events: Vec::new(),
                            actions: Vec::new(),
                        })
                        .inputs = inputs.clone();
                })
            {
                let entry = self.entity;
                error!(
                    ?e,
                    ?entry,
                    "Couldn't access controller component on client entity"
                );
            }
            self.singleton_stream
                .send(ClientMsg::ControllerInputs(inputs))?;
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
                        .map(|l| !client_character_state.same_variant(&l.0))
                        .unwrap_or(true)
                    {
                        let _ = last_character_states
                            .insert(entity, comp::Last(client_character_state.clone()));
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
                    .map(|e: i32| (e.abs() as u32).saturating_sub(2))
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
                                    self.singleton_stream
                                        .send(ClientMsg::TerrainChunkRequest { key: *key })?;
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
            self.singleton_stream.send(ClientMsg::Ping)?;
            self.last_server_ping = self.state.get_time();
        }

        // 6) Update the server about the player's physics attributes.
        if let ClientState::Character = self.client_state {
            if let (Some(pos), Some(vel), Some(ori)) = (
                self.state.read_storage().get(self.entity).cloned(),
                self.state.read_storage().get(self.entity).cloned(),
                self.state.read_storage().get(self.entity).cloned(),
            ) {
                self.singleton_stream
                    .send(ClientMsg::PlayerPhysics { pos, vel, ori })?;
            }
        }

        /*
        // Output debug metrics
        if log_enabled!(Level::Info) && self.tick % 600 == 0 {
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

    async fn handle_message(
        &mut self,
        frontend_events: &mut Vec<Event>,
        cnt: &mut u64,
    ) -> Result<(), Error> {
        loop {
            let msg = self.singleton_stream.recv().await?;
            *cnt += 1;
            match msg {
                ServerMsg::TooManyPlayers => {
                    return Err(Error::ServerWentMad);
                },
                ServerMsg::Shutdown => return Err(Error::ServerShutdown),
                ServerMsg::InitialSync { .. } => return Err(Error::ServerWentMad),
                ServerMsg::PlayerListUpdate(PlayerListUpdate::Init(list)) => {
                    self.player_list = list
                },
                ServerMsg::PlayerListUpdate(PlayerListUpdate::Add(uid, player_info)) => {
                    if let Some(old_player_info) = self.player_list.insert(uid, player_info.clone())
                    {
                        warn!(
                            "Received msg to insert {} with uid {} into the player list but there \
                             was already an entry for {} with the same uid that was overwritten!",
                            player_info.player_alias, uid, old_player_info.player_alias
                        );
                    }
                },
                ServerMsg::PlayerListUpdate(PlayerListUpdate::Admin(uid, admin)) => {
                    if let Some(player_info) = self.player_list.get_mut(&uid) {
                        player_info.is_admin = admin;
                    } else {
                        warn!(
                            "Received msg to update admin status of uid {}, but they were not in \
                             the list.",
                            uid
                        );
                    }
                },
                ServerMsg::PlayerListUpdate(PlayerListUpdate::SelectedCharacter(
                    uid,
                    char_info,
                )) => {
                    if let Some(player_info) = self.player_list.get_mut(&uid) {
                        player_info.character = Some(char_info);
                    } else {
                        warn!(
                            "Received msg to update character info for uid {}, but they were not \
                             in the list.",
                            uid
                        );
                    }
                },
                ServerMsg::PlayerListUpdate(PlayerListUpdate::LevelChange(uid, next_level)) => {
                    if let Some(player_info) = self.player_list.get_mut(&uid) {
                        player_info.character = match &player_info.character {
                            Some(character) => Some(common::msg::CharacterInfo {
                                name: character.name.to_string(),
                                level: next_level,
                            }),
                            None => {
                                warn!(
                                    "Received msg to update character level info to {} for uid \
                                     {}, but this player's character is None.",
                                    next_level, uid
                                );

                                None
                            },
                        };
                    }
                },
                ServerMsg::PlayerListUpdate(PlayerListUpdate::Remove(uid)) => {
                    // Instead of removing players, mark them as offline because we need to
                    // remember the names of disconnected players in chat.
                    //
                    // TODO the server should re-use uids of players that log out and log back
                    // in.

                    if let Some(player_info) = self.player_list.get_mut(&uid) {
                        if player_info.is_online {
                            player_info.is_online = false;
                        } else {
                            warn!(
                                "Received msg to remove uid {} from the player list by they were \
                                 already marked offline",
                                uid
                            );
                        }
                    } else {
                        warn!(
                            "Received msg to remove uid {} from the player list by they weren't \
                             in the list!",
                            uid
                        );
                    }
                },
                ServerMsg::PlayerListUpdate(PlayerListUpdate::Alias(uid, new_name)) => {
                    if let Some(player_info) = self.player_list.get_mut(&uid) {
                        player_info.player_alias = new_name;
                    } else {
                        warn!(
                            "Received msg to alias player with uid {} to {} but this uid is not \
                             in the player list",
                            uid, new_name
                        );
                    }
                },
                ServerMsg::GroupUpdate(change_notification) => {
                        use comp::group::ChangeNotification::*;
                        // Note: we use a hashmap since this would not work with entities outside
                        // the view distance
                        match change_notification {
                            Added(uid) => {
                                if !self.group_members.insert(uid) {
                                    warn!(
                                        "Received msg to add uid {} to the group members but they \
                                         were already there",
                                        uid
                                    );
                                }
                            },
                            Removed(uid) => {
                                if !self.group_members.remove(&uid) {
                                    warn!(
                                        "Received msg to remove uid {} from group members but by \
                                         they weren't in there!",
                                        uid
                                    );
                                }
                            },
                            NewLeader(leader) => {
                                self.group_leader = Some(leader);
                            },
                            NewGroup { leader, members } => {
                                self.group_leader = Some(leader);
                                self.group_members = members.into_iter().collect();
                                // Currently add/remove messages treat client as an implicit member
                                // of the group whereas this message explicitly included them so to
                                // be consistent for now we will remove the client from the
                                // received hashset
                                if let Some(uid) = self.uid() {
                                    self.group_members.remove(&uid);
                                }
                            },
                            NoGroup => {
                                self.group_leader = None;
                                self.group_members = HashSet::new();
                            }
                        }
                },
                ServerMsg::GroupInvite(uid) => {
                    self.group_invite = Some(uid);
                },
                ServerMsg::Ping => {
                    self.singleton_stream.send(ClientMsg::Pong)?;
                },
                ServerMsg::Pong => {
                    self.last_server_pong = self.state.get_time();
                    self.last_ping_delta = self.state.get_time() - self.last_server_ping;

                    // Maintain the correct number of deltas for calculating the rolling average
                    // ping. The client sends a ping to the server every second so we should be
                    // receiving a pong reply roughly every second.
                    while self.ping_deltas.len() > PING_ROLLING_AVERAGE_SECS - 1 {
                        self.ping_deltas.pop_front();
                    }
                    self.ping_deltas.push_back(self.last_ping_delta);
                },
                ServerMsg::ChatMsg(m) => frontend_events.push(Event::Chat(m)),
                ServerMsg::SetPlayerEntity(uid) => {
                    if let Some(entity) = self.state.ecs().entity_from_uid(uid.0) {
                        self.entity = entity;
                    } else {
                        return Err(Error::Other("Failed to find entity from uid.".to_owned()));
                    }
                },
                ServerMsg::TimeOfDay(time_of_day) => {
                    *self.state.ecs_mut().write_resource() = time_of_day;
                },
                ServerMsg::EntitySync(entity_sync_package) => {
                    self.state
                        .ecs_mut()
                        .apply_entity_sync_package(entity_sync_package);
                },
                ServerMsg::CompSync(comp_sync_package) => {
                    self.state
                        .ecs_mut()
                        .apply_comp_sync_package(comp_sync_package);
                },
                ServerMsg::CreateEntity(entity_package) => {
                    self.state.ecs_mut().apply_entity_package(entity_package);
                },
                ServerMsg::DeleteEntity(entity) => {
                    if self.uid() != Some(entity) {
                        self.state
                            .ecs_mut()
                            .delete_entity_and_clear_from_uid_allocator(entity.0);
                    }
                },
                // Cleanup for when the client goes back to the `Registered` state
                ServerMsg::ExitIngameCleanup => {
                    self.clean_state();
                },
                ServerMsg::InventoryUpdate(inventory, event) => {
                    match event {
                        InventoryUpdateEvent::CollectFailed => {},
                        _ => {
                            // Push the updated inventory component to the client
                            self.state.write_component(self.entity, inventory);
                        },
                    }

                    self.update_available_recipes();

                    frontend_events.push(Event::InventoryUpdated(event));
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
                    self.singleton_stream.send(ClientMsg::Terminate)?;
                },
                ServerMsg::CharacterListUpdate(character_list) => {
                    self.character_list.characters = character_list;
                    self.character_list.loading = false;
                },
                ServerMsg::CharacterActionError(error) => {
                    warn!("CharacterActionError: {:?}.", error);
                    self.character_list.error = Some(error);
                },
                ServerMsg::Notification(n) => {
                    frontend_events.push(Event::Notification(n));
                },
                ServerMsg::CharacterDataLoadError(error) => {
                    self.clean_state();
                    self.character_list.error = Some(error);
                },
                ServerMsg::SetViewDistance(vd) => {
                    self.view_distance = Some(vd);
                    frontend_events.push(Event::SetViewDistance(vd));
                },
            }
        }
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
            if duration_since_last_pong >= SERVER_TIMEOUT_GRACE_PERIOD
                && self.state.get_time() - duration_since_last_pong > 0.
            {
                frontend_events.push(Event::DisconnectionNotification(
                    (self.state.get_time() - duration_since_last_pong).round() as u64,
                ));
            }
        }

        let mut handles_msg = 0;

        block_on(async {
            //TIMEOUT 0.01 ms for msg handling
            select!(
                _ = Delay::new(std::time::Duration::from_micros(10)).fuse() => Ok(()),
                err = self.handle_message(&mut frontend_events, &mut handles_msg).fuse() => err,
            )
        })?;

        if handles_msg == 0 && self.state.get_time() - self.last_server_pong > SERVER_TIMEOUT {
            return Err(Error::ServerTimeout);
        }

        Ok(frontend_events)
    }

    /// Get the player's entity.
    pub fn entity(&self) -> EcsEntity { self.entity }

    /// Get the player's Uid.
    pub fn uid(&self) -> Option<Uid> {
        self.state.read_component_copied(self.entity)
    }

    /// Get the client state
    pub fn get_client_state(&self) -> ClientState { self.client_state }

    /// Get the current tick number.
    pub fn get_tick(&self) -> u64 { self.tick }

    pub fn get_ping_ms(&self) -> f64 { self.last_ping_delta * 1000.0 }

    pub fn get_ping_ms_rolling_avg(&self) -> f64 {
        let mut total_weight = 0.;
        let pings = self.ping_deltas.len() as f64;
        (self
            .ping_deltas
            .iter()
            .enumerate()
            .fold(0., |acc, (i, ping)| {
                let weight = i as f64 + 1. / pings;
                total_weight += weight;
                acc + (weight * ping)
            })
            / total_weight)
            * 1000.0
    }

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

    /// Return true if this client is an admin on the server
    pub fn is_admin(&self) -> bool {
        let client_uid = self
            .state
            .read_component_copied::<Uid>(self.entity)
            .expect("Client doesn't have a Uid!!!");

        self.player_list
            .get(&client_uid)
            .map_or(false, |info| info.is_admin)
    }

    /// Clean client ECS state
    fn clean_state(&mut self) {
        let client_uid = self
            .uid()
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
    }

    /// Format a message for the client (voxygen chat box or chat-cli)
    pub fn format_message(&self, msg: &comp::ChatMsg, character_name: bool) -> String {
        let comp::ChatMsg { chat_type, message } = &msg;
        let alias_of_uid = |uid| {
            self.player_list
                .get(uid)
                .map_or("<?>".to_string(), |player_info| {
                    if player_info.is_admin {
                        format!("ADMIN - {}", player_info.player_alias)
                    } else {
                        player_info.player_alias.to_string()
                    }
                })
        };
        let name_of_uid = |uid| {
            let ecs = self.state.ecs();
            (
                &ecs.read_storage::<comp::Stats>(),
                &ecs.read_storage::<Uid>(),
            )
                .join()
                .find(|(_, u)| u == &uid)
                .map(|(c, _)| c.name.clone())
        };
        let message_format = |uid, message, group| {
            let alias = alias_of_uid(uid);
            let name = if character_name {
                name_of_uid(uid)
            } else {
                None
            };
            match (group, name) {
                (Some(group), None) => format!("({}) [{}]: {}", group, alias, message),
                (None, None) => format!("[{}]: {}", alias, message),
                (Some(group), Some(name)) => {
                    format!("({}) [{}] {}: {}", group, alias, name, message)
                },
                (None, Some(name)) => format!("[{}] {}: {}", alias, name, message),
            }
        };
        match chat_type {
            comp::ChatType::Online => message.to_string(),
            comp::ChatType::Offline => message.to_string(),
            comp::ChatType::CommandError => message.to_string(),
            comp::ChatType::CommandInfo => message.to_string(),
            comp::ChatType::Loot => message.to_string(),
            comp::ChatType::FactionMeta(_) => message.to_string(),
            comp::ChatType::GroupMeta(_) => message.to_string(),
            comp::ChatType::Kill => message.to_string(),
            comp::ChatType::Tell(from, to) => {
                let from_alias = alias_of_uid(from);
                let to_alias = alias_of_uid(to);
                if Some(*from) == self.uid() {
                    format!("To [{}]: {}", to_alias, message)
                } else {
                    format!("From [{}]: {}", from_alias, message)
                }
            },
            comp::ChatType::Say(uid) => message_format(uid, message, None),
            comp::ChatType::Group(uid, s) => message_format(uid, message, Some(s)),
            comp::ChatType::Faction(uid, s) => message_format(uid, message, Some(s)),
            comp::ChatType::Region(uid) => message_format(uid, message, None),
            comp::ChatType::World(uid) => message_format(uid, message, None),
            // NPCs can't talk. Should be filtered by hud/mod.rs for voxygen and should be filtered
            // by server (due to not having a Pos) for chat-cli
            comp::ChatType::Npc(_uid, _r) => "".to_string(),
            comp::ChatType::Meta => message.to_string(),
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        trace!("Dropping client");
        if let Err(e) = self.singleton_stream.send(ClientMsg::Disconnect) {
            warn!(
                ?e,
                "Error during drop of client, couldn't send disconnect package, is the connection \
                 already closed?",
            );
        }
        if let Err(e) = block_on(self.participant.take().unwrap().disconnect()) {
            warn!(?e, "error when disconnecting, couldn't send all data");
        }
    }
}
