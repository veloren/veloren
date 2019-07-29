pub mod humanoid;
pub mod object;
pub mod quadruped;
pub mod quadruped_medium;

use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Body {
    Humanoid(humanoid::Body),
    Quadruped(quadruped::Body),
    QuadrupedMedium(quadruped_medium::Body),
    Object(object::Body),
}

impl Component for Body {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
