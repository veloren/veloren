use crate::data::{Actors, Data, Nature};
use hashbrown::HashMap;
use world::World;

impl Data {
    pub fn generate(world: &World) -> Self {
        Self {
            nature: Nature {},
            actors: Actors {
                actors: HashMap::default(),
            },
        }
    }
}
