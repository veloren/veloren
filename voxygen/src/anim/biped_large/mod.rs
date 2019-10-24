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
    knight_head: Bone,
    knight_upper_torso: Bone,
    knight_lower_torso: Bone,
    knight_shoulder_l: Bone,
    knight_shoulder_r: Bone,
    knight_hand_l: Bone,
    knight_hand_r: Bone,
    knight_leg_l: Bone,
    knight_leg_r: Bone,
    knight_foot_l: Bone,
    knight_foot_r: Bone,



}

impl BipedLargeSkeleton {
    pub fn new() -> Self {
        Self {
            knight_head: Bone::default(),
            knight_upper_torso: Bone::default(),
            knight_lower_torso: Bone::default(),
            knight_shoulder_l: Bone::default(),
            knight_shoulder_r: Bone::default(),
            knight_hand_l: Bone::default(),
            knight_hand_r: Bone::default(),
            knight_leg_l: Bone::default(),
            knight_leg_r: Bone::default(),
            knight_foot_l: Bone::default(),
            knight_foot_r: Bone::default(),

        }
    }
}

impl Skeleton for BipedLargeSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        let upper_torso_mat = self.knight_upper_torso.compute_base_matrix();
        let shoulder_l_mat = self.knight_shoulder_l.compute_base_matrix();
        let shoulder_r_mat = self.knight_shoulder_r.compute_base_matrix();
        let leg_l_mat = self.knight_leg_l.compute_base_matrix();
        let leg_r_mat = self.knight_leg_r.compute_base_matrix();


        [
            FigureBoneData::new(self.knight_head.compute_base_matrix()),
            FigureBoneData::new(
                upper_torso_mat,
            ),
            FigureBoneData::new(self.knight_lower_torso.compute_base_matrix() * upper_torso_mat),
            FigureBoneData::new(shoulder_l_mat * upper_torso_mat),
            FigureBoneData::new(shoulder_r_mat * upper_torso_mat),
            FigureBoneData::new(self.knight_hand_l.compute_base_matrix() * shoulder_l_mat * upper_torso_mat),
            FigureBoneData::new(self.knight_hand_r.compute_base_matrix() * shoulder_r_mat *  upper_torso_mat),
            FigureBoneData::new(leg_l_mat),
            FigureBoneData::new(leg_r_mat),
            FigureBoneData::new(self.knight_foot_l.compute_base_matrix() * leg_l_mat),
            FigureBoneData::new(self.knight_foot_r.compute_base_matrix() * leg_r_mat),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
        ]
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.knight_head
            .interpolate(&target.knight_head, dt);
        self.knight_upper_torso.interpolate(&target.knight_upper_torso, dt);
        self.knight_lower_torso
            .interpolate(&target.knight_lower_torso, dt);
        self.knight_shoulder_l.interpolate(&target.knight_shoulder_l, dt);
        self.knight_shoulder_r
            .interpolate(&target.knight_shoulder_r, dt);
        self.knight_hand_l.interpolate(&target.knight_hand_l, dt);
        self.knight_hand_r.interpolate(&target.knight_hand_r, dt);
        self.knight_leg_l.interpolate(&target.knight_leg_l, dt);
        self.knight_leg_r.interpolate(&target.knight_leg_r, dt);
        self.knight_foot_l.interpolate(&target.knight_foot_l, dt);
        self.knight_foot_r.interpolate(&target.knight_foot_r, dt);


    }
}
