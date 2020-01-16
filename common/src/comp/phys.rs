use crate::sync::Uid;
use specs::{Component, FlaggedStorage, NullStorage};
use specs_idvs::IDVStorage;
use vek::*;

// Position
#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pos(pub Vec3<f32>);

impl Component for Pos {
    type Storage = IDVStorage<Self>;
}

// Velocity
#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vel(pub Vec3<f32>);

impl Component for Vel {
    type Storage = IDVStorage<Self>;
}

// Orientation
#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ori(pub Vec3<f32>);

impl Component for Ori {
    type Storage = IDVStorage<Self>;
}

// Scale
#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scale(pub f32);

impl Component for Scale {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}

// Mass
#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mass(pub f32);

impl Component for Mass {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}

#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gravity(pub f32);

impl Component for Gravity {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}

#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sticky;

impl Component for Sticky {
    type Storage = FlaggedStorage<Self, NullStorage<Self>>;
}

// PhysicsState
#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicsState {
    pub on_ground: bool,
    pub on_wall: Option<Vec3<f32>>,
    pub touch_entity: Option<Uid>,
    pub in_fluid: bool,
}

impl Component for PhysicsState {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}

// ForceUpdate
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ForceUpdate;

impl Component for ForceUpdate {
    type Storage = NullStorage<Self>;
}
