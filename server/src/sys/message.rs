use super::SysTimer;
use crate::{
    auth_provider::AuthProvider, client::Client, persistence::character::CharacterLoader,
    ServerSettings, CLIENT_TIMEOUT,
};
use common::{
    comp::{
        CanBuild, ChatMode, ControlEvent, Controller, ForceUpdate, Ori, Player, Pos, Stats, Vel,
    },
    event::{EventBus, ServerEvent},
    msg::{
        validate_chat_msg, CharacterInfo, ChatMsgValidationError, ClientMsg, ClientState,
        PlayerInfo, PlayerListUpdate, RequestStateError, ServerMsg, MAX_BYTES_CHAT_MSG,
    },
    state::{BlockChange, Time},
    sync::Uid,
    terrain::{Block, TerrainChunkSize, TerrainGrid},
    vol::{RectVolSize, Vox},
};
use hashbrown::HashMap;
use specs::{
    Entities, Join, Read, ReadExpect, ReadStorage, System, Write, WriteExpect, WriteStorage,
};

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
        Write<'a, SysTimer<Self>>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, CanBuild>,
        ReadStorage<'a, ForceUpdate>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, ChatMode>,
        WriteExpect<'a, AuthProvider>,
        Write<'a, BlockChange>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Player>,
        WriteStorage<'a, Client>,
        WriteStorage<'a, Controller>,
        WriteStorage<'a, SpeechBubble>,
        Read<'a, ServerSettings>,
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
            mut timer,
            uids,
            can_build,
            force_updates,
            stats,
            chat_modes,
            mut accounts,
            mut block_changes,
            mut positions,
            mut velocities,
            mut orientations,
            mut players,
            mut clients,
            mut controllers,
            mut speech_bubbles,
            settings,
        ): Self::SystemData,
    ) {
        timer.start();

        let mut server_emitter = server_event_bus.emitter();

        let mut new_chat_msgs = Vec::new();

        // Player list to send new players.
        let player_list = (&uids, &players, &stats)
            .join()
            .map(|(uid, player, stats)| {
                ((*uid).into(), PlayerInfo {
                    player_alias: player.alias.clone(),
                    // TODO: player might not have a character selected
                    character: Some(CharacterInfo {
                        name: stats.name.clone(),
                        level: stats.level.level(),
                    }),
                })
            })
            .collect::<HashMap<_, _>>();
        // List of new players to update player lists of all clients.
        let mut new_players = Vec::new();

        for (entity, client) in (&entities, &mut clients).join() {
            let new_msgs = client.postbox.new_messages();

            // Update client ping.
            if new_msgs.len() > 0 {
                client.last_ping = time.0
            } else if time.0 - client.last_ping > CLIENT_TIMEOUT // Timeout
                || client.postbox.error().is_some()
            // Postbox error
            {
                server_emitter.emit(ServerEvent::ClientDisconnect(entity));
            } else if time.0 - client.last_ping > CLIENT_TIMEOUT * 0.5 {
                // Try pinging the client if the timeout is nearing.
                client.postbox.send_message(ServerMsg::Ping);
            }

            // Process incoming messages.
            for msg in new_msgs {
                match msg {
                    // Go back to registered state (char selection screen)
                    ClientMsg::ExitIngame => match client.client_state {
                        // Use ClientMsg::Register instead.
                        ClientState::Connected => {
                            client.error_state(RequestStateError::WrongMessage)
                        },
                        ClientState::Registered => client.error_state(RequestStateError::Already),
                        ClientState::Spectator | ClientState::Character => {
                            server_emitter.emit(ServerEvent::ExitIngame { entity });
                        },
                        ClientState::Pending => {},
                    },
                    // Request spectator state
                    ClientMsg::Spectate => match client.client_state {
                        // Become Registered first.
                        ClientState::Connected => client.error_state(RequestStateError::Impossible),
                        ClientState::Spectator => client.error_state(RequestStateError::Already),
                        ClientState::Registered | ClientState::Character => {
                            client.allow_state(ClientState::Spectator)
                        },
                        ClientState::Pending => {},
                    },
                    // Request registered state (login)
                    ClientMsg::Register {
                        view_distance,
                        token_or_username,
                    } => {
                        let (username, uuid) = match accounts.query(token_or_username.clone()) {
                            Err(err) => {
                                client.error_state(RequestStateError::RegisterDenied(err));
                                break;
                            },
                            Ok((username, uuid)) => (username, uuid),
                        };

                        let vd = view_distance
                            .map(|vd| vd.min(settings.max_view_distance.unwrap_or(vd)));
                        let player = Player::new(username, None, vd, uuid);

                        if !player.is_valid() {
                            // Invalid player
                            client.error_state(RequestStateError::Impossible);
                            break;
                        }

                        match client.client_state {
                            ClientState::Connected => {
                                // Add Player component to this client
                                let _ = players.insert(entity, player);

                                // Tell the client its request was successful.
                                client.allow_state(ClientState::Registered);

                                // Send initial player list
                                client.notify(ServerMsg::PlayerListUpdate(PlayerListUpdate::Init(
                                    player_list.clone(),
                                )));

                                // Add to list to notify all clients of the new player
                                new_players.push(entity);
                            },
                            // Use RequestState instead (No need to send `player` again).
                            _ => client.error_state(RequestStateError::Impossible),
                        }
                        //client.allow_state(ClientState::Registered);

                        // Limit view distance if it's too high
                        // This comes after state registration so that the client actually hears it
                        if settings
                            .max_view_distance
                            .zip(view_distance)
                            .map(|(vd, max)| vd > max)
                            .unwrap_or(false)
                        {
                            client.notify(ServerMsg::SetViewDistance(
                                settings.max_view_distance.unwrap_or(0),
                            ));
                        };
                    },
                    ClientMsg::SetViewDistance(view_distance) => match client.client_state {
                        ClientState::Character { .. } => {
                            if settings
                                .max_view_distance
                                .map(|max| view_distance <= max)
                                .unwrap_or(true)
                            {
                                players.get_mut(entity).map(|player| {
                                    player.view_distance = Some(
                                        settings
                                            .max_view_distance
                                            .map(|max| view_distance.min(max))
                                            .unwrap_or(view_distance),
                                    )
                                });
                            } else {
                                client.notify(ServerMsg::SetViewDistance(
                                    settings.max_view_distance.unwrap_or(0),
                                ));
                            }
                        },
                        _ => {},
                    },
                    ClientMsg::Character(character_id) => match client.client_state {
                        // Become Registered first.
                        ClientState::Connected => client.error_state(RequestStateError::Impossible),
                        ClientState::Registered | ClientState::Spectator => {
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
                                if settings.server_description.len() > 0 {
                                    client.notify(ServerMsg::broadcast(
                                        settings.server_description.clone(),
                                    ));
                                }

                                // Only send login message if it wasn't already
                                // sent previously
                                if !client.login_msg_sent {
                                    new_chat_msgs.push((
                                        None,
                                        ServerMsg::broadcast(format!(
                                            "[{}] is now online.",
                                            &player.alias
                                        )),
                                    ));

                                    client.login_msg_sent = true;
                                }
                            } else {
                                client.notify(ServerMsg::CharacterDataLoadError(String::from(
                                    "Failed to fetch player entity",
                                )))
                            }
                        },
                        ClientState::Character => client.error_state(RequestStateError::Already),
                        ClientState::Pending => {},
                    },
                    ClientMsg::ControllerInputs(inputs) => match client.client_state {
                        ClientState::Connected
                        | ClientState::Registered
                        | ClientState::Spectator => {
                            client.error_state(RequestStateError::Impossible)
                        },
                        ClientState::Character => {
                            if let Some(controller) = controllers.get_mut(entity) {
                                controller.inputs.update_with_new(inputs);
                            }
                        },
                        ClientState::Pending => {},
                    },
                    ClientMsg::ControlEvent(event) => match client.client_state {
                        ClientState::Connected
                        | ClientState::Registered
                        | ClientState::Spectator => {
                            client.error_state(RequestStateError::Impossible)
                        },
                        ClientState::Character => {
                            // Skip respawn if client entity is alive
                            if let &ControlEvent::Respawn = &event {
                                if stats.get(entity).map_or(true, |s| !s.is_dead) {
                                    continue;
                                }
                            }
                            if let Some(controller) = controllers.get_mut(entity) {
                                controller.events.push(event);
                            }
                        },
                        ClientState::Pending => {},
                    },
                    ClientMsg::ControlAction(event) => match client.client_state {
                        ClientState::Connected
                        | ClientState::Registered
                        | ClientState::Spectator => {
                            client.error_state(RequestStateError::Impossible)
                        },
                        ClientState::Character => {
                            if let Some(controller) = controllers.get_mut(entity) {
                                controller.actions.push(event);
                            }
                        },
                        ClientState::Pending => {},
                    },
                    ClientMsg::ChatMsg(message) => match client.client_state {
                        ClientState::Connected => client.error_state(RequestStateError::Impossible),
                        ClientState::Registered
                        | ClientState::Spectator
                        | ClientState::Character => match validate_chat_msg(&message) {
                            Ok(()) => {
                                if let Some(from) = uids.get(entity) {
                                    let mode = chat_modes.get(entity).unwrap_or(&ChatMode::World);
                                    let msg = ServerMsg::chat(*mode, *from, message);
                                    new_chat_msgs.push((Some(entity), msg));
                                } else {
                                    tracing::error!("Could not send message. Missing player uid");
                                }
                            },
                            Err(ChatMsgValidationError::TooLong) => {
                                let max = MAX_BYTES_CHAT_MSG;
                                let len = message.len();
                                tracing::warn!(?len, ?max, "Recieved a chat message that's too long")
                            },
                        },
                        ClientState::Pending => {},
                    },
                    ClientMsg::PlayerPhysics { pos, vel, ori } => match client.client_state {
                        ClientState::Character => {
                            if force_updates.get(entity).is_none()
                                && stats.get(entity).map_or(true, |s| !s.is_dead)
                            {
                                let _ = positions.insert(entity, pos);
                                let _ = velocities.insert(entity, vel);
                                let _ = orientations.insert(entity, ori);
                            }
                        },
                        // Only characters can send positions.
                        _ => client.error_state(RequestStateError::Impossible),
                    },
                    ClientMsg::BreakBlock(pos) => {
                        if can_build.get(entity).is_some() {
                            block_changes.set(pos, Block::empty());
                        }
                    },
                    ClientMsg::PlaceBlock(pos, block) => {
                        if can_build.get(entity).is_some() {
                            block_changes.try_set(pos, block);
                        }
                    },
                    ClientMsg::TerrainChunkRequest { key } => match client.client_state {
                        ClientState::Connected | ClientState::Registered => {
                            client.error_state(RequestStateError::Impossible);
                        },
                        ClientState::Spectator | ClientState::Character => {
                            let in_vd = if let (Some(view_distance), Some(pos)) = (
                                players.get(entity).and_then(|p| p.view_distance),
                                positions.get(entity),
                            ) {
                                pos.0.xy().map(|e| e as f64).distance(
                                    key.map(|e| e as f64 + 0.5)
                                        * TerrainChunkSize::RECT_SIZE.map(|e| e as f64),
                                ) < (view_distance as f64 + 1.5)
                                    * TerrainChunkSize::RECT_SIZE.x as f64
                            } else {
                                true
                            };
                            if in_vd {
                                match terrain.get_key(key) {
                                    Some(chunk) => {
                                        client.postbox.send_message(ServerMsg::TerrainChunkUpdate {
                                            key,
                                            chunk: Ok(Box::new(chunk.clone())),
                                        })
                                    },
                                    None => {
                                        server_emitter.emit(ServerEvent::ChunkRequest(entity, key))
                                    },
                                }
                            }
                        },
                        ClientState::Pending => {},
                    },
                    // Always possible.
                    ClientMsg::Ping => client.postbox.send_message(ServerMsg::Pong),
                    ClientMsg::Pong => {},
                    ClientMsg::Disconnect => {
                        client.postbox.send_message(ServerMsg::Disconnect);
                    },
                    ClientMsg::Terminate => {
                        server_emitter.emit(ServerEvent::ClientDisconnect(entity));
                    },
                    ClientMsg::RequestCharacterList => {
                        if let Some(player) = players.get(entity) {
                            character_loader.load_character_list(entity, player.uuid().to_string())
                        }
                    },
                    ClientMsg::CreateCharacter { alias, tool, body } => {
                        if let Some(player) = players.get(entity) {
                            character_loader.create_character(
                                entity,
                                player.uuid().to_string(),
                                alias,
                                tool,
                                body,
                            );
                        }
                    },
                    ClientMsg::DeleteCharacter(character_id) => {
                        if let Some(player) = players.get(entity) {
                            character_loader.delete_character(
                                entity,
                                player.uuid().to_string(),
                                character_id,
                            );
                        }
                    },
                }
            }
        }

        // Handle new players.
        // Tell all clients to add them to the player list.
        for entity in new_players {
            if let (Some(uid), Some(player)) = (uids.get(entity), players.get(entity)) {
                let msg =
                    ServerMsg::PlayerListUpdate(PlayerListUpdate::Add((*uid).into(), PlayerInfo {
                        player_alias: player.alias.clone(),
                        character: None, // new players will be on character select.
                    }));
                for client in (&mut clients).join().filter(|c| c.is_registered()) {
                    client.notify(msg.clone())
                }
            }
        }

        // Handle new chat messages.
        for (entity, msg) in new_chat_msgs {
            match msg {
                ServerMsg::ChatMsg(msg) => {
                    // Handle chat commands.
                    if msg.message.starts_with("/") {
                        if let (Some(entity), true) = (entity, msg.message.len() > 1) {
                            let argv = String::from(&msg.message[1..]);
                            server_emitter.emit(ServerEvent::ChatCmd(entity, argv));
                        }
                    } else {
                        // TODO FIXME speech bubbles and prefixes are handled by the client now
                        for client in (&mut clients).join().filter(|c| c.is_registered()) {
                            client.notify(ServerMsg::ChatMsg(msg.clone()));
                        }
                    }
                },
                _ => {
                    panic!("Invalid message type.");
                },
            }
        }

        timer.end()
    }
}
