use specs::{Component, NullStorage, VecStorage};
use vek::*;

pub type Position = Vec3<f32>;
pub type Velocity = Vec3<f32>;
pub type Acceleration = Vec3<f32>;

// Position
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Pos(pub Position);

impl Component for Pos {
    type Storage = VecStorage<Self>;
}

// Velocity (with Acceleration)
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Vel {
    pub linear: Velocity,
    pub accel: Acceleration,
}

impl Component for Vel {
    type Storage = VecStorage<Self>;
}

// Orientation
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Ori(pub Vec3<f32>);

impl Component for Ori {
    type Storage = VecStorage<Self>;
}

// ForceUpdate
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct ForceUpdate;

impl Component for ForceUpdate {
    type Storage = NullStorage<Self>;
}
