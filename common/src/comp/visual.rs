use serde::{Deserialize, Serialize};
use specs::{Component, DenseVecStorage, DerefFlaggedStorage};
use vek::*;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LightEmitter {
    pub col: Rgb<f32>,
    pub strength: f32,
    pub flicker: f32,
    pub animated: bool,
    // (direction, +cos(beam_angle))
    pub dir: Option<(Vec3<f32>, f32)>,
}

impl Component for LightEmitter {
    type Storage = DerefFlaggedStorage<Self, DenseVecStorage<Self>>;
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LightAnimation {
    pub offset: Vec3<f32>,
    pub col: Rgb<f32>,
    pub strength: f32,
    // (direction, +cos(beam_angle))
    pub dir: Option<(Vec3<f32>, f32)>,
}

impl Component for LightAnimation {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FrontendMarker {
    JoltArrow,
}

impl Component for FrontendMarker {
    type Storage = DerefFlaggedStorage<Self, specs::HashMapStorage<Self>>;
}
