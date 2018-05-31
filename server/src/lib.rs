extern crate worldgen;
extern crate worldsim;
extern crate network;

use std::{thread, time};
use std::sync::{Mutex, Arc};
use std::net::ToSocketAddrs;
use network::server::ServerConn;
use worldgen::MacroWorld;

struct Server {
    running: bool,
    conn: ServerConn,
    mw: MacroWorld,
}

impl Server {
    pub fn new<A: ToSocketAddrs>(bind_addr: A, seed: u32, world_size: u32) -> Option<Arc<Mutex<Server>>> {
        let server = Arc::new(Mutex::new(Server {
            running: true,
            conn: match ServerConn::new(bind_addr) {
                Ok(c) => c,
                Err(_) => return None, // TODO: Handle errors correctly
            },
            mw: MacroWorld::new(seed, world_size),
        }));

        Some(server)
    }

    fn handle_packet(&mut self) {
        let packet = self.conn.recv();
        println!("Received packet: {:?}", packet);
    }

    fn next_tick(&mut self) {
        worldsim::simulate(&mut self.mw, 1);
        thread::sleep(time::Duration::from_millis(20));
    }
}

pub struct ServerHandle {
    server: Arc<Mutex<Server>>,
}

impl ServerHandle {
    pub fn new<A: ToSocketAddrs>(bind_addr: A, seed: u32, world_size: u32) -> Option<ServerHandle> {
        Some(ServerHandle {
            server: match Server::new(bind_addr, seed, world_size) {
                Some(s) => s,
                None => return None,
            },
        })
    }

    pub fn run(&mut self) {
        let server_ref = self.server.clone();
        thread::spawn(move || {
            while server_ref.lock().unwrap().running {
                server_ref.lock().unwrap().handle_packet();
            }
        });

        while self.server.lock().unwrap().running {
            self.server.lock().unwrap().next_tick();
        }
    }
}
