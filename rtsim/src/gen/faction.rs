use crate::data::Faction;
use rand::prelude::*;
use vek::*;
use world::{IndexRef, World};

impl Faction {
    pub fn generate(world: &World, index: IndexRef, rng: &mut impl Rng) -> Self {
        Self {
            leader: None,
            good_or_evil: rng.gen(),
        }
    }
}
