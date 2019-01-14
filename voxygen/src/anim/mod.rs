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

pub struct CharacterSkeleton {
    head: Bone,
    chest: Bone,
    belt: Bone,
    leggings: Bone,
    l_hand: Bone,
    r_hand: Bone,
    l_foot: Bone,
    r_foot: Bone,
    back: Bone,
}

impl CharacterSkeleton {
    pub fn new() -> Self {
        Self {
            head: Bone::default(),
            chest: Bone::default(),
            belt: Bone::default(),
            leggings: Bone::default(),
            l_hand: Bone::default(),
            r_hand: Bone::default(),
            l_foot: Bone::default(),
            r_foot: Bone::default(),
            back: Bone::default(),
        }
    }
}

impl Skeleton for CharacterSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        let chest_mat = self.chest.compute_base_matrix();

        [
            FigureBoneData::new(self.head.compute_base_matrix()),
            FigureBoneData::new(chest_mat),
            FigureBoneData::new(self.belt.compute_base_matrix()),
            FigureBoneData::new(self.leggings.compute_base_matrix()),
            FigureBoneData::new(self.l_hand.compute_base_matrix()),
            FigureBoneData::new(self.r_hand.compute_base_matrix()),
            FigureBoneData::new(self.l_foot.compute_base_matrix()),
            FigureBoneData::new(self.r_foot.compute_base_matrix()),
            FigureBoneData::new(chest_mat * self.back.compute_base_matrix()),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
        ]
    }
}

pub trait Animation {
    type Skeleton;
    type Dependency;

    fn update_skeleton(
        skeleton: &mut Self::Skeleton,
        dependency: Self::Dependency,
    );
}

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Skeleton = CharacterSkeleton;
    type Dependency = f64;

    fn update_skeleton(
        skeleton: &mut Self::Skeleton,
        time: f64,
    ) {
        let wave = (time as f32 * 10.0).sin();
        let wave_fast = (time as f32 * 5.0).sin();

        skeleton.head.offset = Vec3::unit_z() * 13.0;
        skeleton.head.ori = Quaternion::rotation_z(wave * 0.3);

        skeleton.chest.offset = Vec3::unit_z() * 9.0;
        skeleton.chest.ori = Quaternion::rotation_z(wave * 0.3);

        skeleton.belt.offset = Vec3::unit_z() * 7.0;
        skeleton.belt.ori = Quaternion::rotation_z(wave * 0.3);

        skeleton.leggings.offset = Vec3::unit_z() * 4.0;
        skeleton.leggings.ori = Quaternion::rotation_z(wave * 0.3);

        skeleton.l_hand.offset = Vec3::new(-8.0, wave * 4.0, 9.0);
        skeleton.r_hand.offset = Vec3::new(8.0, -wave * 4.0, 9.0);

        skeleton.l_foot.offset = Vec3::new(-3.0, -wave * 4.0, -(wave_fast.abs() - 0.5) * 3.0);
        skeleton.r_foot.offset = Vec3::new(3.0, wave * 4.0, (wave_fast.abs() - 0.5) * 3.0);

        skeleton.back.offset = Vec3::new(-8.0, 5.0, 16.0);
        skeleton.back.ori = Quaternion::rotation_y(2.5);
    }
}
