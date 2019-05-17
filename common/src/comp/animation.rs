use specs::{Component, FlaggedStorage, VecStorage};
use vek::*;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Animation {
    Idle,
    Run,
    Jump,
    Gliding,
    Attack,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct AnimationInfo {
    pub animation: Animation,
    pub time: f64,
    pub changed: bool,
}

impl AnimationInfo {
    pub fn new() -> Self {
        Self {
            animation: Animation::Idle,
            time: 0.0,
            changed: true,
        }
    }
}

impl Component for AnimationInfo {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}
