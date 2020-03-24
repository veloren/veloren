use super::SysTimer;
use crate::{auth_provider::AuthProvider, client::Client, CLIENT_TIMEOUT};
use common::{
    comp::{Admin, CanBuild, ControlEvent, Controller, ForceUpdate, Ori, Player, Pos, Stats, Vel},
    event::{EventBus, ServerEvent},
    msg::{
        validate_chat_msg, ChatMsgValidationError, ClientMsg, ClientState, PlayerListUpdate,
        RequestStateError, ServerMsg, MAX_BYTES_CHAT_MSG,
    },
    state::{BlockChange, Time},
    sync::Uid,
    terrain::{Block, TerrainGrid},
    vol::Vox,
};
use hashbrown::HashMap;
use specs::{
    Entities, Join, Read, ReadExpect, ReadStorage, System, Write, WriteExpect, WriteStorage,
};

/// This system will handle new messages from clients
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, Time>,
        ReadExpect<'a, TerrainGrid>,
        Write<'a, SysTimer<Self>>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, CanBuild>,
        ReadStorage<'a, Admin>,
        ReadStorage<'a, ForceUpdate>,
        ReadStorage<'a, Stats>,
        WriteExpect<'a, AuthProvider>,
        Write<'a, BlockChange>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Player>,
        WriteStorage<'a, Client>,
        WriteStorage<'a, Controller>,
    );

    fn run(
        &mut self,
        (
            entities,
            server_event_bus,
            time,
            terrain,
            mut timer,
            uids,
            can_build,
            admins,
            force_updates,
            stats,
            mut accounts,
            mut block_changes,
            mut positions,
            mut velocities,
            mut orientations,
            mut players,
            mut clients,
            mut controllers,
        ): Self::SystemData,
    ) {
        timer.start();

        let time = time.0;

        let mut server_emitter = server_event_bus.emitter();

        let mut new_chat_msgs = Vec::new();

        // Player list to send new players.
        let player_list = (&uids, &players)
            .join()
            .map(|(uid, player)| ((*uid).into(), player.alias.clone()))
            .collect::<HashMap<_, _>>();
        // List of new players to update player lists of all clients.
        let mut new_players = Vec::new();

        for (entity, client) in (&entities, &mut clients).join() {
            let new_msgs = client.postbox.new_messages();

            // Update client ping.
            if new_msgs.len() > 0 {
                client.last_ping = time
            } else if time - client.last_ping > CLIENT_TIMEOUT // Timeout
                || client.postbox.error().is_some()
            // Postbox error
            {
                server_emitter.emit(ServerEvent::ClientDisconnect(entity));
            } else if time - client.last_ping > CLIENT_TIMEOUT * 0.5 {
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

                        let player = Player::new(username, view_distance, uuid);

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
                    },
                    ClientMsg::SetViewDistance(view_distance) => match client.client_state {
                        ClientState::Character { .. } => {
                            players
                                .get_mut(entity)
                                .map(|player| player.view_distance = Some(view_distance));
                        },
                        _ => {},
                    },
                    ClientMsg::Character { name, body, main } => match client.client_state {
                        // Become Registered first.
                        ClientState::Connected => client.error_state(RequestStateError::Impossible),
                        ClientState::Registered | ClientState::Spectator => {
                            if let (Some(player), false) = (
                                players.get(entity),
                                // Only send login message if it wasn't already sent
                                // previously
                                client.login_msg_sent,
                            ) {
                                new_chat_msgs.push((
                                    None,
                                    ServerMsg::broadcast(format!(
                                        "[{}] is now online.",
                                        &player.alias
                                    )),
                                ));
                                client.login_msg_sent = true;
                            }

                            server_emitter.emit(ServerEvent::CreateCharacter {
                                entity,
                                name,
                                body,
                                main,
                            });
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
                    ClientMsg::ChatMsg { message } => match client.client_state {
                        ClientState::Connected => client.error_state(RequestStateError::Impossible),
                        ClientState::Registered
                        | ClientState::Spectator
                        | ClientState::Character => match validate_chat_msg(&message) {
                            Ok(()) => new_chat_msgs.push((Some(entity), ServerMsg::chat(message))),
                            Err(ChatMsgValidationError::TooLong) => log::warn!(
                                "Recieved a chat message that's too long (max:{} len:{})",
                                MAX_BYTES_CHAT_MSG,
                                message.len()
                            ),
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
                            match terrain.get_key(key) {
                                Some(chunk) => {
                                    client.postbox.send_message(ServerMsg::TerrainChunkUpdate {
                                        key,
                                        chunk: Ok(Box::new(chunk.clone())),
                                    })
                                },
                                None => server_emitter.emit(ServerEvent::ChunkRequest(entity, key)),
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
                }
            }
        }

        // Handle new players.
        // Tell all clients to add them to the player list.
        for entity in new_players {
            if let (Some(uid), Some(player)) = (uids.get(entity), players.get(entity)) {
                let msg = ServerMsg::PlayerListUpdate(PlayerListUpdate::Add(
                    (*uid).into(),
                    player.alias.clone(),
                ));
                for client in (&mut clients).join().filter(|c| c.is_registered()) {
                    client.notify(msg.clone())
                }
            }
        }

        // Handle new chat messages.
        for (entity, msg) in new_chat_msgs {
            match msg {
                ServerMsg::ChatMsg { chat_type, message } => {
                    if let Some(entity) = entity {
                        // Handle chat commands.
                        if message.starts_with("/") && message.len() > 1 {
                            let argv = String::from(&message[1..]);
                            server_emitter.emit(ServerEvent::ChatCmd(entity, argv));
                        } else {
                            let message = match players.get(entity) {
                                Some(player) => {
                                    if admins.get(entity).is_some() {
                                        format!("[ADMIN][{}] {}", &player.alias, message)
                                    } else {
                                        format!("[{}] {}", &player.alias, message)
                                    }
                                },
                                None => format!("[<Unknown>] {}", message),
                            };
                            let msg = ServerMsg::ChatMsg { chat_type, message };
                            for client in (&mut clients).join().filter(|c| c.is_registered()) {
                                client.notify(msg.clone());
                            }
                        }
                    } else {
                        let msg = ServerMsg::ChatMsg { chat_type, message };
                        for client in (&mut clients).join().filter(|c| c.is_registered()) {
                            client.notify(msg.clone());
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
