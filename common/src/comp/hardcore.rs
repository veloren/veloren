use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage, VecStorage};

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hardcore;

impl Component for Hardcore {
    type Storage = DerefFlaggedStorage<Self, VecStorage<Self>>;
}
