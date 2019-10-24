use super::SysTimer;
use crate::{auth_provider::AuthProvider, client::Client, CLIENT_TIMEOUT};
use common::{
    assets,
    comp::{Admin, Body, CanBuild, Controller, Ori, Player, Pos, Vel},
    event::{EventBus, ServerEvent},
    msg::{validate_chat_msg, ChatMsgValidationError, MAX_BYTES_CHAT_MSG},
    msg::{ClientMsg, ClientState, RequestStateError, ServerMsg},
    state::{BlockChange, Time},
    terrain::{Block, TerrainGrid},
    vol::Vox,
};
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
        ReadStorage<'a, Body>,
        ReadStorage<'a, CanBuild>,
        ReadStorage<'a, Admin>,
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
            server_emitter,
            time,
            terrain,
            mut timer,
            bodies,
            can_build,
            admins,
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

        let mut new_chat_msgs = Vec::new();

        for (entity, client) in (&entities, &mut clients).join() {
            let mut disconnect = false;
            let new_msgs = client.postbox.new_messages();

            // Update client ping.
            if new_msgs.len() > 0 {
                client.last_ping = time
            } else if time - client.last_ping > CLIENT_TIMEOUT // Timeout
                || client.postbox.error().is_some()
            // Postbox error
            {
                disconnect = true;
            } else if time - client.last_ping > CLIENT_TIMEOUT * 0.5 {
                // Try pinging the client if the timeout is nearing.
                client.postbox.send_message(ServerMsg::Ping);
            }

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
                            ClientState::Spectator | ClientState::Character | ClientState::Dead => {
                                client.allow_state(ClientState::Registered)
                            }
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
                                let _ = players.insert(entity, player);

                                // Tell the client its request was successful.
                                client.allow_state(ClientState::Registered);
                            }
                            // Use RequestState instead (No need to send `player` again).
                            _ => client.error_state(RequestStateError::Impossible),
                        }
                        //client.allow_state(ClientState::Registered);
                    }
                    // Invalid player
                    ClientMsg::Register { .. } => client.error_state(RequestStateError::Impossible),
                    ClientMsg::SetViewDistance(view_distance) => match client.client_state {
                        ClientState::Character { .. } => {
                            players
                                .get_mut(entity)
                                .map(|player| player.view_distance = Some(view_distance));
                        }
                        _ => {}
                    },
                    ClientMsg::Character { name, body, main } => match client.client_state {
                        // Become Registered first.
                        ClientState::Connected => client.error_state(RequestStateError::Impossible),
                        ClientState::Registered | ClientState::Spectator | ClientState::Dead => {
                            if let (Some(player), None) = (
                                players.get(entity),
                                // Only send login message if the player didn't have a body
                                // previously
                                bodies.get(entity),
                            ) {
                                new_chat_msgs.push((
                                    None,
                                    ServerMsg::broadcast(format!(
                                        "[{}] is now online.",
                                        &player.alias
                                    )),
                                ));
                            }

                            server_emitter.emit(ServerEvent::CreatePlayer {
                                entity,
                                name,
                                body,
                                main: main
                                    .and_then(|specifier| assets::load_cloned(&specifier).ok()),
                            });
                        }
                        ClientState::Character => client.error_state(RequestStateError::Already),
                        ClientState::Pending => {}
                    },
                    ClientMsg::ControllerInputs(inputs) => match client.client_state {
                        ClientState::Connected
                        | ClientState::Registered
                        | ClientState::Spectator => {
                            client.error_state(RequestStateError::Impossible)
                        }
                        ClientState::Dead | ClientState::Character => {
                            if let Some(controller) = controllers.get_mut(entity) {
                                controller.inputs = inputs;
                            }
                        }
                        ClientState::Pending => {}
                    },
                    ClientMsg::ControlEvent(event) => match client.client_state {
                        ClientState::Connected
                        | ClientState::Registered
                        | ClientState::Spectator => {
                            client.error_state(RequestStateError::Impossible)
                        }
                        ClientState::Dead | ClientState::Character => {
                            if let Some(controller) = controllers.get_mut(entity) {
                                controller.events.push(event);
                            }
                        }
                        ClientState::Pending => {}
                    },
                    ClientMsg::ChatMsg { chat_type, message } => match client.client_state {
                        ClientState::Connected => client.error_state(RequestStateError::Impossible),
                        ClientState::Registered
                        | ClientState::Spectator
                        | ClientState::Dead
                        | ClientState::Character => match validate_chat_msg(&message) {
                            Ok(()) => new_chat_msgs
                                .push((Some(entity), ServerMsg::ChatMsg { chat_type, message })),
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
                            let _ = positions.insert(entity, pos);
                            let _ = velocities.insert(entity, vel);
                            let _ = orientations.insert(entity, ori);
                        }
                        // Only characters can send positions.
                        _ => client.error_state(RequestStateError::Impossible),
                    },
                    ClientMsg::BreakBlock(pos) => {
                        if can_build.get(entity).is_some() {
                            block_changes.set(pos, Block::empty());
                        }
                    }
                    ClientMsg::PlaceBlock(pos, block) => {
                        if can_build.get(entity).is_some() {
                            block_changes.try_set(pos, block);
                        }
                    }
                    ClientMsg::TerrainChunkRequest { key } => match client.client_state {
                        ClientState::Connected | ClientState::Registered | ClientState::Dead => {
                            client.error_state(RequestStateError::Impossible);
                        }
                        ClientState::Spectator | ClientState::Character => {
                            match terrain.get_key(key) {
                                Some(chunk) => {
                                    client.postbox.send_message(ServerMsg::TerrainChunkUpdate {
                                        key,
                                        chunk: Ok(Box::new(chunk.clone())),
                                    })
                                }
                                None => server_emitter.emit(ServerEvent::ChunkRequest(entity, key)),
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

            if disconnect {
                if let (Some(player), Some(_)) = (
                    players.get(entity),
                    // It only shows a message if you had a body (not in char selection)
                    bodies.get(entity),
                ) {
                    new_chat_msgs.push((
                        None,
                        ServerMsg::broadcast(format!("{} went offline.", &player.alias)),
                    ));
                }
                server_emitter.emit(ServerEvent::ClientDisconnect(entity));
                client.postbox.send_message(ServerMsg::Disconnect);
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
                                }
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
                }
                _ => {
                    panic!("Invalid message type.");
                }
            }
        }

        timer.end()
    }
}
