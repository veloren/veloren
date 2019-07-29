use specs::{Component, FlaggedStorage, NullStorage};
use vek::*;
use specs_idvs::IDVStorage;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Respawning;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MoveDir(pub Vec2<f32>);

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Wielding {
    pub time: f32,
    pub applied: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Attacking {
    pub time: f32,
    pub applied: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rolling {
    pub time: f32,
    pub applied: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OnGround;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CanBuild;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Jumping;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Gliding;

impl Component for Respawning {
    type Storage = NullStorage<Self>;
}

impl Wielding {
    pub fn start() -> Self {
        Self {
            time: 0.0,
            applied: false,
        }
    }
}

impl Attacking {
    pub fn start() -> Self {
        Self {
            time: 0.0,
            applied: false,
        }
    }
}

impl Rolling {
    pub fn start() -> Self {
        Self {
            time: 0.0,
            applied: false,
        }
    }
}

impl Component for MoveDir {
    type Storage = IDVStorage<Self>;
}

impl Component for Wielding {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}

impl Component for Attacking {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}

impl Component for Rolling {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}

impl Component for OnGround {
    type Storage = NullStorage<Self>;
}

impl Component for CanBuild {
    type Storage = FlaggedStorage<Self, NullStorage<Self>>;
}

impl Component for Jumping {
    type Storage = NullStorage<Self>;
}

impl Component for Gliding {
    type Storage = FlaggedStorage<Self, NullStorage<Self>>;
}
