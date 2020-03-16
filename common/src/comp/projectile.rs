use crate::{comp, sync::Uid};
use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    Damage(comp::HealthChange),
    Vanish,
    Stick,
    Possess,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Projectile {
    // TODO: use SmallVec for these effects
    pub hit_ground: Vec<Effect>,
    pub hit_wall: Vec<Effect>,
    pub hit_entity: Vec<Effect>,
    /// Time left until the projectile will despawn
    pub time_left: Duration,
    pub owner: Option<Uid>,
}

impl Component for Projectile {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
