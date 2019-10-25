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
pub struct FishMediumSkeleton {
    marlin_head: Bone,
    marlin_torso: Bone,
    marlin_rear: Bone,
    marlin_tail: Bone,
    marlin_fin_l: Bone,
    marlin_fin_r: Bone,
}

impl FishMediumSkeleton {
    pub fn new() -> Self {
        Self {
            marlin_head: Bone::default(),
            marlin_torso: Bone::default(),
            marlin_rear: Bone::default(),
            marlin_tail: Bone::default(),
            marlin_fin_l: Bone::default(),
            marlin_fin_r: Bone::default(),
        }
    }
}

impl Skeleton for FishMediumSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        let torso_mat = self.marlin_torso.compute_base_matrix();
        let rear_mat = self.marlin_rear.compute_base_matrix();

        [
            FigureBoneData::new(self.marlin_head.compute_base_matrix() * torso_mat),
            FigureBoneData::new(torso_mat),
            FigureBoneData::new(rear_mat * torso_mat),
            FigureBoneData::new(self.marlin_tail.compute_base_matrix() * rear_mat),
            FigureBoneData::new(self.marlin_fin_l.compute_base_matrix() * rear_mat),
            FigureBoneData::new(self.marlin_fin_r.compute_base_matrix() * rear_mat),
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
        self.marlin_head.interpolate(&target.marlin_head, dt);
        self.marlin_torso.interpolate(&target.marlin_torso, dt);
        self.marlin_rear.interpolate(&target.marlin_rear, dt);
        self.marlin_tail.interpolate(&target.marlin_tail, dt);
        self.marlin_fin_l.interpolate(&target.marlin_fin_l, dt);
        self.marlin_fin_r.interpolate(&target.marlin_fin_r, dt);
    }
}
