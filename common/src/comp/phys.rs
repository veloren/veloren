use specs::{Component, FlaggedStorage, NullStorage, VecStorage};
use vek::*;

// Position

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Pos(pub Vec3<f32>);

impl Component for Pos {
    type Storage = VecStorage<Self>;
}

// Velocity

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Vel(pub Vec3<f32>);

impl Component for Vel {
    type Storage = VecStorage<Self>;
}

// Direction

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Dir(pub Vec3<f32>);

impl Component for Dir {
    type Storage = VecStorage<Self>;
}

// Update

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct ForceUpdate;

impl Component for ForceUpdate {
    type Storage = NullStorage<Self>;
}
