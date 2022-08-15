use crate::data::Faction;
use vek::*;
use rand::prelude::*;
use world::{
    World,
    IndexRef,
};

impl Faction {
    pub fn generate(world: &World, index: IndexRef, rng: &mut impl Rng) -> Self {
        Self { leader: None }
    }
}
