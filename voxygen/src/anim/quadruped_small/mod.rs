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
    pig_head: Bone,
    pig_chest: Bone,
    pig_leg_lf: Bone,
    pig_leg_rf: Bone,
    pig_leg_lb: Bone,
    pig_leg_rb: Bone,
}

impl QuadrupedSmallSkeleton {
    pub fn new() -> Self {
        Self {
            pig_head: Bone::default(),
            pig_chest: Bone::default(),
            pig_leg_lf: Bone::default(),
            pig_leg_rf: Bone::default(),
            pig_leg_lb: Bone::default(),
            pig_leg_rb: Bone::default(),
        }
    }
}

impl Skeleton for QuadrupedSmallSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        [
            FigureBoneData::new(self.pig_head.compute_base_matrix()),
            FigureBoneData::new(self.pig_chest.compute_base_matrix()),
            FigureBoneData::new(self.pig_leg_lf.compute_base_matrix()),
            FigureBoneData::new(self.pig_leg_rf.compute_base_matrix()),
            FigureBoneData::new(self.pig_leg_lb.compute_base_matrix()),
            FigureBoneData::new(self.pig_leg_rb.compute_base_matrix()),
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
        self.pig_head.interpolate(&target.pig_head, dt);
        self.pig_chest.interpolate(&target.pig_chest, dt);
        self.pig_leg_lf.interpolate(&target.pig_leg_lf, dt);
        self.pig_leg_rf.interpolate(&target.pig_leg_rf, dt);
        self.pig_leg_lb.interpolate(&target.pig_leg_lb, dt);
        self.pig_leg_rb.interpolate(&target.pig_leg_rb, dt);
    }
}
