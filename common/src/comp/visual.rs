use specs::{Component, FlaggedStorage, VecStorage};
use vek::*;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LightEmitter {
    pub col: Rgb<f32>,
    pub strength: f32,
}

impl Default for LightEmitter {
    fn default() -> Self {
        Self {
            col: Rgb::one(),
            strength: 250.0,
        }
    }
}

impl Component for LightEmitter {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}
