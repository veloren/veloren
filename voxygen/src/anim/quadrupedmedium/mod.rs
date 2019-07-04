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
pub struct QuadrupedMediumSkeleton {
    wolf_head_upper: Bone,
    wolf_jaw: Bone,
    wolf_head_lower: Bone,
    wolf_tail: Bone,
    wolf_torso_back: Bone,
    wolf_torso_mid: Bone,
    wolf_ears: Bone,
    wolf_foot_lf: Bone,
    wolf_foot_rf: Bone,
    wolf_foot_lb: Bone,
    wolf_foot_rb: Bone,
}

impl QuadrupedMediumSkeleton {
    pub fn new() -> Self {
        Self {
            wolf_head_upper: Bone::default(),
            wolf_jaw: Bone::default(),
            wolf_head_lower: Bone::default(),
            wolf_tail: Bone::default(),
            wolf_torso_back: Bone::default(),
            wolf_torso_mid: Bone::default(),
            wolf_ears: Bone::default(),
            wolf_foot_lf: Bone::default(),
            wolf_foot_rf: Bone::default(),
            wolf_foot_lb: Bone::default(),
            wolf_foot_rb: Bone::default(),
        }
    }
}

impl Skeleton for QuadrupedMediumSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        let ears_mat = self.wolf_ears.compute_base_matrix();
        let head_upper_mat = self.wolf_head_upper.compute_base_matrix();
        let head_lower_mat = self.wolf_head_lower.compute_base_matrix();

        [
            FigureBoneData::new(head_upper_mat),
            FigureBoneData::new(
                head_upper_mat * head_lower_mat * self.wolf_jaw.compute_base_matrix(),
            ),
            FigureBoneData::new(head_upper_mat * head_lower_mat),
            FigureBoneData::new(self.wolf_tail.compute_base_matrix()),
            FigureBoneData::new(self.wolf_torso_back.compute_base_matrix()),
            FigureBoneData::new(self.wolf_torso_mid.compute_base_matrix()),
            FigureBoneData::new(head_upper_mat * ears_mat),
            FigureBoneData::new(self.wolf_foot_lf.compute_base_matrix()),
            FigureBoneData::new(self.wolf_foot_rf.compute_base_matrix()),
            FigureBoneData::new(self.wolf_foot_lb.compute_base_matrix()),
            FigureBoneData::new(self.wolf_foot_rb.compute_base_matrix()),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
        ]
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.wolf_head_upper
            .interpolate(&target.wolf_head_upper, dt);
        self.wolf_jaw.interpolate(&target.wolf_jaw, dt);
        self.wolf_head_lower
            .interpolate(&target.wolf_head_lower, dt);
        self.wolf_tail.interpolate(&target.wolf_tail, dt);
        self.wolf_torso_back
            .interpolate(&target.wolf_torso_back, dt);
        self.wolf_torso_mid.interpolate(&target.wolf_torso_mid, dt);
        self.wolf_ears.interpolate(&target.wolf_ears, dt);
        self.wolf_foot_lf.interpolate(&target.wolf_foot_lf, dt);
        self.wolf_foot_rf.interpolate(&target.wolf_foot_rf, dt);
        self.wolf_foot_lb.interpolate(&target.wolf_foot_lb, dt);
        self.wolf_foot_rb.interpolate(&target.wolf_foot_rb, dt);
    }
}
