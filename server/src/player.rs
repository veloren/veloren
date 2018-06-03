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
}
