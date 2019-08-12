use specs::{Component, FlaggedStorage, NullStorage};
use specs_idvs::IDVStorage;
use vek::*;

#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pos(pub Vec3<f32>);

#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vel(pub Vec3<f32>);

#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ori(pub Vec3<f32>);

#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scale(pub f32);

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ForceUpdate(pub bool);

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PhysicsState {
    pub on_ground: bool,
    // TODO: on_wall
}

impl Component for Pos {
    type Storage = IDVStorage<Self>;
}
impl Component for Vel {
    type Storage = IDVStorage<Self>;
}

impl Component for Ori {
    type Storage = IDVStorage<Self>;
}
impl Component for Scale {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
impl Component for PhysicsState {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
impl Component for ForceUpdate {
    type Storage = IDVStorage<Self>;
}
