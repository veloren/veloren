extern crate worldgen;
extern crate worldsim;
extern crate network;

use std::{thread, time};
use network::server::ServerConn;
use worldgen::MacroWorld;

pub struct Server {
    conn: ServerConn,
    mw: MacroWorld,
}

impl Server {
    pub fn new(bind_addr: &str, seed: u32, world_size: u32) -> Option<Server> {
        Some(Server {
            conn: match ServerConn::new(bind_addr) {
                Ok(c) => c,
                Err(_) => return None, // TODO: Handle errors correctly
            },
            mw: MacroWorld::new(seed, world_size),
        })
    }

    pub fn next_tick(&mut self) -> bool {
        worldsim::simulate(&mut self.mw, 1);

        thread::sleep(time::Duration::from_millis(20));

        true
    }
}
