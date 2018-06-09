#![feature(nll)]

extern crate noise;
#[macro_use]
extern crate euler;
extern crate region;

mod gen;
mod map;

use region::Chunk;

// Reexports
pub use map::Map as Map;
pub use map::Biome as Biome;

pub struct World {
    map: Map,
}

impl World {
    pub fn new(seed: u32, size: u32) -> World {
        World {
            map: Map::new(seed, size),
        }
    }

    pub fn tick(&mut self, secs: f64) {
        self.map.tick(secs);
    }

    pub fn map<'a>(&'a mut self) -> &'a mut Map {
        &mut self.map
    }
}
