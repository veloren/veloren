extern crate world;

use ClientMode;
use world::Coordinate;

pub struct Player {
    mode: ClientMode,
    position: Coordinate,
    alias: String,
}

impl Player {
    pub fn new(mode: ClientMode, alias: &str, position: Coordinate) -> Player {
        Player {
            mode,
            alias: alias.to_string(),
            position,
        }
    }

    pub fn alias<'a>(&'a self) -> &str {
        &self.alias
    }

    pub fn position<'a>(&'a self) -> &Coordinate {
        &self.position
    }

    pub fn move_by(&mut self, dx: f32, dy: f32, dz: f32) {
        self.position.translate(dx, dy, dz);
    }
}
