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
pub struct FishSmallSkeleton {
    cardinalfish_torso: Bone,
    cardinalfish_tail: Bone,

}

impl FishSmallSkeleton {
    pub fn new() -> Self {
        Self {
            cardinalfish_torso: Bone::default(),
            cardinalfish_tail: Bone::default(),
        }
    }
}

impl Skeleton for FishSmallSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        let torso_mat = self.cardinalfish_torso.compute_base_matrix();


        [
            FigureBoneData::new(torso_mat),
            FigureBoneData::new(self.cardinalfish_tail.compute_base_matrix() * torso_mat),
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
            FigureBoneData::default(),
            FigureBoneData::default(),
        ]
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.cardinalfish_torso
            .interpolate(&target.cardinalfish_torso, dt);
        self.cardinalfish_tail.interpolate(&target.cardinalfish_tail, dt);
    }
}
