use crate::{combat::Attack, comp::item::Reagent, effect::Effect};
use serde::{Deserialize, Serialize};
use vek::Rgb;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Explosion {
    pub effects: Vec<RadiusEffect>,
    pub radius: f32,
    pub reagent: Option<Reagent>,
    pub min_falloff: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RadiusEffect {
    TerrainDestruction(f32, Rgb<f32>),
    Entity(Effect),
    Attack(Attack),
}
