use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;
use vek::*;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Controller {
    pub move_dir: Vec2<f32>,
    pub jump: bool,
    pub attack: bool,
    pub block: bool,
    pub roll: bool,
    pub glide: bool,
    pub respawn: bool,
}

impl Component for Controller {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
