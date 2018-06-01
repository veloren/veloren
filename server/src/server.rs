use std::net::{SocketAddr, ToSocketAddrs};
use std::collections::HashMap;

use network::server::ServerConn;
use network::packet::{ClientPacket, ServerPacket};
use world::World;
use player::Player;

pub struct Server {
    running: bool,
    time: f64,
    world: World,

    conn: ServerConn,
    players: HashMap<SocketAddr, Player>,
}

impl Server {
    pub fn new<A: ToSocketAddrs>(bind_addr: A, seed: u32, world_size: u32) -> Option<Server> {
        let server = Server {
            running: true,
            time: 0.0,
            world: World::new(seed, world_size),
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
            ClientPacket::Connect { alias } => {
                if self.players.contains_key(&sock_addr) {
                    match self.players.get(&sock_addr) {
                        Some(p) => println!("[WARNING] Player '{}' tried to connect twice with the new alias '{}'", p.alias(), &alias),
                        None => {},
                    }
                } else {
                    self.players.insert(sock_addr, Player::new(&alias));
                    println!("[INFO] Player '{}' connected!", alias);
                }
            },
            ClientPacket::Disconnect => {
                match self.players.remove(&sock_addr) {
                    Some(p) => println!("[INFO] Player '{}' disconnected!", p.alias()),
                    None => println!("[WARNING] A player attempted to disconnect without being connected"),
                }
            },
            ClientPacket::Ping => {
                if self.players.contains_key(&sock_addr) {
                    self.conn.send_to(sock_addr, &ServerPacket::Ping);
                } else {
                    println!("[WARNING] A ping was received from an unconnected player");
                }
            },
            ClientPacket::SendChatMsg { msg } => {
                if self.players.contains_key(&sock_addr) {
                    let alias = match self.players.get(&sock_addr) {
                        Some(p) => p.alias().to_string(),
                        None => "<unknown>".to_string(),
                    };

                    println!("[MSG] {}: {}", alias, msg);

                    let packet = ServerPacket::RecvChatMsg{ alias, msg };

                    for sock_addr in self.players.keys() {
                        self.conn.send_to(sock_addr, &packet);
                    }
                }
            },
        }
    }

    pub fn next_tick(&mut self, dt: f64) {
        self.world.tick(dt);
        self.time += dt;
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        for sock_addr in self.players.keys() {
            self.conn.send_to(sock_addr, &ServerPacket::Shutdown);
        }
    }
}
