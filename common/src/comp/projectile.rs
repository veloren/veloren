use crate::sync::Uid;
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    Damage(i32),
    Knockback(f32),
    RewardEnergy(u32),
    Explode(Explosion),
    Vanish,
    Stick,
    Possess,
}

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Projectile {
    // TODO: use SmallVec for these effects
    pub hit_solid: Vec<Effect>,
    pub hit_entity: Vec<Effect>,
    /// Time left until the projectile will despawn
    pub time_left: Duration,
    pub owner: Option<Uid>,
    /// Whether projectile collides with entities in the same group as its
    /// owner
    pub ignore_group: bool,
}

impl Component for Projectile {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
