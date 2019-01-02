// Library
use specs::{Component, VecStorage};
use vek::*;

// Pos

#[derive(Copy, Clone, Debug)]
pub struct Pos(pub Vec3<f32>);

impl Component for Pos {
    type Storage = VecStorage<Self>;
}

// Vel

#[derive(Copy, Clone, Debug)]
pub struct Vel(pub Vec3<f32>);

impl Component for Vel {
    type Storage = VecStorage<Self>;
}

// Dir

#[derive(Copy, Clone, Debug)]
pub struct Dir(pub Vec3<f32>);

impl Component for Dir {
    type Storage = VecStorage<Self>;
}
