use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;
use vek::*;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LightEmitter {
    pub col: Rgb<f32>,
    pub strength: f32,
    pub flicker: f32,
    pub animated: bool,
}

impl Default for LightEmitter {
    fn default() -> Self {
        Self {
            col: Rgb::one(),
            strength: 1.0,
            flicker: 0.0,
            animated: false,
        }
    }
}

impl Component for LightEmitter {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LightAnimation {
    pub offset: Vec3<f32>,
    pub col: Rgb<f32>,
    pub strength: f32,
}

impl Default for LightAnimation {
    fn default() -> Self {
        Self {
            offset: Vec3::zero(),
            col: Rgb::zero(),
            strength: 0.0,
        }
    }
}

impl Component for LightAnimation {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
