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
    head: Bone,
    torso: Bone,
    tail: Bone,
    wing_l: Bone,
    wing_r: Bone,
    leg_l: Bone,
    leg_r: Bone,
}

impl BirdMediumSkeleton {
    pub fn new() -> Self {
        Self {
            head: Bone::default(),
            torso: Bone::default(),
            tail: Bone::default(),
            wing_l: Bone::default(),
            wing_r: Bone::default(),
            leg_l: Bone::default(),
            leg_r: Bone::default(),
        }
    }
}

impl Skeleton for BirdMediumSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        let torso_mat = self.torso.compute_base_matrix();

        [
            FigureBoneData::new(self.head.compute_base_matrix() * torso_mat),
            FigureBoneData::new(torso_mat),
            FigureBoneData::new(self.tail.compute_base_matrix() * torso_mat),
            FigureBoneData::new(self.wing_l.compute_base_matrix() * torso_mat),
            FigureBoneData::new(self.wing_r.compute_base_matrix() * torso_mat),
            FigureBoneData::new(self.leg_l.compute_base_matrix()),
            FigureBoneData::new(self.leg_r.compute_base_matrix()),
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
        self.torso.interpolate(&target.torso, dt);
        self.tail.interpolate(&target.tail, dt);
        self.wing_l.interpolate(&target.wing_l, dt);
        self.wing_r.interpolate(&target.wing_r, dt);
        self.leg_l.interpolate(&target.leg_l, dt);
        self.leg_r.interpolate(&target.leg_r, dt);
    }
}
