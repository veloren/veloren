use specs::{Component, FlaggedStorage, VecStorage};
use vek::*;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Animation {
    Idle,
    Run,
    Jump,
    Gliding,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct ActionState {
    pub animation: Animation,
    pub changed: bool,
    pub time: f64,
    pub attack_started: bool,
}

impl ActionState {
    pub fn new() -> Self {
        Self {
            animation: Animation::Idle,
            changed: true,
            time: 0.0,
            attack_started: false,
        }
    }
}

impl Component for ActionState {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}
