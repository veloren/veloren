use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage, NullStorage};

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hardcore;

impl Component for Hardcore {
    type Storage = DerefFlaggedStorage<Self, NullStorage<Self>>;
}
