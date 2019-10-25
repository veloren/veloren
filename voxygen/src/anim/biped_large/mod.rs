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
pub struct BipedLargeSkeleton {
    head: Bone,
    upper_torso: Bone,
    lower_torso: Bone,
    shoulder_l: Bone,
    shoulder_r: Bone,
    hand_l: Bone,
    hand_r: Bone,
    leg_l: Bone,
    leg_r: Bone,
    foot_l: Bone,
    foot_r: Bone,
}

impl BipedLargeSkeleton {
    pub fn new() -> Self {
        Self {
            head: Bone::default(),
            upper_torso: Bone::default(),
            lower_torso: Bone::default(),
            shoulder_l: Bone::default(),
            shoulder_r: Bone::default(),
            hand_l: Bone::default(),
            hand_r: Bone::default(),
            leg_l: Bone::default(),
            leg_r: Bone::default(),
            foot_l: Bone::default(),
            foot_r: Bone::default(),
        }
    }
}

impl Skeleton for BipedLargeSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        let upper_torso_mat = self.upper_torso.compute_base_matrix();
        let shoulder_l_mat = self.shoulder_l.compute_base_matrix();
        let shoulder_r_mat = self.shoulder_r.compute_base_matrix();
        let leg_l_mat = self.leg_l.compute_base_matrix();
        let leg_r_mat = self.leg_r.compute_base_matrix();

        [
            FigureBoneData::new(self.head.compute_base_matrix()),
            FigureBoneData::new(upper_torso_mat),
            FigureBoneData::new(self.lower_torso.compute_base_matrix() * upper_torso_mat),
            FigureBoneData::new(shoulder_l_mat * upper_torso_mat),
            FigureBoneData::new(shoulder_r_mat * upper_torso_mat),
            FigureBoneData::new(
                self.hand_l.compute_base_matrix() * shoulder_l_mat * upper_torso_mat,
            ),
            FigureBoneData::new(
                self.hand_r.compute_base_matrix() * shoulder_r_mat * upper_torso_mat,
            ),
            FigureBoneData::new(leg_l_mat),
            FigureBoneData::new(leg_r_mat),
            FigureBoneData::new(self.foot_l.compute_base_matrix() * leg_l_mat),
            FigureBoneData::new(self.foot_r.compute_base_matrix() * leg_r_mat),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
        ]
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.head.interpolate(&target.head, dt);
        self.upper_torso.interpolate(&target.upper_torso, dt);
        self.lower_torso.interpolate(&target.lower_torso, dt);
        self.shoulder_l.interpolate(&target.shoulder_l, dt);
        self.shoulder_r.interpolate(&target.shoulder_r, dt);
        self.hand_l.interpolate(&target.hand_l, dt);
        self.hand_r.interpolate(&target.hand_r, dt);
        self.leg_l.interpolate(&target.leg_l, dt);
        self.leg_r.interpolate(&target.leg_r, dt);
        self.foot_l.interpolate(&target.foot_l, dt);
        self.foot_r.interpolate(&target.foot_r, dt);
    }
}
