use super::Skeleton;
use crate::render::FigureBoneData;

#[derive(Clone)]
pub struct FixtureSkeleton;

pub struct SkeletonAttr;

impl FixtureSkeleton {
    pub fn new() -> Self {
        Self {}
    }
}

impl Skeleton for FixtureSkeleton {
    type Attr = SkeletonAttr;
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

    fn interpolate(&mut self, _target: &Self, _dt: f32) {}
}
