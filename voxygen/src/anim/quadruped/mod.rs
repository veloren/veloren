
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
    head: Bone,
    chest: Bone,
    lf_leg: Bone,
    rf_leg: Bone,
    lb_leg: Bone,
    rb_leg: Bone,

}

impl QuadrupedSkeleton {
    pub fn new() -> Self {
        Self {
            head: Bone::default(),
            chest: Bone::default(),
            lf_leg: Bone::default(),
            rf_leg: Bone::default(),
            lb_leg: Bone::default(),
            rb_leg: Bone::default(),
        }
    }
}

impl Skeleton for QuadrupedSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        [
            FigureBoneData::new(self.head.compute_base_matrix()),
            FigureBoneData::new(self.chest.compute_base_matrix()),
            FigureBoneData::new(self.lf_leg.compute_base_matrix()),
            FigureBoneData::new(self.rf_leg.compute_base_matrix()),
            FigureBoneData::new(self.lb_leg.compute_base_matrix()),
            FigureBoneData::new(self.rb_leg.compute_base_matrix()),
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
        self.head.interpolate(&target.head);
        self.chest.interpolate(&target.chest);
        self.lf_leg.interpolate(&target.lf_leg);
        self.rf_leg.interpolate(&target.rf_leg);
        self.lb_leg.interpolate(&target.lb_leg);
        self.rb_leg.interpolate(&target.rb_leg);
    }
}
