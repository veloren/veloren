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
pub struct QuadrupedSmallSkeleton {
    head: Bone,
    chest: Bone,
    leg_lf: Bone,
    leg_rf: Bone,
    leg_lb: Bone,
    leg_rb: Bone,
}

impl QuadrupedSmallSkeleton {
    pub fn new() -> Self {
        Self {
            head: Bone::default(),
            chest: Bone::default(),
            leg_lf: Bone::default(),
            leg_rf: Bone::default(),
            leg_lb: Bone::default(),
            leg_rb: Bone::default(),
        }
    }
}

impl Skeleton for QuadrupedSmallSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        [
            FigureBoneData::new(self.head.compute_base_matrix()),
            FigureBoneData::new(self.chest.compute_base_matrix()),
            FigureBoneData::new(self.leg_lf.compute_base_matrix()),
            FigureBoneData::new(self.leg_rf.compute_base_matrix()),
            FigureBoneData::new(self.leg_lb.compute_base_matrix()),
            FigureBoneData::new(self.leg_rb.compute_base_matrix()),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
        ]
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.head.interpolate(&target.head, dt);
        self.chest.interpolate(&target.chest, dt);
        self.leg_lf.interpolate(&target.leg_lf, dt);
        self.leg_rf.interpolate(&target.leg_rf, dt);
        self.leg_lb.interpolate(&target.leg_lb, dt);
        self.leg_rb.interpolate(&target.leg_rb, dt);
    }
}
