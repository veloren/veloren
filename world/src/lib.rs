#![feature(nll)]

extern crate noise;
#[macro_use]
extern crate euler;

mod gen;
mod map;

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

#[derive(Debug)]
pub struct Coordinate {
    x: f32,
    y: f32,
    z: f32
}

impl Coordinate {
    pub fn new(x: f32, y: f32, z: f32) -> Coordinate {
        Coordinate {
            x: x,
            y: y,
            z: z
        }
    }
    pub fn translate(&mut self, x: f32, y: f32, z: f32) {
        self.x += x;
        self.y += y;
        self.z += z;
    }
}
