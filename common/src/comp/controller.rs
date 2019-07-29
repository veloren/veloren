use specs::{Component, FlaggedStorage};
use vek::*;
use specs_idvs::IDVStorage;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Controller {
    pub move_dir: Vec2<f32>,
    pub jump: bool,
    pub attack: bool,
    pub roll: bool,
    pub glide: bool,
    pub respawn: bool,
}

impl Component for Controller {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
