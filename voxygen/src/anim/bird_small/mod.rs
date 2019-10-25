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
pub struct BirdSmallSkeleton {
    crow_head: Bone,
    crow_torso: Bone,
    crow_wing_l: Bone,
    crow_wing_r: Bone,
}

impl BirdSmallSkeleton {
    pub fn new() -> Self {
        Self {
            crow_head: Bone::default(),
            crow_torso: Bone::default(),
            crow_wing_l: Bone::default(),
            crow_wing_r: Bone::default(),
        }
    }
}

impl Skeleton for BirdSmallSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        let torso_mat = self.crow_torso.compute_base_matrix();

        [
            FigureBoneData::new(self.crow_head.compute_base_matrix() * torso_mat),
            FigureBoneData::new(torso_mat),
            FigureBoneData::new(self.crow_wing_l.compute_base_matrix() * torso_mat),
            FigureBoneData::new(self.crow_wing_r.compute_base_matrix() * torso_mat),
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
            FigureBoneData::default(),
            FigureBoneData::default(),
        ]
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.crow_head.interpolate(&target.crow_head, dt);
        self.crow_torso.interpolate(&target.crow_torso, dt);
        self.crow_wing_l.interpolate(&target.crow_wing_l, dt);
        self.crow_wing_r.interpolate(&target.crow_wing_r, dt);
    }
}
