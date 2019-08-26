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

// PhysicsState
#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicsState {
    pub on_ground: bool,
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
