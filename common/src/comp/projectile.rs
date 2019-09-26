use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Effect {
    Damage(u32),
    Vanish,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Projectile {
    pub hit_ground: Vec<Effect>,
    pub hit_entity: Vec<Effect>,
}

impl Component for Projectile {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
