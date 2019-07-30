use specs::{Component, NullStorage, VecStorage};
use vek::*;

// Position
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pos(pub Vec3<f32>);

impl Component for Pos {
    type Storage = VecStorage<Self>;
}

// Velocity
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vel(pub Vec3<f32>);

impl Component for Vel {
    type Storage = VecStorage<Self>;
}

// Orientation
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ori(pub Vec3<f32>);

impl Component for Ori {
    type Storage = VecStorage<Self>;
}

// ForceUpdate
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ForceUpdate;

impl Component for ForceUpdate {
    type Storage = NullStorage<Self>;
}
