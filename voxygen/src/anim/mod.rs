pub mod character;

// Library
use vek::*;

// Crate
use crate::render::FigureBoneData;

#[derive(Copy, Clone)]
pub struct Bone {
    parent_idx: Option<u8>, // MUST be less than the current bone index
    pub offset: Vec3<f32>,
    pub ori: Quaternion<f32>,
}

impl Bone {
    pub fn default() -> Self {
        Self {
            parent_idx: None,
            offset: Vec3::zero(),
            ori: Quaternion::identity(),
        }
    }

    pub fn get_parent_idx(&self) -> Option<u8> { self.parent_idx }

    pub fn set_parent_idx(&mut self, parent_idx: u8) {
        self.parent_idx = Some(parent_idx);
    }

    pub fn compute_base_matrix(&self) -> Mat4<f32> {
        Mat4::<f32>::translation_3d(self.offset) * Mat4::from(self.ori)
    }
}

pub trait Skeleton {
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
