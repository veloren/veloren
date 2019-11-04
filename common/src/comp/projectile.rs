use crate::comp;
use crate::state::Uid;
use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Effect {
    Damage(comp::HealthChange),
    Vanish,
    Stick,
    Possess,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Projectile {
    pub owner: Uid,
    // TODO: use SmallVec for these effects
    pub hit_ground: Vec<Effect>,
    pub hit_wall: Vec<Effect>,
    pub hit_entity: Vec<Effect>,
    /// Time left until the projectile will despawn
    pub time_left: Duration,
}

impl Component for Projectile {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
