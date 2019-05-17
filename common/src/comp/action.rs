use specs::{Component, FlaggedStorage, VecStorage};
use vek::*;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Action {
    Attack,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Actions(pub Vec<Action>);

impl Actions {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

impl Component for Actions {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}
