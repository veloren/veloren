// Crate
use crate::render::FigureBoneData;

// Local
use super::{Bone, Skeleton};

const SCALE: f32 = 44.0;

pub struct FixtureSkeleton;

impl FixtureSkeleton {
    pub fn new() -> Self {
        Self {}
    }
}

impl Skeleton for FixtureSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        [
            FigureBoneData::new(vek::Mat4::identity()),
            FigureBoneData::new(vek::Mat4::identity()),
            FigureBoneData::new(vek::Mat4::identity()),
            FigureBoneData::new(vek::Mat4::identity()),
            FigureBoneData::new(vek::Mat4::identity()),
            FigureBoneData::new(vek::Mat4::identity()),
            FigureBoneData::new(vek::Mat4::identity()),
            FigureBoneData::new(vek::Mat4::identity()),
            FigureBoneData::new(vek::Mat4::identity()),
            FigureBoneData::new(vek::Mat4::identity()),
            FigureBoneData::new(vek::Mat4::identity()),
            FigureBoneData::new(vek::Mat4::identity()),
            FigureBoneData::new(vek::Mat4::identity()),
            FigureBoneData::new(vek::Mat4::identity()),
            FigureBoneData::new(vek::Mat4::identity()),
            FigureBoneData::new(vek::Mat4::identity()),
        ]
    }

    fn interpolate(&mut self, target: &Self) {}
}
