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
    Explode { power: f32 },
    Vanish,
    Stick,
    Possess,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Projectile {
    // TODO: use SmallVec for these effects
    pub hit_solid: Vec<Effect>,
    pub hit_entity: Vec<Effect>,
    /// Time left until the projectile will despawn
    pub time_left: Duration,
    pub owner: Option<Uid>,
}

impl Component for Projectile {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
