pub mod humanoid;
pub mod object;
pub mod quadruped;
pub mod quadruped_medium;
pub mod bird_medium;
pub mod fish_medium;

use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Body {
    Humanoid(humanoid::Body),
    Quadruped(quadruped::Body),
    QuadrupedMedium(quadruped_medium::Body),
    BirdMedium(bird_medium::Body),
    FishMedium(fish_medium::Body),
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
