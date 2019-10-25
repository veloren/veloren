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
    head_upper: Bone,
    jaw: Bone,
    head_lower: Bone,
    tail: Bone,
    torso_back: Bone,
    torso_mid: Bone,
    ears: Bone,
    foot_lf: Bone,
    foot_rf: Bone,
    foot_lb: Bone,
    foot_rb: Bone,
}

impl QuadrupedMediumSkeleton {
    pub fn new() -> Self {
        Self {
            head_upper: Bone::default(),
            jaw: Bone::default(),
            head_lower: Bone::default(),
            tail: Bone::default(),
            torso_back: Bone::default(),
            torso_mid: Bone::default(),
            ears: Bone::default(),
            foot_lf: Bone::default(),
            foot_rf: Bone::default(),
            foot_lb: Bone::default(),
            foot_rb: Bone::default(),
        }
    }
}

impl Skeleton for QuadrupedMediumSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        let ears_mat = self.ears.compute_base_matrix();
        let head_upper_mat = self.head_upper.compute_base_matrix();
        let head_lower_mat = self.head_lower.compute_base_matrix();

        [
            FigureBoneData::new(head_upper_mat),
            FigureBoneData::new(head_upper_mat * head_lower_mat * self.jaw.compute_base_matrix()),
            FigureBoneData::new(head_upper_mat * head_lower_mat),
            FigureBoneData::new(self.tail.compute_base_matrix()),
            FigureBoneData::new(self.torso_back.compute_base_matrix()),
            FigureBoneData::new(self.torso_mid.compute_base_matrix()),
            FigureBoneData::new(head_upper_mat * ears_mat),
            FigureBoneData::new(self.foot_lf.compute_base_matrix()),
            FigureBoneData::new(self.foot_rf.compute_base_matrix()),
            FigureBoneData::new(self.foot_lb.compute_base_matrix()),
            FigureBoneData::new(self.foot_rb.compute_base_matrix()),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
        ]
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.head_upper.interpolate(&target.head_upper, dt);
        self.jaw.interpolate(&target.jaw, dt);
        self.head_lower.interpolate(&target.head_lower, dt);
        self.tail.interpolate(&target.tail, dt);
        self.torso_back.interpolate(&target.torso_back, dt);
        self.torso_mid.interpolate(&target.torso_mid, dt);
        self.ears.interpolate(&target.ears, dt);
        self.foot_lf.interpolate(&target.foot_lf, dt);
        self.foot_rf.interpolate(&target.foot_rf, dt);
        self.foot_lb.interpolate(&target.foot_lb, dt);
        self.foot_rb.interpolate(&target.foot_rb, dt);
    }
}
