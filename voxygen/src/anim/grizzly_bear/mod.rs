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
pub struct GrizzlyBearSkeleton {
    grizzly_bear_upper_head: Bone,
    grizzly_bear_lower_head: Bone,
    grizzly_bear_upper_torso: Bone,
    grizzly_bear_lower_torso: Bone,
    grizzly_bear_ears: Bone,
    grizzly_bear_leg_lf: Bone,
    grizzly_bear_leg_rf: Bone,
    grizzly_bear_leg_lb: Bone,
    grizzly_bear_leg_rb: Bone,
    grizzly_bear_foot_lf: Bone,
    grizzly_bear_foot_rf: Bone,
    grizzly_bear_foot_lb: Bone,
    grizzly_bear_foot_rb: Bone,
}

impl GrizzlyBearSkeleton {
    pub fn new() -> Self {
        Self {
            grizzly_bear_upper_head: Bone::default(),
            grizzly_bear_lower_head: Bone::default(),
            grizzly_bear_upper_torso: Bone::default(),
            grizzly_bear_lower_torso: Bone::default(),
            grizzly_bear_ears: Bone::default(),
            grizzly_bear_leg_lf: Bone::default(),
            grizzly_bear_leg_rf: Bone::default(),
            grizzly_bear_leg_lb: Bone::default(),
            grizzly_bear_leg_rb: Bone::default(),
            grizzly_bear_foot_lf: Bone::default(),
            grizzly_bear_foot_rf: Bone::default(),
            grizzly_bear_foot_lb: Bone::default(),
            grizzly_bear_foot_rb: Bone::default(),
        }
    }
}

impl Skeleton for GrizzlyBearSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        [
            FigureBoneData::new(self.grizzly_bear_upper_head.compute_base_matrix()),
            FigureBoneData::new(self.grizzly_bear_lower_head.compute_base_matrix()),
            FigureBoneData::new(self.grizzly_bear_upper_torso.compute_base_matrix()),
            FigureBoneData::new(self.grizzly_bear_lower_torso.compute_base_matrix()),
            FigureBoneData::new(self.grizzly_bear_ears.compute_base_matrix()),
            FigureBoneData::new(self.grizzly_bear_leg_lf.compute_base_matrix()),
            FigureBoneData::new(self.grizzly_bear_leg_rf.compute_base_matrix()),
            FigureBoneData::new(self.grizzly_bear_leg_lb.compute_base_matrix()),
            FigureBoneData::new(self.grizzly_bear_leg_rb.compute_base_matrix()),
            FigureBoneData::new(self.grizzly_bear_foot_lf.compute_base_matrix()),
            FigureBoneData::new(self.grizzly_bear_foot_rf.compute_base_matrix()),
            FigureBoneData::new(self.grizzly_bear_foot_lb.compute_base_matrix()),
            FigureBoneData::new(self.grizzly_bear_foot_rb.compute_base_matrix()),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),

        ]
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.grizzly_bear_upper_head.interpolate(&target.grizzly_bear_upper_head, dt);
        self.grizzly_bear_lower_head.interpolate(&target.grizzly_bear_lower_head, dt);
        self.grizzly_bear_upper_torso.interpolate(&target.grizzly_bear_upper_torso, dt);
        self.grizzly_bear_lower_torso.interpolate(&target.grizzly_bear_lower_torso, dt);
        self.grizzly_bear_ears.interpolate(&target.grizzly_bear_ears, dt);
        self.grizzly_bear_leg_lf.interpolate(&target.grizzly_bear_leg_lf, dt);
        self.grizzly_bear_leg_rf.interpolate(&target.grizzly_bear_leg_rf, dt);
        self.grizzly_bear_leg_lb.interpolate(&target.grizzly_bear_leg_lb, dt);
        self.grizzly_bear_leg_rb.interpolate(&target.grizzly_bear_leg_rb, dt);
        self.grizzly_bear_foot_lf.interpolate(&target.grizzly_bear_foot_lf, dt);
        self.grizzly_bear_foot_rf.interpolate(&target.grizzly_bear_foot_rf, dt);
        self.grizzly_bear_foot_lb.interpolate(&target.grizzly_bear_foot_lb, dt);
        self.grizzly_bear_foot_rb.interpolate(&target.grizzly_bear_foot_rb, dt);
    }
}
