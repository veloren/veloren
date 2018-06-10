use bifrost::relay::Relay;
use common::get_version;
use common::network::ClientMode;
use common::network::packet::{ClientPacket, ServerPacket};
use nalgebra::Vector3;
use player::Player;
use region::Entity;
use world_context::World;

pub fn handle_packet(relay: &Relay<World>, world: &mut World, session_id: u32, packet: &ClientPacket) {
    match packet {
        &ClientPacket::Connect { mode, ref alias, ref version } => {
            match *version == get_version() {
                true => {
                    let entity_id = match mode {
                        ClientMode::Headless => {
                            println!("Player '{}' connected in headless mode.", alias);
                            None
                        },
                        ClientMode::Character => {
                            let uid = world.new_uid();
                            println!("Player '{}' connected in character mode. Assigned entity uid: {}", alias, uid);
                            world.add_entity(uid, box Entity::new(Vector3::new(0.0, 0.0, 0.0)));
                            Some(uid)
                        }
                    };

                    let player_uid = world.new_uid();
                    println!("Player got playid {}", player_uid);
                    world.add_player(box Player::new(session_id, player_uid, entity_id, &alias));
                    world.get_session(session_id).unwrap().set_player_id(Some(player_uid));

                    world.get_session(session_id).map(|it| it.get_handler().send_packet(
                        &ServerPacket::Connected { entity_uid: entity_id, version: get_version() }
                    ));
                }
                false => {
                    println!("Player attempted to connect with {} but was rejected due to incompatible version ({})", alias, version);
                    world.get_session(session_id).map(|it| it.get_handler().send_packet(
                        &ServerPacket::Kicked { reason: format!("Incompatible version! Server is running version ({})", get_version()) }
                    ));
                }
            }
        }
        &ClientPacket::Disconnect => {
            match world.del_session(session_id) {
                Some(session) => {
                    session.get_player_id().map(|it| {
                        match world.del_player(it) {
                            None => (),
                            Some(player) => println!("Player '{}' disconnected!", player.alias()),
                        }
                    });
                },
                None => println!("A player attempted to disconnect without being connected"),
            }
        }
        &ClientPacket::Ping => {
            world.get_session(session_id).map(|it| it.get_handler().send_packet(
                &ServerPacket::Ping
            ));
        }
        ClientPacket::ChatMsg { msg } => {
            if let Some(ref mut player) = world.get_session(session_id)
                .and_then(|it| it.get_player_id())
                .and_then(|id| world.get_player(id)) {

                let alias = player.alias().to_string();
                println!("[MSG] {}: {}", alias, &msg);
                let packet = ServerPacket::RecvChatMsg { alias, msg: msg.to_string() };
                world.broadcast_packet(&packet);
            }
        }
        &ClientPacket::SendCmd { ref cmd } => handle_command(relay, world, session_id, cmd.to_string()),
        &ClientPacket::PlayerEntityUpdate { pos } => {
            if let Some(ref player) = world.get_session(session_id)
                .and_then(|it| it.get_player_id())
                .and_then(|id| world.get_player(id)) {

                if let Some(entity_id) = player.get_entity_id() {
                    world.get_entity(entity_id).map(|e| *e.pos_mut() = pos);
                }

            }
        }
    }
}


fn handle_command(relay: &Relay<World>, world: &mut World, session_id: u32, command_str: String) {
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
