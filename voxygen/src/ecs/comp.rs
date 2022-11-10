use common::{comp::Ori, outcome::HealthChangeInfo};
use specs::{Component, VecStorage};
use vek::*;

// Floats over entity that has had a health change, rising up over time until it
// vanishes
#[derive(Copy, Clone, Debug)]
pub struct HpFloater {
    pub timer: f32,
    // Used for the "jumping" animation of the HpFloater whenever it changes it's value
    pub jump_timer: f32,
    pub info: HealthChangeInfo,
    // Used for randomly offsetting
    pub rand: f32,
}
#[derive(Clone, Debug, Default)]
pub struct HpFloaterList {
    // Order oldest to newest
    pub floaters: Vec<HpFloater>,

    // The time since you last damaged this entity
    // Used to display nametags outside normal range if this time is below a certain value
    pub time_since_last_dmg_by_me: Option<f32>,
}
impl Component for HpFloaterList {
    type Storage = VecStorage<Self>;
}

// Used for smooth interpolation of visual elements that are tied to entity
// position
#[derive(Copy, Clone, Debug)]
pub struct Interpolated {
    pub pos: Vec3<f32>,
    pub ori: Ori,
}
impl Component for Interpolated {
    type Storage = VecStorage<Self>;
}
