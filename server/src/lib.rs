extern crate worldgen;
extern crate worldsim;

use std::{thread, time};
use worldgen::MacroWorld;

pub struct Server {
    mw: MacroWorld,
}

impl Server {
    pub fn new(seed: u32, world_size: u32) -> Server {
        Server {
            mw: MacroWorld::new(seed, world_size),
        }
    }

    pub fn next_tick(&mut self) -> bool {
        worldsim::simulate(&mut self.mw, 1);

        thread::sleep(time::Duration::from_millis(20));

        true
    }
}
