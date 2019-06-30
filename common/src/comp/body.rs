pub mod humanoid;
pub mod quadruped;
pub mod quadruped_medium;

use specs::{Component, FlaggedStorage, VecStorage};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Body {
    Humanoid(humanoid::Body),
    Quadruped(quadruped::Body),
    QuadrupedMedium(quadruped_medium::Body),
}

impl Component for Body {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}
