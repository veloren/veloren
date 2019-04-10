pub mod character;

// Library
use vek::*;

// Crate
use crate::render::FigureBoneData;

#[derive(Copy, Clone)]
pub struct Bone {
    pub offset: Vec3<f32>,
    pub ori: Quaternion<f32>,
}

impl Bone {
    pub fn default() -> Self {
        Self {
            offset: Vec3::zero(),
            ori: Quaternion::identity(),
        }
    }

    pub fn compute_base_matrix(&self) -> Mat4<f32> {
        Mat4::<f32>::translation_3d(self.offset) * Mat4::from(self.ori)
    }
}

pub trait Skeleton: Send + Sync + 'static {
    fn compute_matrices(&self) -> [FigureBoneData; 16];
}

pub trait Animation {
    type Skeleton;
    type Dependency;

    fn update_skeleton(
        skeleton: &mut Self::Skeleton,
        dependency: Self::Dependency,
    );
}
