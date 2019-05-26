pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::idle::IdleAnimation;
pub use self::jump::JumpAnimation;
pub use self::run::RunAnimation;

// Crate
use crate::render::FigureBoneData;

// Local
use super::{Bone, Skeleton};

const SCALE: f32 = 11.0;

#[derive(Clone)]
pub struct QuadrupedMediumSkeleton {
    wolf_upperhead: Bone,
    wolf_jaw: Bone,
    wolf_lowerhead: Bone,
    wolf_tail: Bone,
    wolf_torsoback: Bone,
    wolf_torsomid: Bone,
    wolf_ears: Bone,
    wolf_LFFoot: Bone,
    wolf_RFFoot: Bone,
    wolf_LBFoot: Bone,
    wolf_RBFoot: Bone,
}

impl QuadrupedMediumSkeleton {
    pub fn new() -> Self {
        Self {
            wolf_upperhead: Bone::default(),
            wolf_jaw: Bone::default(),
            wolf_lowerhead: Bone::default(),
            wolf_tail: Bone::default(),
            wolf_torsoback: Bone::default(),
            wolf_torsomid: Bone::default(),
            wolf_ears: Bone::default(),
            wolf_LFFoot: Bone::default(),
            wolf_RFFoot: Bone::default(),
            wolf_LBFoot: Bone::default(),
            wolf_RBFoot: Bone::default(),
        }
    }
}

impl Skeleton for QuadrupedMediumSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        let ears_mat = self.wolf_ears.compute_base_matrix();
        let upperhead_mat = self.wolf_upperhead.compute_base_matrix();
        let lowerhead_mat = self.wolf_lowerhead.compute_base_matrix();

        [
            FigureBoneData::new(upperhead_mat),
            FigureBoneData::new(
                upperhead_mat * lowerhead_mat * self.wolf_jaw.compute_base_matrix(),
            ),
            FigureBoneData::new(upperhead_mat * lowerhead_mat),
            FigureBoneData::new(self.wolf_tail.compute_base_matrix()),
            FigureBoneData::new(self.wolf_torsoback.compute_base_matrix()),
            FigureBoneData::new(self.wolf_torsomid.compute_base_matrix()),
            FigureBoneData::new(upperhead_mat * ears_mat),
            FigureBoneData::new(self.wolf_LFFoot.compute_base_matrix()),
            FigureBoneData::new(self.wolf_RFFoot.compute_base_matrix()),
            FigureBoneData::new(self.wolf_LBFoot.compute_base_matrix()),
            FigureBoneData::new(self.wolf_RBFoot.compute_base_matrix()),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
        ]
    }

    fn interpolate(&mut self, target: &Self) {
        self.wolf_upperhead.interpolate(&target.wolf_upperhead);
        self.wolf_jaw.interpolate(&target.wolf_jaw);
        self.wolf_lowerhead.interpolate(&target.wolf_lowerhead);
        self.wolf_tail.interpolate(&target.wolf_tail);
        self.wolf_torsoback.interpolate(&target.wolf_torsoback);
        self.wolf_torsomid.interpolate(&target.wolf_torsomid);
        self.wolf_ears.interpolate(&target.wolf_ears);
        self.wolf_LFFoot.interpolate(&target.wolf_LFFoot);
        self.wolf_RFFoot.interpolate(&target.wolf_RFFoot);
        self.wolf_LBFoot.interpolate(&target.wolf_LBFoot);
        self.wolf_RBFoot.interpolate(&target.wolf_RBFoot);
    }
}
