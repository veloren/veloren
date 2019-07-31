use specs::{Component, NullStorage};
use specs_idvs::IDVStorage;
use vek::*;

// Position
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pos(pub Vec3<f32>);

impl Component for Pos {
    type Storage = IDVStorage<Self>;
}

// Velocity
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vel(pub Vec3<f32>);

impl Component for Vel {
    type Storage = IDVStorage<Self>;
}

// Orientation
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ori(pub Vec3<f32>);

impl Component for Ori {
    type Storage = IDVStorage<Self>;
}

// ForceUpdate
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ForceUpdate;

impl Component for ForceUpdate {
    type Storage = NullStorage<Self>;
}
