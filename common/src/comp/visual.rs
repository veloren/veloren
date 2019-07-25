use specs::{Component, FlaggedStorage, VecStorage};
use vek::*;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LightEmitter {
    pub offset: Vec3<f32>,
    pub col: Rgb<f32>,
    pub strength: f32,
}

impl Default for LightEmitter {
    fn default() -> Self {
        Self {
            offset: Vec3::zero(),
            col: Rgb::one(),
            strength: 1.0,
        }
    }
}

impl Component for LightEmitter {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}
