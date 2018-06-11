
use std::net::{SocketAddr, ToSocketAddrs};
use std::collections::HashMap;

use nalgebra::Vector3;

use network::ClientMode;
use network::server::ServerConn;
use network::packet::{ClientPacket, ServerPacket};
use world::World;
use player::Player;
use region::Entity;
use common::Clock;
use std::time::Duration;
use common::get_version;
use common::Uid;

pub struct Server {
    running: bool,
    clock: Clock,

    uid_count: Uid,
    world: World,
    entities: HashMap<Uid, Entity>,

    conn: ServerConn,
    players: HashMap<SocketAddr, Player>,
}

impl Server {
    pub fn new<A: ToSocketAddrs>(bind_addr: A, seed: u32, world_size: u32) -> Option<Server> {
        let server = Server {
            running: true,
            clock: Clock::new(),

            uid_count: 0,
            world: World::new(seed, world_size),
            entities: HashMap::new(),

            conn: match ServerConn::new(bind_addr) {
                Ok(c) => c,
                Err(_) => return None, // TODO: Handle errors correctly
            },
            players: HashMap::new(),
        };

        Some(server)
    }

    pub fn running(&self) -> bool {
        self.running
    }

    pub fn conn(&self) -> ServerConn {
        self.conn.clone()
    }

    pub fn handle_packet(&mut self, data: (SocketAddr, ClientPacket)) {
        let (sock_addr, packet) = data;

        match packet {
            ClientPacket::Connect { mode, alias, version } => {
                match version == get_version() {
                    true => {
                        self.handle_player_connect(sock_addr, mode, alias);
                    },
                    false => {
                        info!("Player attempted to connect with {} but was rejected due to incompatible version ({})", alias, version);
                        let _ = self.conn.send_to(sock_addr, &ServerPacket::Kicked { reason: format!("Incompatible version! Server is running version ({})", get_version()) });
                    },
                }

            },
            ClientPacket::Disconnect => {
                match self.players.remove(&sock_addr) {
                    Some(p) => info!("Player '{}' disconnected!", p.alias()),
                    None => warn!("A player attempted to disconnect without being connected"),
                }
            },
            ClientPacket::Ping => {
                if self.players.contains_key(&sock_addr) {
                    let _ = self.conn.send_to(sock_addr, &ServerPacket::Ping);
                } else {
                    warn!("A ping was received from an unconnected player");
                }
            },
            ClientPacket::SendChatMsg { msg } => {
                match self.players.get(&sock_addr) {
                    Some(p) => {
                        let alias = p.alias().to_string();

                        info!("[MSG] {}: {}", alias, msg);

                        let packet = ServerPacket::RecvChatMsg{ alias, msg };
                        for sock_addr in self.players.keys() {
                            let _ = self.conn.send_to(sock_addr, &packet);
                        }
                    },
                    None => {},
                };

            },
            ClientPacket::SendCmd { cmd } => self.handle_command(&sock_addr, cmd),
            ClientPacket::PlayerEntityUpdate { pos } => {
                if let Some(ref mut p) = self.players.get_mut(&sock_addr) {
                    // TODO: Check this movement is acceptable.
                    match self.entities.get_mut(&p.entity_uid().unwrap()) {
                        Some(e) => {
                            *e.pos_mut() = pos;
                        },
                        None => {},
                    }
                }
            },
        }
    }

    pub fn new_uid(&mut self) -> Uid {
        self.uid_count += 1;
        self.uid_count
    }

    pub fn add_entity(&mut self, entity: Entity) -> Uid {
        let uid = self.new_uid();
        self.entities.insert(uid, entity);
        uid
    }

    pub fn next_tick(&mut self, dt: Duration) {
        //self.world.tick(dt); // TODO: Fix issue #11 and uncomment
        self.clock.tick(dt);
        debug!("TICK!");
        // Send Entity Updates
        // For each entity
        for (uid, entity) in self.entities.iter() {
            // Send their Entity data
            let packet = ServerPacket::EntityUpdate{ uid: *uid, pos: *entity.pos() };
            // To OTHER players.
            for (sock_addr, player) in self.players.iter() {
                // Check that the player has an entity
                if let Some(player_entity_uid) = player.entity_uid() {
                    // Check we aren't telling the player his own entity data
                    if player_entity_uid != *uid {
                        debug!("Sending update of entity [{}] to {}!", uid, player.alias());
                        let _ = self.conn.send_to(sock_addr, &packet);
                    }
                }
            }
        }
    }

    fn handle_player_connect(&mut self, sock_addr: SocketAddr, mode: ClientMode, alias: String) {
        match self.players.get(&sock_addr) {
            Some(p) => warn!("Player '{}' tried to connect twice with the new alias '{}'", p.alias(), &alias),
            None => {
                let player_entity_uid = match mode {
                    ClientMode::Headless => {
                        info!("Player '{}' connected in headless mode.", alias);
                        None
                    },
                    ClientMode::Character => {
                        let player_entity = self.add_entity(Entity::new(Vector3::new(0.0, 0.0, 0.0)));
                        info!("Player '{}' connected in character mode. Assigned entity uid: {}", alias, player_entity);
                        Some(player_entity)
                    },
                };

                self.players.insert(sock_addr, Player::new(player_entity_uid, mode, &alias));

                let _ = self.conn.send_to(sock_addr, &ServerPacket::Connected { player_entity_uid, version: get_version() });
            },
        }
    }

    fn handle_command(&mut self, sock_addr: &SocketAddr, command_str: String) {
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
    }
}

fn handle_move_packet(entity: &mut Entity, str_args: Vec<&str>) -> String {
        let args: Vec<f32> = str_args.iter()
                            .filter_map(|arg| arg.parse::<f32>().ok())
                            .collect();
        let response: String;
        if args.len() == 3 { // Check we have the right number of args
            let x = args[0];
            let y = args[1];
            let z = args[2];
            *entity.pos_mut() = (Vector3::new(x, y, z));
            response = String::from("Moved successfully!")
        } else {
            // Handle invalid number of args?
            warn!("Invalid number of arguments for move_by command");
            response = String::from("Invalid number of arguments for move_by command!");
        }
        response
    }

impl Drop for Server {
    fn drop(&mut self) {
        for (sock_addr, player) in &self.players {
            self.conn.send_to(sock_addr, &ServerPacket::Shutdown).unwrap_or_else(|e| {
                error!("Failed to send shutdown packet to player '{}' ({}): {:?}", player.alias(), sock_addr.to_string(), e);
            });
        }
    }
}
