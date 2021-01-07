use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage, NullStorage};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CanBuild;
impl Component for CanBuild {
    type Storage = DerefFlaggedStorage<Self, NullStorage<Self>>;
}
