use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Explosion {
    pub radius: f32,
    pub max_damage: u32,
    pub min_damage: u32,
    pub max_heal: u32,
    pub min_heal: u32,
    pub terrain_destruction_power: f32,
    pub energy_regen: u32,
}
