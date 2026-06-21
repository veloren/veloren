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

const MAX_FEET: usize = 8;

/// Track footsteps over time by detecting flips in foot velocity.
#[derive(Default)]
pub struct Footsteps {
    // What were the last known foot positions?
    foot_state: [Option<[f32; 2]>; MAX_FEET],
    // Did a step just occur?
    stepped: [bool; MAX_FEET],
}
impl Component for Footsteps {
    type Storage = VecStorage<Self>;
}

impl Footsteps {
    pub fn update(&mut self, foot_z: &[f32]) {
        self.stepped = [false; MAX_FEET];
        for (i, foot_z) in foot_z.iter().take(MAX_FEET).enumerate() {
            match &mut self.foot_state[i] {
                Some([a, b]) => {
                    // Position second differential flipped - footstep detected!
                    if a > b && *b < *foot_z {
                        self.stepped[i] = true;
                    }
                    *a = *b;
                    *b = *foot_z;
                },
                foot_state @ None => *foot_state = Some([*foot_z; 2]),
            }
        }
    }

    pub fn is_stepping(&self, foot: usize) -> bool {
        self.stepped.get(foot).copied().unwrap_or(false)
    }

    pub fn is_any_stepping(&self) -> bool { self.stepped.iter().any(|x| *x) }
}
