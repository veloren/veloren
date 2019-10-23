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
pub struct BirdMediumSkeleton {
    duck_m_head: Bone,
    duck_m_torso: Bone,
    duck_m_tail: Bone,
    duck_m_wing_l: Bone,
    duck_m_wing_r: Bone,
    duck_m_leg_l: Bone,
    duck_m_leg_r: Bone,

}

impl BirdMediumSkeleton {
    pub fn new() -> Self {
        Self {
            duck_m_head: Bone::default(),
            duck_m_torso: Bone::default(),
            duck_m_tail: Bone::default(),
            duck_m_wing_l: Bone::default(),
            duck_m_wing_r: Bone::default(),
            duck_m_leg_l: Bone::default(),
            duck_m_leg_r: Bone::default(),


        }
    }
}

impl Skeleton for BirdMediumSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        let torso_mat = self.duck_m_torso.compute_base_matrix();


        [
            FigureBoneData::new(self.duck_m_head.compute_base_matrix() * torso_mat),
            FigureBoneData::new(
                torso_mat,
            ),
            FigureBoneData::new(self.duck_m_tail.compute_base_matrix() * torso_mat),
            FigureBoneData::new(self.duck_m_wing_l.compute_base_matrix() * torso_mat),
            FigureBoneData::new(self.duck_m_wing_r.compute_base_matrix() * torso_mat),
            FigureBoneData::new(self.duck_m_leg_l.compute_base_matrix()),
            FigureBoneData::new(self.duck_m_leg_r.compute_base_matrix()),
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
        self.duck_m_head
            .interpolate(&target.duck_m_head, dt);
        self.duck_m_torso.interpolate(&target.duck_m_torso, dt);
        self.duck_m_tail
            .interpolate(&target.duck_m_tail, dt);
        self.duck_m_wing_l.interpolate(&target.duck_m_wing_l, dt);
        self.duck_m_wing_r
            .interpolate(&target.duck_m_wing_r, dt);
        self.duck_m_leg_l.interpolate(&target.duck_m_leg_l, dt);
        self.duck_m_leg_r.interpolate(&target.duck_m_leg_r, dt);
    }
}
