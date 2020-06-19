use super::{FigureBoneData, Skeleton};
use vek::Vec3;

#[derive(Clone)]
pub struct FixtureSkeleton;

pub struct SkeletonAttr;

impl FixtureSkeleton {
    #[allow(clippy::new_without_default)] // TODO: Pending review in #587
    pub fn new() -> Self { Self {} }
}

impl Skeleton for FixtureSkeleton {
    type Attr = SkeletonAttr;

    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"fixture_compute_mats\0";

    fn bone_count(&self) -> usize { 1 }

    #[cfg_attr(feature = "be-dyn-lib", export_name = "fixture_compute_mats")]

    fn compute_matrices_inner(&self) -> ([FigureBoneData; 16], Vec3<f32>) {
        (
            [
                FigureBoneData::new(vek::Mat4::identity()), // <-- This is actually a bone!
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
            ],
            Vec3::default(),
        )
    }

    fn interpolate(&mut self, _target: &Self, _dt: f32) {}
}
