use super::SysTimer;
use crate::{
    alias_validator::AliasValidator,
    character_creator,
    client::Client,
    login_provider::LoginProvider,
    metrics::{NetworkRequestMetrics, PlayerMetrics},
    persistence::character_loader::CharacterLoader,
    EditableSettings, Settings,
};
use common::{
    comp::{
        Admin, CanBuild, ChatMode, ChatType, ControlEvent, Controller, ForceUpdate, Ori, Player,
        Pos, Stats, UnresolvedChatMsg, Vel,
    },
    event::{EventBus, ServerEvent},
    msg::{
        validate_chat_msg, CharacterInfo, ChatMsgValidationError, ClientGeneralMsg,
        ClientInGameMsg, ClientIngame, ClientNotInGameMsg, ClientRegisterMsg, DisconnectReason,
        PingMsg, PlayerInfo, PlayerListUpdate, RegisterError, ServerGeneralMsg, ServerInGameMsg,
        ServerNotInGameMsg, ServerRegisterAnswerMsg, MAX_BYTES_CHAT_MSG,
    },
    span,
    state::{BlockChange, Time},
    sync::Uid,
    terrain::{TerrainChunkSize, TerrainGrid},
    vol::{ReadVol, RectVolSize},
};
use futures_executor::block_on;
use futures_timer::Delay;
use futures_util::{select, FutureExt};
use hashbrown::HashMap;
use specs::{
    Entities, Join, Read, ReadExpect, ReadStorage, System, Write, WriteExpect, WriteStorage,
};
use tracing::{debug, error, info, trace, warn};

