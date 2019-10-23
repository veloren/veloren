pub mod character;
pub mod fixture;
pub mod object;
pub mod quadruped;
pub mod quadrupedmedium;
pub mod birdmedium;
pub mod fishmedium;

use crate::render::FigureBoneData;
use common::comp::{self, item::Tool};
use vek::*;

#[derive(Copy, Clone)]
pub struct Bone {
    pub offset: Vec3<f32>,
    pub ori: Quaternion<f32>,
    pub scale: Vec3<f32>,
}

impl Bone {
    pub fn default() -> Self {
        Self {
            offset: Vec3::zero(),
            ori: Quaternion::identity(),
            scale: Vec3::broadcast(1.0 / 11.0),
        }
    }

    pub fn compute_base_matrix(&self) -> Mat4<f32> {
        Mat4::<f32>::translation_3d(self.offset)
            * Mat4::scaling_3d(self.scale)
            * Mat4::from(self.ori)
    }

    /// Change the current bone to be more like `target`.
    fn interpolate(&mut self, target: &Bone, dt: f32) {
        // TODO: Make configurable.
        let factor = (15.0 * dt).min(1.0);
        self.offset += (target.offset - self.offset) * factor;
        self.ori = vek::ops::Slerp::slerp(self.ori, target.ori, factor);
        self.scale += (target.scale - self.scale) * factor;
    }
}

pub trait Skeleton: Send + Sync + 'static {
    fn compute_matrices(&self) -> [FigureBoneData; 16];

    /// Change the current skeleton to be more like `target`.
    fn interpolate(&mut self, target: &Self, dt: f32);
}

pub struct SkeletonAttr {
    scaler: f32,
    head_scale: f32,
    neck_height: f32,
    neck_forward: f32,
    neck_right: f32,
    weapon_x: f32,
    weapon_y: f32,
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            scaler: 1.0,
            head_scale: 1.0,
            neck_height: 1.0,
            neck_forward: 1.0,
            neck_right: 1.0,
            weapon_x: 1.0,
            weapon_y: 1.0,
        }
    }
}

impl<'a> From<&'a comp::humanoid::Body> for SkeletonAttr {
    fn from(body: &'a comp::humanoid::Body) -> Self {
        use comp::humanoid::{BodyType::*, Race::*};
        Self {
            scaler: match (body.race, body.body_type) {
                (Orc, Male) => 0.95,
                (Orc, Female) => 0.8,
                (Human, Male) => 0.8,
                (Human, Female) => 0.75,
                (Elf, Male) => 0.85,
                (Elf, Female) => 0.8,
                (Dwarf, Male) => 0.7,
                (Dwarf, Female) => 0.65,
                (Undead, Male) => 0.8,
                (Undead, Female) => 0.75,
                (Danari, Male) => 0.58,
                (Danari, Female) => 0.58,
            },
            head_scale: match (body.race, body.body_type) {
                (Orc, Male) => 0.9,
                (Orc, Female) => 1.0,
                (Human, Male) => 1.0,
                (Human, Female) => 1.0,
                (Elf, Male) => 0.95,
                (Elf, Female) => 1.0,
                (Dwarf, Male) => 1.0,
                (Dwarf, Female) => 1.0,
                (Undead, Male) => 1.0,
                (Undead, Female) => 1.0,
                (Danari, Male) => 1.15,
                (Danari, Female) => 1.15,
            },
            neck_height: match (body.race, body.body_type) {
                (Orc, Male) => 0.0,
                (Orc, Female) => 0.0,
                (Human, Male) => 0.0,
                (Human, Female) => 0.0,
                (Elf, Male) => 0.0,
                (Elf, Female) => 0.0,
                (Dwarf, Male) => 0.0,
                (Dwarf, Female) => 0.0,
                (Undead, Male) => 0.5,
                (Undead, Female) => 0.5,
                (Danari, Male) => 0.5,
                (Danari, Female) => 0.5,
            },
            neck_forward: match (body.race, body.body_type) {
                (Orc, Male) => 0.0,
                (Orc, Female) => 0.0,
                (Human, Male) => 0.5,
                (Human, Female) => 0.0,
                (Elf, Male) => 0.5,
                (Elf, Female) => 0.5,
                (Dwarf, Male) => 0.5,
                (Dwarf, Female) => 0.0,
                (Undead, Male) => 0.5,
                (Undead, Female) => 0.5,
                (Danari, Male) => 0.0,
                (Danari, Female) => 0.0,
            },
            neck_right: match (body.race, body.body_type) {
                (Orc, Male) => 0.0,
                (Orc, Female) => 0.0,
                (Human, Male) => 0.0,
                (Human, Female) => 0.0,
                (Elf, Male) => 0.0,
                (Elf, Female) => 0.0,
                (Dwarf, Male) => 0.0,
                (Dwarf, Female) => 0.0,
                (Undead, Male) => 0.0,
                (Undead, Female) => 0.0,
                (Danari, Male) => 0.0,
                (Danari, Female) => 0.0,
            },
            weapon_x: match Tool::Hammer {
                // TODO: Inventory
                Tool::Sword => 0.0,
                Tool::Axe => 3.0,
                Tool::Hammer => 0.0,
                Tool::Shield => 3.0,
                Tool::Staff => 3.0,
                Tool::Bow => 0.0,
                Tool::Dagger => 0.0,
                Tool::Debug(_) => 0.0,
            },
            weapon_y: match Tool::Hammer {
                // TODO: Inventory
                Tool::Sword => -1.25,
                Tool::Axe => 0.0,
                Tool::Hammer => -2.0,
                Tool::Shield => 0.0,
                Tool::Staff => 0.0,
                Tool::Bow => -2.0,
                Tool::Dagger => -2.0,
                Tool::Debug(_) => 0.0,
            },
        }
    }
}

pub trait Animation {
    type Skeleton;
    type Dependency;

    /// Returns a new skeleton that is generated by the animation.
    fn update_skeleton(
        skeleton: &Self::Skeleton,
        dependency: Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton;
}
