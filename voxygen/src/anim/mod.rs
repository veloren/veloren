// Library
use vek::*;

// Crate
use crate::render::FigureBoneData;

#[derive(Copy, Clone)]
pub struct Bone {
    parent: Option<u8>, // MUST be less than the current bone index
    pub offset: Vec3<f32>,
    pub ori: Quaternion<f32>,
}

impl Bone {
    pub fn default() -> Self {
        Self {
            parent: None,
            offset: Vec3::zero(),
            ori: Quaternion::identity(),
        }
    }

    pub fn compute_base_matrix(&self) -> Mat4<f32> {
        Mat4::<f32>::translation_3d(self.offset) * Mat4::from(self.ori)
    }
}

#[derive(Copy, Clone)]
pub struct Skeleton {
    bones: [Bone; 16],
}

impl Skeleton {
    pub fn default() -> Self {
        Self {
            bones: [Bone::default(); 16],
        }
    }

    pub fn with_bone(mut self, bone_idx: u8, bone: Bone) -> Self {
        self.bones[bone_idx as usize] = bone;
        self
    }

    pub fn bone(&self, bone_idx: u8) -> &Bone { &self.bones[bone_idx as usize] }
    pub fn bone_mut(&mut self, bone_idx: u8) -> &mut Bone { &mut self.bones[bone_idx as usize] }

    pub fn compute_matrices(&self) -> [FigureBoneData; 16] {
        let mut bone_data = [FigureBoneData::default(); 16];
        for i in 0..16 {
            bone_data[i] = FigureBoneData::new(
                self.bones[i].compute_base_matrix()
                // *
                //if let Some(parent_idx) = self.bones[i].parent {
                //    bone_data[parent_idx as usize]
                //} else {
                //    Mat4::identity()
                //}
            );
        }
        bone_data
    }
}