impl Sys {
    #[allow(clippy::too_many_arguments)]
    fn handle_client_msg(
        server_emitter: &mut common::event::Emitter<'_, ServerEvent>,
        new_chat_msgs: &mut Vec<(Option<specs::Entity>, UnresolvedChatMsg)>,
        entity: specs::Entity,
        client: &mut Client,
        player_metrics: &ReadExpect<'_, PlayerMetrics>,
        uids: &ReadStorage<'_, Uid>,
        chat_modes: &ReadStorage<'_, ChatMode>,
        msg: ClientGeneralMsg,
    ) -> Result<(), crate::error::Error> {
        match msg {
            ClientGeneralMsg::ChatMsg(message) => {
                if client.registered {
                    match validate_chat_msg(&message) {
                        Ok(()) => {
                            if let Some(from) = uids.get(entity) {
                                let mode = chat_modes.get(entity).cloned().unwrap_or_default();
                                let msg = mode.new_message(*from, message);
                                new_chat_msgs.push((Some(entity), msg));
                            } else {
                                error!("Could not send message. Missing player uid");
                            }
                        },
                        Err(ChatMsgValidationError::TooLong) => {
                            let max = MAX_BYTES_CHAT_MSG;
                            let len = message.len();
                            warn!(?len, ?max, "Received a chat message that's too long")
                        },
                    }
                }
            },
            ClientGeneralMsg::Disconnect => {
                client.send_msg(ServerGeneralMsg::Disconnect(DisconnectReason::Requested));
            },
            ClientGeneralMsg::Terminate => {
                debug!(?entity, "Client send message to termitate session");
                player_metrics
                    .clients_disconnected
                    .with_label_values(&["gracefully"])
                    .inc();
                server_emitter.emit(ServerEvent::ClientDisconnect(entity));
            },
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_client_in_game_msg(
        server_emitter: &mut common::event::Emitter<'_, ServerEvent>,
        entity: specs::Entity,
        client: &mut Client,
        terrain: &ReadExpect<'_, TerrainGrid>,
        network_metrics: &ReadExpect<'_, NetworkRequestMetrics>,
        can_build: &ReadStorage<'_, CanBuild>,
        force_updates: &ReadStorage<'_, ForceUpdate>,
        stats: &mut WriteStorage<'_, Stats>,
        block_changes: &mut Write<'_, BlockChange>,
        positions: &mut WriteStorage<'_, Pos>,
        velocities: &mut WriteStorage<'_, Vel>,
        orientations: &mut WriteStorage<'_, Ori>,
        players: &mut WriteStorage<'_, Player>,
        controllers: &mut WriteStorage<'_, Controller>,
        settings: &Read<'_, Settings>,
        msg: ClientInGameMsg,
    ) -> Result<(), crate::error::Error> {
        if client.in_game.is_none() {
            debug!(?entity, "client is not in_game, ignoring msg");
            trace!(?msg, "ignored msg content");
            if matches!(msg, ClientInGameMsg::TerrainChunkRequest{ .. }) {
                network_metrics.chunks_request_dropped.inc();
            }
            return Ok(());
        }
        match msg {
            // Go back to registered state (char selection screen)
            ClientInGameMsg::ExitInGame => {
                client.in_game = None;
                server_emitter.emit(ServerEvent::ExitIngame { entity });
                client.send_in_game(ServerInGameMsg::ExitInGameSuccess);
            },
            ClientInGameMsg::SetViewDistance(view_distance) => {
                players.get_mut(entity).map(|player| {
                    player.view_distance = Some(
                        settings
                            .max_view_distance
                            .map(|max| view_distance.min(max))
                            .unwrap_or(view_distance),
                    )
                });

                //correct client if its VD is to high
                if settings
                    .max_view_distance
                    .map(|max| view_distance > max)
                    .unwrap_or(false)
                {
                    client.send_in_game(ServerInGameMsg::SetViewDistance(
                        settings.max_view_distance.unwrap_or(0),
                    ));
                }
            },
            ClientInGameMsg::ControllerInputs(inputs) => {
                if let Some(ClientIngame::Character) = client.in_game {
                    if let Some(controller) = controllers.get_mut(entity) {
                        controller.inputs.update_with_new(inputs);
                    }
                }
            },
            ClientInGameMsg::ControlEvent(event) => {
                if let Some(ClientIngame::Character) = client.in_game {
                    // Skip respawn if client entity is alive
                    if let ControlEvent::Respawn = event {
                        if stats.get(entity).map_or(true, |s| !s.is_dead) {
                            //Todo: comment why return!
                            return Ok(());
                        }
                    }
                    if let Some(controller) = controllers.get_mut(entity) {
                        controller.events.push(event);
                    }
                }
            },
            ClientInGameMsg::ControlAction(event) => {
                if let Some(ClientIngame::Character) = client.in_game {
                    if let Some(controller) = controllers.get_mut(entity) {
                        controller.actions.push(event);
                    }
                }
            },
            ClientInGameMsg::PlayerPhysics { pos, vel, ori } => {
                if let Some(ClientIngame::Character) = client.in_game {
                    if force_updates.get(entity).is_none()
                        && stats.get(entity).map_or(true, |s| !s.is_dead)
                    {
                        let _ = positions.insert(entity, pos);
                        let _ = velocities.insert(entity, vel);
                        let _ = orientations.insert(entity, ori);
                    }
                }
            },
            ClientInGameMsg::BreakBlock(pos) => {
                if let Some(block) = can_build.get(entity).and_then(|_| terrain.get(pos).ok()) {
                    block_changes.set(pos, block.into_vacant());
                }
            },
            ClientInGameMsg::PlaceBlock(pos, block) => {
                if can_build.get(entity).is_some() {
                    block_changes.try_set(pos, block);
                }
            },
            ClientInGameMsg::TerrainChunkRequest { key } => {
                let in_vd = if let (Some(view_distance), Some(pos)) = (
                    players.get(entity).and_then(|p| p.view_distance),
                    positions.get(entity),
                ) {
                    pos.0.xy().map(|e| e as f64).distance(
                        key.map(|e| e as f64 + 0.5) * TerrainChunkSize::RECT_SIZE.map(|e| e as f64),
                    ) < (view_distance as f64 - 1.0 + 2.5 * 2.0_f64.sqrt())
                        * TerrainChunkSize::RECT_SIZE.x as f64
                } else {
                    true
                };
                if in_vd {
                    match terrain.get_key(key) {
                        Some(chunk) => {
                            network_metrics.chunks_served_from_memory.inc();
                            client.send_in_game(ServerInGameMsg::TerrainChunkUpdate {
                                key,
                                chunk: Ok(Box::new(chunk.clone())),
                            })
                        },
                        None => {
                            network_metrics.chunks_generation_triggered.inc();
                            server_emitter.emit(ServerEvent::ChunkRequest(entity, key))
                        },
                    }
                } else {
                    network_metrics.chunks_request_dropped.inc();
                }
            },
            ClientInGameMsg::UnlockSkill(skill) => {
                stats
                    .get_mut(entity)
                    .map(|s| s.skill_set.unlock_skill(skill));
            },
            ClientInGameMsg::RefundSkill(skill) => {
                stats
                    .get_mut(entity)
                    .map(|s| s.skill_set.refund_skill(skill));
            },
            ClientInGameMsg::UnlockSkillGroup(skill_group_type) => {
                stats
                    .get_mut(entity)
                    .map(|s| s.skill_set.unlock_skill_group(skill_group_type));
            },
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_client_not_in_game_msg(
        server_emitter: &mut common::event::Emitter<'_, ServerEvent>,
        new_chat_msgs: &mut Vec<(Option<specs::Entity>, UnresolvedChatMsg)>,
        entity: specs::Entity,
        client: &mut Client,
        character_loader: &ReadExpect<'_, CharacterLoader>,
        uids: &ReadStorage<'_, Uid>,
        players: &mut WriteStorage<'_, Player>,
        editable_settings: &ReadExpect<'_, EditableSettings>,
        alias_validator: &ReadExpect<'_, AliasValidator>,
        msg: ClientNotInGameMsg,
    ) -> Result<(), crate::error::Error> {
        match msg {
            // Request spectator state
            ClientNotInGameMsg::Spectate => {
                if client.registered {
                    client.in_game = Some(ClientIngame::Spectator)
                } else {
                    debug!("dropped Spectate msg from unregistered client");
                }
            },
            ClientNotInGameMsg::Character(character_id) => {
                if client.registered && client.in_game.is_none() {
                    // Only send login message if it wasn't already
                    // sent previously
                    if let Some(player) = players.get(entity) {
                        // Send a request to load the character's component data from the
                        // DB. Once loaded, persisted components such as stats and inventory
                        // will be inserted for the entity
                        character_loader.load_character_data(
                            entity,
                            player.uuid().to_string(),
                            character_id,
                        );

                        // Start inserting non-persisted/default components for the entity
                        // while we load the DB data
                        server_emitter.emit(ServerEvent::InitCharacterData {
                            entity,
                            character_id,
                        });

                        // Give the player a welcome message
                        if !editable_settings.server_description.is_empty() {
                            client.send_msg(
                                ChatType::CommandInfo.server_msg(String::from(
                                    &*editable_settings.server_description,
                                )),
                            );
                        }

                        // Only send login message if it wasn't already
                        // sent previously
                        if !client.login_msg_sent {
                            if let Some(player_uid) = uids.get(entity) {
                                new_chat_msgs.push((None, UnresolvedChatMsg {
                                    chat_type: ChatType::Online(*player_uid),
                                    message: "".to_string(),
                                }));

                                client.login_msg_sent = true;
                            }
                        }
                    } else {
                        client.send_not_in_game(ServerNotInGameMsg::CharacterDataLoadError(
                            String::from("Failed to fetch player entity"),
                        ))
                    }
                } else {
                    let registered = client.registered;
                    let in_game = client.in_game;
                    debug!(?registered, ?in_game, "dropped Character msg from client");
                }
            },
            ClientNotInGameMsg::RequestCharacterList => {
                if let Some(player) = players.get(entity) {
                    character_loader.load_character_list(entity, player.uuid().to_string())
                }
            },
            ClientNotInGameMsg::CreateCharacter { alias, tool, body } => {
                if let Err(error) = alias_validator.validate(&alias) {
                    debug!(?error, ?alias, "denied alias as it contained a banned word");
                    client.send_not_in_game(ServerNotInGameMsg::CharacterActionError(
                        error.to_string(),
                    ));
                } else if let Some(player) = players.get(entity) {
                    character_creator::create_character(
                        entity,
                        player.uuid().to_string(),
                        alias,
                        tool,
                        body,
                        character_loader,
                    );
                }
            },
            ClientNotInGameMsg::DeleteCharacter(character_id) => {
                if let Some(player) = players.get(entity) {
                    character_loader.delete_character(
                        entity,
                        player.uuid().to_string(),
                        character_id,
                    );
                }
            },
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_ping_msg(client: &mut Client, msg: PingMsg) -> Result<(), crate::error::Error> {
        match msg {
            PingMsg::Ping => client.send_ping(PingMsg::Pong),
            PingMsg::Pong => {},
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_register_msg(
        player_list: &HashMap<Uid, PlayerInfo>,
        new_players: &mut Vec<specs::Entity>,
        entity: specs::Entity,
        client: &mut Client,
        player_metrics: &ReadExpect<'_, PlayerMetrics>,
        login_provider: &mut WriteExpect<'_, LoginProvider>,
        admins: &mut WriteStorage<'_, Admin>,
        players: &mut WriteStorage<'_, Player>,
        editable_settings: &ReadExpect<'_, EditableSettings>,
        msg: ClientRegisterMsg,
    ) -> Result<(), crate::error::Error> {
        let (username, uuid) = match login_provider.try_login(
            &msg.token_or_username,
            &*editable_settings.admins,
            &*editable_settings.whitelist,
            &*editable_settings.banlist,
        ) {
            Err(err) => {
                client
                    .register_stream
                    .send(ServerRegisterAnswerMsg::Err(err))?;
                return Ok(());
            },
            Ok((username, uuid)) => (username, uuid),
        };

        const INITIAL_VD: Option<u32> = Some(5); //will be changed after login
        let player = Player::new(username.clone(), None, INITIAL_VD, uuid);
        let is_admin = editable_settings.admins.contains(&uuid);

        if !player.is_valid() {
            // Invalid player
            client.register_stream.send(ServerRegisterAnswerMsg::Err(
                RegisterError::InvalidCharacter,
            ))?;
            return Ok(());
        }

        if !client.registered && client.in_game.is_none() {
            // Add Player component to this client
            let _ = players.insert(entity, player);
            player_metrics.players_connected.inc();

            // Give the Admin component to the player if their name exists in
            // admin list
            if is_admin {
                let _ = admins.insert(entity, Admin);
            }

            // Tell the client its request was successful.
            client.registered = true;
            client
                .register_stream
                .send(ServerRegisterAnswerMsg::Ok(()))?;

            // Send initial player list
            client.send_msg(ServerGeneralMsg::PlayerListUpdate(PlayerListUpdate::Init(
                player_list.clone(),
            )));

            // Add to list to notify all clients of the new player
            new_players.push(entity);
        }
        Ok(())
    }

    ///We needed to move this to a async fn, if we would use a async closures
    /// the compiler generates to much recursion and fails to compile this
    #[allow(clippy::too_many_arguments)]
    async fn handle_messages(
        server_emitter: &mut common::event::Emitter<'_, ServerEvent>,
        new_chat_msgs: &mut Vec<(Option<specs::Entity>, UnresolvedChatMsg)>,
        player_list: &HashMap<Uid, PlayerInfo>,
        new_players: &mut Vec<specs::Entity>,
        entity: specs::Entity,
        client: &mut Client,
        cnt: &mut u64,
        character_loader: &ReadExpect<'_, CharacterLoader>,
        terrain: &ReadExpect<'_, TerrainGrid>,
        network_metrics: &ReadExpect<'_, NetworkRequestMetrics>,
        player_metrics: &ReadExpect<'_, PlayerMetrics>,
        uids: &ReadStorage<'_, Uid>,
        can_build: &ReadStorage<'_, CanBuild>,
        force_updates: &ReadStorage<'_, ForceUpdate>,
        stats: &mut WriteStorage<'_, Stats>,
        chat_modes: &ReadStorage<'_, ChatMode>,
        login_provider: &mut WriteExpect<'_, LoginProvider>,
        block_changes: &mut Write<'_, BlockChange>,
        admins: &mut WriteStorage<'_, Admin>,
        positions: &mut WriteStorage<'_, Pos>,
        velocities: &mut WriteStorage<'_, Vel>,
        orientations: &mut WriteStorage<'_, Ori>,
        players: &mut WriteStorage<'_, Player>,
        controllers: &mut WriteStorage<'_, Controller>,
        settings: &Read<'_, Settings>,
        editable_settings: &ReadExpect<'_, EditableSettings>,
        alias_validator: &ReadExpect<'_, AliasValidator>,
    ) -> Result<(), crate::error::Error> {
        loop {
            let q1 = Client::internal_recv(&client.network_error, &mut client.singleton_stream);
            let q2 = Client::internal_recv(&client.network_error, &mut client.in_game_stream);
            let q3 = Client::internal_recv(&client.network_error, &mut client.not_in_game_stream);
            let q4 = Client::internal_recv(&client.network_error, &mut client.ping_stream);
            let q5 = Client::internal_recv(&client.network_error, &mut client.register_stream);

            let (m1, m2, m3, m4, m5) = select!(
                msg = q1.fuse() => (Some(msg?), None, None, None, None),
                msg = q2.fuse() => (None, Some(msg?), None, None, None),
                msg = q3.fuse() => (None, None, Some(msg?), None, None),
                msg = q4.fuse() => (None, None, None, Some(msg?), None),
                msg = q5.fuse() => (None, None, None, None,Some(msg?)),
            );
            *cnt += 1;
            if let Some(msg) = m1 {
                Self::handle_client_msg(
                    server_emitter,
                    new_chat_msgs,
                    entity,
                    client,
                    player_metrics,
                    uids,
                    chat_modes,
                    msg,
                )?;
            }
            if let Some(msg) = m2 {
                Self::handle_client_in_game_msg(
                    server_emitter,
                    entity,
                    client,
                    terrain,
                    network_metrics,
                    can_build,
                    force_updates,
                    stats,
                    block_changes,
                    positions,
                    velocities,
                    orientations,
                    players,
                    controllers,
                    settings,
                    msg,
                )?;
            }
            if let Some(msg) = m3 {
                Self::handle_client_not_in_game_msg(
                    server_emitter,
                    new_chat_msgs,
                    entity,
                    client,
                    character_loader,
                    uids,
                    players,
                    editable_settings,
                    alias_validator,
                    msg,
                )?;
            }
            if let Some(msg) = m4 {
                Self::handle_ping_msg(client, msg)?;
            }
            if let Some(msg) = m5 {
                Self::handle_register_msg(
                    player_list,
                    new_players,
                    entity,
                    client,
                    player_metrics,
                    login_provider,
                    admins,
                    players,
                    editable_settings,
                    msg,
                )?;
            }
        }
    }
}

/// This system will handle new messages from clients
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)] // TODO: Pending review in #587
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, Time>,
        ReadExpect<'a, CharacterLoader>,
        ReadExpect<'a, TerrainGrid>,
        ReadExpect<'a, NetworkRequestMetrics>,
        ReadExpect<'a, PlayerMetrics>,
        Write<'a, SysTimer<Self>>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, CanBuild>,
        ReadStorage<'a, ForceUpdate>,
        WriteStorage<'a, Stats>,
        ReadStorage<'a, ChatMode>,
        WriteExpect<'a, LoginProvider>,
        Write<'a, BlockChange>,
        WriteStorage<'a, Admin>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Player>,
        WriteStorage<'a, Client>,
        WriteStorage<'a, Controller>,
        Read<'a, Settings>,
        ReadExpect<'a, EditableSettings>,
        ReadExpect<'a, AliasValidator>,
    );

    #[allow(clippy::match_ref_pats)] // TODO: Pending review in #587
    #[allow(clippy::single_char_pattern)] // TODO: Pending review in #587
    #[allow(clippy::single_match)] // TODO: Pending review in #587
    fn run(
        &mut self,
        (
            entities,
            server_event_bus,
            time,
            character_loader,
            terrain,
            network_metrics,
            player_metrics,
            mut timer,
            uids,
            can_build,
            force_updates,
            mut stats,
            chat_modes,
            mut accounts,
            mut block_changes,
            mut admins,
            mut positions,
            mut velocities,
            mut orientations,
            mut players,
            mut clients,
            mut controllers,
            settings,
            editable_settings,
            alias_validator,
        ): Self::SystemData,
    ) {
        span!(_guard, "run", "message::Sys::run");
        timer.start();

        let mut server_emitter = server_event_bus.emitter();

        let mut new_chat_msgs = Vec::new();

        // Player list to send new players.
        let player_list = (&uids, &players, stats.maybe(), admins.maybe())
            .join()
            .map(|(uid, player, stats, admin)| {
                (*uid, PlayerInfo {
                    is_online: true,
                    is_admin: admin.is_some(),
                    player_alias: player.alias.clone(),
                    character: stats.map(|stats| CharacterInfo {
                        name: stats.name.clone(),
                        level: stats.level.level(),
                    }),
                })
            })
            .collect::<HashMap<_, _>>();
        // List of new players to update player lists of all clients.
        let mut new_players = Vec::new();

        for (entity, client) in (&entities, &mut clients).join() {
            let mut cnt = 0;

            let network_err: Result<(), crate::error::Error> = block_on(async {
                //TIMEOUT 0.02 ms for msg handling
                let work_future = Self::handle_messages(
                    &mut server_emitter,
                    &mut new_chat_msgs,
                    &player_list,
                    &mut new_players,
                    entity,
                    client,
                    &mut cnt,
                    &character_loader,
                    &terrain,
                    &network_metrics,
                    &player_metrics,
                    &uids,
                    &can_build,
                    &force_updates,
                    &mut stats,
                    &chat_modes,
                    &mut accounts,
                    &mut block_changes,
                    &mut admins,
                    &mut positions,
                    &mut velocities,
                    &mut orientations,
                    &mut players,
                    &mut controllers,
                    &settings,
                    &editable_settings,
                    &alias_validator,
                );
                select!(
                    _ = Delay::new(std::time::Duration::from_micros(20)).fuse() => Ok(()),
                    err = work_future.fuse() => err,
                )
            });

            // Update client ping.
            if cnt > 0 {
                client.last_ping = time.0
            } else if time.0 - client.last_ping > settings.client_timeout.as_secs() as f64
            // Timeout
            {
                info!(?entity, "timeout error with client, disconnecting");
                player_metrics
                    .clients_disconnected
                    .with_label_values(&["timeout"])
                    .inc();
                server_emitter.emit(ServerEvent::ClientDisconnect(entity));
            } else if network_err.is_err()
            // Postbox error
            {
                debug!(?entity, "postbox error with client, disconnecting");
                player_metrics
                    .clients_disconnected
                    .with_label_values(&["network_error"])
                    .inc();
                server_emitter.emit(ServerEvent::ClientDisconnect(entity));
            } else if time.0 - client.last_ping > settings.client_timeout.as_secs() as f64 * 0.5 {
                // Try pinging the client if the timeout is nearing.
                client.send_ping(PingMsg::Ping);
            }
        }

        // Handle new players.
        // Tell all clients to add them to the player list.
        for entity in new_players {
            if let (Some(uid), Some(player)) = (uids.get(entity), players.get(entity)) {
                let msg =
                    ServerGeneralMsg::PlayerListUpdate(PlayerListUpdate::Add(*uid, PlayerInfo {
                        player_alias: player.alias.clone(),
                        is_online: true,
                        is_admin: admins.get(entity).is_some(),
                        character: None, // new players will be on character select.
                    }));
                for client in (&mut clients).join().filter(|c| c.registered) {
                    client.send_msg(msg.clone())
                }
            }
        }

        // Handle new chat messages.
        for (entity, msg) in new_chat_msgs {
            // Handle chat commands.
            if msg.message.starts_with("/") {
                if let (Some(entity), true) = (entity, msg.message.len() > 1) {
                    let argv = String::from(&msg.message[1..]);
                    server_emitter.emit(ServerEvent::ChatCmd(entity, argv));
                }
            } else {
                // Send chat message
                server_emitter.emit(ServerEvent::Chat(msg));
            }
        }

        timer.end()
    }
}
