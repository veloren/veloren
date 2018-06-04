
use std::net::{SocketAddr, ToSocketAddrs};
use std::collections::HashMap;

use network::ClientMode;
use network::server::ServerConn;
use network::packet::{ClientPacket, ServerPacket};
use world::World;
use player::Player;
use region::Entity;

pub struct Server {
    running: bool,
    time: f64,

    uid_count: u64, // TODO: Turn u64 into Uid
    world: World,
    entities: HashMap<u64, Entity>, // TODO: Turn u64 into Uid

    conn: ServerConn,
    players: HashMap<SocketAddr, Player>,
}

impl Server {
    pub fn new<A: ToSocketAddrs>(bind_addr: A, seed: u32, world_size: u32) -> Option<Server> {
        let server = Server {
            running: true,
            time: 0.0,

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
            ClientPacket::Connect { mode, alias } => {
                if self.players.contains_key(&sock_addr) {
                    match self.players.get(&sock_addr) {
                        Some(p) => warn!("Player '{}' tried to connect twice with the new alias '{}'", p.alias(), &alias),
                        None => {},
                    }
                } else {
                    self.players.insert(sock_addr, Player::new(mode, &alias, 0.0, 0.0, 0.0));
                    info!("Player '{}' connected!", alias);

                    let pe = match mode {
                        ClientMode::Headless => None,
                        ClientMode::Character => Some(self.add_entity(Entity::new())),
                    };

                    let _ = self.conn.send_to(sock_addr, &ServerPacket::Connected { player_entity: pe });
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
                if self.players.contains_key(&sock_addr) {
                    let alias = match self.players.get(&sock_addr) {
                        Some(p) => p.alias().to_string(),
                        None => "<unknown>".to_string(),
                    };

                    info!("[MSG] {}: {}", alias, msg);

                    let packet = ServerPacket::RecvChatMsg{ alias, msg };

                    for sock_addr in self.players.keys() {
                        let _ = self.conn.send_to(sock_addr, &packet);
                    }
                }
            },
            ClientPacket::SendCommand { cmd } => {
                if self.players.contains_key(&sock_addr) {
                    // Surely this can be cleaned up?
                    if let Some(p) = self.players.get(&sock_addr) {
                        debug!("Received command from {}: {}", p.alias(), cmd);
                    }
                    self.handle_command(&sock_addr, cmd);
                }
            },
            ClientPacket::PlayerEntityUpdate { pos } => {
                // TODO: Implement
            },
        }
    }

    pub fn new_uid(&mut self) -> u64 {
        self.uid_count += 1;
        self.uid_count
    }

    pub fn add_entity(&mut self, entity: Entity) -> u64 {
        let uid = self.new_uid();
        self.entities.insert(uid, entity);
        uid
    }

    pub fn next_tick(&mut self, dt: f64) {
        self.world.tick(dt);
        self.time += dt;
    }

    fn handle_command(&mut self, sock_addr: &SocketAddr, command_str: String) {
        // TODO: Implement some sort of command structure with a hashmap of Commands.
        let players = &mut self.players;
        if let Some(ref mut p) = players.get_mut(&sock_addr) {
            // Split command into parts, seperated by space.
            let mut parts = command_str.split(" ");
            if let Some(command) = parts.next() {
                let response = match command {
                    "move_by" => {
                        let str_args = parts.collect::<Vec<&str>>();
                        handle_move_by_command(p, str_args)
                    },
                    _ => String::from("Command not recognised...")
                };
                let packet = ServerPacket::RecvChatMsg{alias: String::from("Server"), msg: response};
                let _ = self.conn.send_to(sock_addr, &packet);
            }
        }
    }
}

fn handle_move_by_command<'a>(p: &'a mut Player, str_args: Vec<&str>) -> String {
    // Collect args as f32, if one of the str_args fails to convert, it is dropped.
    // Potential issue as the command below is valid due to the dropped a.
    // !move_by 5 a 3 2
    // TODO: Do some smarter error checking.
    let args: Vec<f32> = str_args.iter()
        .filter_map(|arg| arg.parse::<f32>().ok())
        .collect();

    if args.len() == 3 { // Check we have the right number of args
        let x = args[0];
        let y = args[1];
        let z = args[2];
        p.move_by(x, y, z);

        info!("Moved player {} to {:#?}", p.alias(), p.position());
        format!("Moved to {:#?}", p.position())
    } else {
        // Handle invalid number of args?
        String::from("Invalid number of arguments for move_by command")
    }
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
