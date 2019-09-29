use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;
use vek::*;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Waypoint {
    pos: Vec3<f32>,
}

impl Waypoint {
    pub fn new(pos: Vec3<f32>) -> Self {
        Self { pos }
    }

    pub fn get_pos(&self) -> Vec3<f32> {
        self.pos
    }
}

impl Component for Waypoint {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
