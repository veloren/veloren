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
    // Note: this might need to be refined to something more complex for realistic
    // behavior with less cylindrical bodies (e.g. wolfs)
    pub fn radius(&self) -> f32 {
        // TODO: Improve these values (some might be reliant on more info in inner type)
        match self {
            Body::Humanoid(_) => 0.5,
            Body::QuadrupedSmall(_) => 0.6,
            Body::QuadrupedMedium(_) => 0.9,
            Body::BirdMedium(_) => 0.5,
            Body::FishMedium(_) => 0.5,
            Body::Dragon(_) => 2.5,
            Body::BirdSmall(_) => 0.2,
            Body::FishSmall(_) => 0.2,
            Body::BipedLarge(_) => 1.0,
            Body::Object(_) => 0.3,
        }
    }
}

impl Component for Body {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
