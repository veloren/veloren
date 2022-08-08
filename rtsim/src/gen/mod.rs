use crate::data::{Actors, Data, Nature};
use hashbrown::HashMap;
use world::World;

impl Data {
    pub fn generate(world: &World) -> Self {
        Self {
            nature: Nature::generate(world),
            actors: Actors {
                actors: HashMap::default(),
            },
        }
    }
}
