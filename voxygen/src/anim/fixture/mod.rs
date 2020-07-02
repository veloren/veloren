use super::Skeleton;
use crate::render::FigureBoneData;
use vek::{Mat4, Vec3};

#[derive(Clone)]
pub struct FixtureSkeleton;

pub struct SkeletonAttr;

impl FixtureSkeleton {
    pub fn new() -> Self { Self {} }
}

impl Skeleton for FixtureSkeleton {
    type Attr = SkeletonAttr;

    fn bone_count(&self) -> usize { 1 }

    fn compute_matrices<F: FnMut(Mat4<f32>) -> FigureBoneData>(
        &self,
        _make_bone: F,
    ) -> ([FigureBoneData; 16], Vec3<f32>) {
        (
            [
                FigureBoneData::default(), // <-- This is actually a bone!
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
                FigureBoneData::default(),
            ],
            Vec3::default(),
        )
    }

    fn interpolate(&mut self, _target: &Self, _dt: f32) {}
}
