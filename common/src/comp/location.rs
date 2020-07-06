use crate::state::Time;
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use vek::*;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Waypoint {
    pos: Vec3<f32>,
    last_save: Time,
}

impl Waypoint {
    pub fn new(pos: Vec3<f32>, last_save: Time) -> Self { Self { pos, last_save } }

    pub fn get_pos(&self) -> Vec3<f32> { self.pos }

    /// Time in seconds since this waypoint was saved
    pub fn elapsed(&self, time: Time) -> f64 { time.0 - self.last_save.0 }
}

impl Component for Waypoint {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct WaypointArea(f32);

impl WaypointArea {
    pub fn radius(&self) -> f32 { self.0 }
}

impl Component for WaypointArea {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

impl Default for WaypointArea {
    fn default() -> Self { Self(5.0) }
}
