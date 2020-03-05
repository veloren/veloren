use super::Skeleton;
use crate::render::FigureBoneData;
use vek::*;

#[derive(Clone)]
pub struct ObjectSkeleton;
pub struct SkeletonAttr;

impl ObjectSkeleton {
    pub fn new() -> Self { Self {} }
}

const SCALE: f32 = 1.0 / 11.0;

impl Skeleton for ObjectSkeleton {
    type Attr = SkeletonAttr;

    fn compute_matrices(&self) -> [FigureBoneData; 18] {
        [
            FigureBoneData::new(Mat4::scaling_3d(Vec3::broadcast(SCALE))),
            FigureBoneData::new(Mat4::scaling_3d(Vec3::broadcast(SCALE))),
            FigureBoneData::new(Mat4::scaling_3d(Vec3::broadcast(SCALE))),
            FigureBoneData::new(Mat4::scaling_3d(Vec3::broadcast(SCALE))),
            FigureBoneData::new(Mat4::scaling_3d(Vec3::broadcast(SCALE))),
            FigureBoneData::new(Mat4::scaling_3d(Vec3::broadcast(SCALE))),
            FigureBoneData::new(Mat4::scaling_3d(Vec3::broadcast(SCALE))),
            FigureBoneData::new(Mat4::scaling_3d(Vec3::broadcast(SCALE))),
            FigureBoneData::new(Mat4::scaling_3d(Vec3::broadcast(SCALE))),
            FigureBoneData::new(Mat4::scaling_3d(Vec3::broadcast(SCALE))),
            FigureBoneData::new(Mat4::scaling_3d(Vec3::broadcast(SCALE))),
            FigureBoneData::new(Mat4::scaling_3d(Vec3::broadcast(SCALE))),
            FigureBoneData::new(Mat4::scaling_3d(Vec3::broadcast(SCALE))),
            FigureBoneData::new(Mat4::scaling_3d(Vec3::broadcast(SCALE))),
            FigureBoneData::new(Mat4::scaling_3d(Vec3::broadcast(SCALE))),
            FigureBoneData::new(Mat4::scaling_3d(Vec3::broadcast(SCALE))),
            FigureBoneData::new(Mat4::scaling_3d(Vec3::broadcast(SCALE))),
            FigureBoneData::new(Mat4::scaling_3d(Vec3::broadcast(SCALE))),
        ]
    }

    fn interpolate(&mut self, _target: &Self, _dt: f32) {}
}
