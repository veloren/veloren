pub mod biped_large;
pub mod bird_medium;
pub mod bird_small;
pub mod dragon;
pub mod fish_medium;
pub mod fish_small;
pub mod humanoid;
pub mod object;
pub mod quadruped_medium;
pub mod quadruped_small;

use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Body {
    Humanoid(humanoid::Body),
    QuadrupedSmall(quadruped_small::Body),
    QuadrupedMedium(quadruped_medium::Body),
    BirdMedium(bird_medium::Body),
    FishMedium(fish_medium::Body),
    Dragon(dragon::Body),
    BirdSmall(bird_small::Body),
    FishSmall(fish_small::Body),
    BipedLarge(biped_large::Body),
    Object(object::Body),
}

impl Body {
    pub fn is_humanoid(&self) -> bool {
        match self {
            Body::Humanoid(_) => true,
            _ => false,
        }
    }
}

impl Component for Body {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
