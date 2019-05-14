
pub mod run;

// Reexports
pub use self::run::RunAnimation;
// Crate
use crate::render::FigureBoneData;

// Local
use super::{Bone, Skeleton};

const SCALE: f32 = 11.0;

#[derive(Clone)]
pub struct QuadrupedSkeleton {
    pighead: Bone,
    pigchest: Bone,
    piglf_leg: Bone,
    pigrf_leg: Bone,
    piglb_leg: Bone,
    pigrb_leg: Bone,

}

impl QuadrupedSkeleton {
    pub fn new() -> Self {
        Self {
            pighead: Bone::default(),
            pigchest: Bone::default(),
            piglf_leg: Bone::default(),
            pigrf_leg: Bone::default(),
            piglb_leg: Bone::default(),
            pigrb_leg: Bone::default(),
        }
    }
}

impl Skeleton for QuadrupedSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        [
            FigureBoneData::new(self.pighead.compute_base_matrix()),
            FigureBoneData::new(self.pigchest.compute_base_matrix()),
            FigureBoneData::new(self.piglf_leg.compute_base_matrix()),
            FigureBoneData::new(self.pigrf_leg.compute_base_matrix()),
            FigureBoneData::new(self.piglb_leg.compute_base_matrix()),
            FigureBoneData::new(self.pigrb_leg.compute_base_matrix()),
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

    fn interpolate(&mut self, target: &Self) {
        self.pighead.interpolate(&target.pighead);
        self.pigchest.interpolate(&target.pigchest);
        self.piglf_leg.interpolate(&target.piglf_leg);
        self.pigrf_leg.interpolate(&target.pigrf_leg);
        self.piglb_leg.interpolate(&target.piglb_leg);
        self.pigrb_leg.interpolate(&target.pigrb_leg);
    }
}
