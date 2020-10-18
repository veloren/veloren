use crate::{sync::Uid, Damages, Explosion, Knockback};
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    Damages(Damages),
    Knockback(Knockback),
    RewardEnergy(u32),
    Explode(Explosion),
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
    /// Whether projectile collides with entities in the same group as its
    /// owner
    pub ignore_group: bool,
}

impl Component for Projectile {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
