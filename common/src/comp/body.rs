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
#[repr(u32)]
pub enum Body {
    Humanoid(humanoid::Body) = 0,
    QuadrupedSmall(quadruped_small::Body) = 1,
    QuadrupedMedium(quadruped_medium::Body) = 2,
    BirdMedium(bird_medium::Body) = 3,
    FishMedium(fish_medium::Body) = 4,
    Dragon(dragon::Body) = 5,
    BirdSmall(bird_small::Body) = 6,
    FishSmall(fish_small::Body) = 7,
    BipedLarge(biped_large::Body) = 8,
    Object(object::Body) = 9,
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
