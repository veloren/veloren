use specs::{Component, FlaggedStorage, NullStorage, VecStorage};
use vek::*;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Control {
    pub move_dir: Vec2<f32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Respawning;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Attacking {
    pub time: f32,
    pub applied: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Cidling {
    pub time: f32,
    pub applied: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rolling {
    pub time: f32,
    pub applied: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Jumping;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Gliding;

impl Component for Control {
    type Storage = VecStorage<Self>;
}

impl Component for Respawning {
    type Storage = NullStorage<Self>;
}

impl Attacking {
    pub fn start() -> Self {
        Self {
            time: 0.0,
            applied: false,
        }
    }
}
impl Component for Attacking {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

impl Cidling {
    pub fn start() -> Self {
        Self {
            time: 0.0,
            applied: false,
        }
    }
}
impl Component for Cidling {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

impl Rolling {
    pub fn start() -> Self {
        Self {
            time: 0.0,
            applied: false,
        }
    }
}
impl Component for Rolling {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

impl Component for Jumping {
    type Storage = NullStorage<Self>;
}

impl Component for Gliding {
    type Storage = NullStorage<Self>;
}
