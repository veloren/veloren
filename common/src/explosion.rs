use crate::{combat::Attack, effect::Effect, comp::item::Reagent};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Explosion {
    pub effects: Vec<RadiusEffect>,
    pub radius: f32,
    pub reagent: Option<Reagent>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RadiusEffect {
    TerrainDestruction(f32),
    Entity(Effect),
    Attack(Attack),
}
