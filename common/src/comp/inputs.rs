use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage, NullStorage};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CanBuild;
impl Component for CanBuild {
    type Storage = FlaggedStorage<Self, NullStorage<Self>>;
}
