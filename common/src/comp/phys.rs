use specs::{Component, VecStorage, FlaggedStorage, NullStorage};
use vek::*;

// Pos

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Pos(pub Vec3<f32>);

impl Component for Pos {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

// Vel

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Vel(pub Vec3<f32>);

impl Component for Vel {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

// Dir

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Dir(pub Vec3<f32>);

impl Component for Dir {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

// Dir

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct ForceUpdate;

impl Component for ForceUpdate {
    type Storage = NullStorage<Self>;
}
