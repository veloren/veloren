// Library
use bifrost::Relay;
use coord::prelude::*;

// Project
use common::get_version;
use common::net::ClientMode;
use common::net::{ClientPacket, ServerPacket};
use region::Entity;

// Local
use player::Player;
use server_context::ServerContext;

pub fn handle_packet(relay: &Relay<ServerContext>, world: &mut ServerContext, session_id: u32, packet: ClientPacket) {
    match packet {
        ClientPacket::Connect { mode, ref alias, ref version } => {
            match *version == get_version() {
                true => {
                    let entity_uid = match mode {
                        ClientMode::Headless => {
                            info!("Player '{}' connected in headless mode.", alias);
                            None
                        },
                        ClientMode::Character => {
                            let uid = world.new_uid();
                            info!("Player '{}' connected in character mode. Assigned entity uid: {}", alias, uid);
                            world.add_entity(uid, box Entity::new(vec3!(0.0, 0.0, 60.0), 0.0));
                            Some(uid)
                        }
                    };

                    let player_uid = world.new_uid();
                    debug!("Player got playid {}", player_uid);
                    world.add_player(box Player::new(session_id, player_uid, entity_uid, &alias));
                    world.get_session_mut(session_id).unwrap().set_player_id(Some(player_uid));

                    world.send_packet(
                        session_id,
                        &ServerPacket::Connected { entity_uid, version: get_version() }
                    );
                }
                false => {
                    info!("Player attempted to connect with {} but was rejected due to incompatible version ({})", alias, version);
                    world.send_packet(
                        session_id,
                        &ServerPacket::Kicked { reason: format!("Incompatible version! Server is running version ({})", get_version()) }
                    );
                }
            }
        },
        ClientPacket::Disconnect => {
            world.kick_session(session_id);
        },
        ClientPacket::Ping => {
            world.send_packet(
                session_id,
                &ServerPacket::Ping
            );
        },
        ClientPacket::ChatMsg { msg } => {
            if let Some(ref mut player) = world.get_session(session_id)
                .and_then(|it| it.get_player_id())
                .and_then(|id| world.get_player(id)) {

                let alias = player.alias().to_string();
                debug!("[MSG] {}: {}", alias, &msg);
                world.broadcast_packet(&ServerPacket::RecvChatMsg { alias, msg: msg.to_string() });
            }
        },
        ClientPacket::SendCmd { ref cmd } => handle_command(relay, world, session_id, cmd.to_string()),
        ClientPacket::PlayerEntityUpdate { pos, ori } => {
            if let Some(ref player) = world.get_session(session_id)
                .and_then(|it| it.get_player_id())
                .and_then(|id| world.get_player(id)) {

                let player_name = player.alias().to_string();

                if let Some(entity_uid) = player.get_entity_uid() {
                    if let Some(e) = world.get_entity(entity_uid) {
                        let dist = (e.pos() - pos).length();
                        if dist > 5.0 {
                            info!("player: {} moved to fast, resetting him", player_name);
                            world.send_packet(
                                session_id,
                                &ServerPacket::EntityUpdate { uid: entity_uid, pos: e.pos(), ori: e.ori() }
                            );
                        } else {
                            *e.pos_mut() = pos;
                            *e.ori_mut() = ori;
                        }
                    }
                }
            }
        },
    }
}


fn handle_command(relay: &Relay<ServerContext>, world: &mut ServerContext, session_id: u32, command_str: String) {
    /*
    // TODO: Implement some sort of command structure with a hashmap of Commands.
    if let Some(p) = self.players.get(&sock_addr) {
        debug!("Received command from {}: {}", p.alias(), command_str);
        // Split command into parts, seperated by space.
        let mut parts = command_str.split(" ");
        if let Some(command) = parts.next() {
            let response: String;
            match command {
                "move_by" => {
                    let str_args = parts.collect::<Vec<&str>>();

                    match p.entity_uid() {
                        Some(entity_id) => match self.entities.get_mut(&entity_id) {
                            Some(entity) => {
                                // TODO: Parse these args without discarding non f32 elements.


                                response = handle_move_packet(entity, str_args);
                            },
                            None => {
                                debug!("Entity does not exist within hashmap.");
                                response = String::from("You do not have an entity to move!.");
                            },
                        },
                        None => {
                            debug!("Player does not have entity to move.");
                            response = String::from("You do not have an entity to move!.");
                        },
                    }
                },
                _ => response = String::from("Command not recognised..."),
            };
            let packet = ServerPacket::RecvChatMsg{alias: String::from("Server"), msg: response};
            let _ = self.conn.send_to(sock_addr, &packet);
        }
    }
    */
}
