use specs::{Component, FlaggedStorage, VecStorage};
use vek::*;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InputEvent {
    Jump,
    Attack,
    Respawn,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Inputs {
    // Held down
    pub move_dir: Vec2<f32>,
    pub jumping: bool,
    pub gliding: bool,

    // Event based
    pub events: Vec<InputEvent>,
}

impl Component for Inputs {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Actions {
    pub attack_time: Option<f32>,
}

impl Component for Actions {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}
