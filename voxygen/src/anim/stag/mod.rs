pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::idle::IdleAnimation;
pub use self::jump::JumpAnimation;
pub use self::run::RunAnimation;

use super::{Bone, Skeleton};
use crate::render::FigureBoneData;

#[derive(Clone)]
pub struct StagSkeleton {
    stag_head: Bone,
    stag_torso: Bone,
    stag_neck: Bone,
    stag_leg_lf: Bone,
    stag_leg_rf: Bone,
    stag_leg_lb: Bone,
    stag_leg_rb: Bone,
    stag_foot_lf: Bone,
    stag_foot_rf: Bone,
    stag_foot_lb: Bone,
    stag_foot_rb: Bone,
}

impl StagSkeleton {
    pub fn new() -> Self {
        Self {
            stag_head: Bone::default(),
            stag_torso: Bone::default(),
            stag_neck: Bone::default(),
            stag_leg_lf: Bone::default(),
            stag_leg_rf: Bone::default(),
            stag_leg_lb: Bone::default(),
            stag_leg_rb: Bone::default(),
            stag_foot_lf: Bone::default(),
            stag_foot_rf: Bone::default(),
            stag_foot_lb: Bone::default(),
            stag_foot_rb: Bone::default(),
        }
    }
}

impl Skeleton for StagSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        [
            FigureBoneData::new(self.stag_head.compute_base_matrix()),
            FigureBoneData::new(self.stag_torso.compute_base_matrix()),
            FigureBoneData::new(self.stag_neck.compute_base_matrix()),
            FigureBoneData::new(self.stag_leg_lf.compute_base_matrix()),
            FigureBoneData::new(self.stag_leg_rf.compute_base_matrix()),
            FigureBoneData::new(self.stag_leg_lb.compute_base_matrix()),
            FigureBoneData::new(self.stag_leg_rb.compute_base_matrix()),
            FigureBoneData::new(self.stag_foot_lf.compute_base_matrix()),
            FigureBoneData::new(self.stag_foot_rf.compute_base_matrix()),
            FigureBoneData::new(self.stag_foot_lb.compute_base_matrix()),
            FigureBoneData::new(self.stag_foot_rb.compute_base_matrix()),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
        ]
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.stag_head.interpolate(&target.stag_head, dt);
        self.stag_torso.interpolate(&target.stag_torso, dt);
        self.stag_neck.interpolate(&target.stag_neck, dt);
        self.stag_leg_lf.interpolate(&target.stag_leg_lf, dt);
        self.stag_leg_rf.interpolate(&target.stag_leg_rf, dt);
        self.stag_leg_lb.interpolate(&target.stag_leg_lb, dt);
        self.stag_leg_rb.interpolate(&target.stag_leg_rb, dt);
        self.stag_foot_lf.interpolate(&target.stag_foot_lf, dt);
        self.stag_foot_rf.interpolate(&target.stag_foot_rf, dt);
        self.stag_foot_lb.interpolate(&target.stag_foot_lb, dt);
        self.stag_foot_rb.interpolate(&target.stag_foot_rb, dt);
    }
}
