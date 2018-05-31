extern crate worldgen;
extern crate worldsim;
extern crate network;

mod server;
mod player;

use std::time;
use std::thread;
use std::sync::{Mutex, Arc};
use std::net::ToSocketAddrs;

pub struct ServerHandle {
    server: Arc<Mutex<server::Server>>,
}

impl ServerHandle {
    pub fn new<A: ToSocketAddrs>(bind_addr: A, seed: u32, world_size: u32) -> Option<ServerHandle> {
        Some(ServerHandle {
            server: match server::Server::new(bind_addr, seed, world_size) {
                Some(s) => s,
                None => return None,
            },
        })
    }

    pub fn run(&mut self) {
        let server_ref = self.server.clone();
        thread::spawn(move || {
            while server_ref.lock().unwrap().running() {
                server_ref.lock().unwrap().handle_packet();
            }
        });

        while self.server.lock().unwrap().running() {
            self.server.lock().unwrap().next_tick(0.20);
            thread::sleep(time::Duration::from_millis(20));
        }
    }
}
