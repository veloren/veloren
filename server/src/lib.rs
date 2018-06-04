#![feature(nll)]
#[macro_use]
extern crate log;
extern crate world;
extern crate network;
extern crate nalgebra;

mod server;
mod player;

use std::time;
use std::thread;
use std::sync::{Mutex, Arc};
use std::net::ToSocketAddrs;

// Reexports
pub use network::ClientMode;

pub struct ServerHandle {
    server: Arc<Mutex<server::Server>>,
}

impl ServerHandle {
    pub fn new<A: ToSocketAddrs>(bind_addr: A, seed: u32, world_size: u32) -> Option<ServerHandle> {
        Some(ServerHandle {
            server: Arc::new(Mutex::new(match server::Server::new(bind_addr, seed, world_size) {
                Some(s) => s,
                None => return None,
            })),
        })
    }

    pub fn run(&mut self) {
        let server_ref = self.server.clone();
        thread::spawn(move || {
            let mut conn = server_ref.lock().unwrap().conn();
            while server_ref.lock().unwrap().running() {
                match conn.recv() {
                    Ok(data) => server_ref.lock().unwrap().handle_packet(data),
                    Err(_) => {}, // There was a packet error, but there's nothing we can or should do about it
                }
            }
        });

        while self.server.lock().unwrap().running() {
            self.server.lock().unwrap().next_tick(0.20);
            thread::sleep(time::Duration::from_millis(20));
        }
    }
}
