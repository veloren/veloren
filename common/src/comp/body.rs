pub mod grizzly_bear;
pub mod humanoid;
pub mod object;
pub mod quadruped;
pub mod quadruped_medium;
pub mod stag;
pub mod wild_boar;

use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Body {
    Humanoid(humanoid::Body),
    Quadruped(quadruped::Body),
    QuadrupedMedium(quadruped_medium::Body),
    grizzly_bear(grizzly_bear::Body),
    wild_boar(wild_boar::Body),
    stag(stag::Body),
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
