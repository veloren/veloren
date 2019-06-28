pub mod character;
pub mod fixture;
pub mod quadruped;
pub mod quadrupedmedium;

use crate::render::FigureBoneData;
use common::comp::actor::{BodyType, HumanoidBody, Race, Weapon};
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

impl<'a> From<&'a HumanoidBody> for SkeletonAttr {
    fn from(body: &'a HumanoidBody) -> Self {
        Self {
            scaler: match (body.race, body.body_type) {
                (Race::Orc, BodyType::Male) => 1.2,
                (Race::Orc, BodyType::Female) => 1.0,
                (Race::Human, BodyType::Male) => 1.0,
                (Race::Human, BodyType::Female) => 0.90,
                (Race::Elf, BodyType::Male) => 1.0,
                (Race::Elf, BodyType::Female) => 1.0,
                (Race::Dwarf, BodyType::Male) => 0.92,
                (Race::Dwarf, BodyType::Female) => 0.89,
                (Race::Undead, BodyType::Male) => 0.98,
                (Race::Undead, BodyType::Female) => 0.93,
                (Race::Danari, BodyType::Male) => 0.85,
                (Race::Danari, BodyType::Female) => 0.82,
                _ => 1.0,
            },
            head_scale: match (body.race, body.body_type) {
                (Race::Orc, BodyType::Male) => 0.9,
                (Race::Orc, BodyType::Female) => 1.0,
                (Race::Human, BodyType::Male) => 1.0,
                (Race::Human, BodyType::Female) => 1.0,
                (Race::Elf, BodyType::Male) => 1.0,
                (Race::Elf, BodyType::Female) => 1.0,
                (Race::Dwarf, BodyType::Male) => 1.0,
                (Race::Dwarf, BodyType::Female) => 1.0,
                (Race::Undead, BodyType::Male) => 1.0,
                (Race::Undead, BodyType::Female) => 1.0,
                (Race::Danari, BodyType::Male) => 1.11,
                (Race::Danari, BodyType::Female) => 1.11,
                _ => 1.0,
            },
            neck_height: match (body.race, body.body_type) {
                (Race::Orc, BodyType::Male) => -2.0,
                (Race::Orc, BodyType::Female) => -2.0,
                (Race::Human, BodyType::Male) => 6.0,
                (Race::Human, BodyType::Female) => -2.0,
                (Race::Elf, BodyType::Male) => 0.75,
                (Race::Elf, BodyType::Female) => -1.25,
                (Race::Dwarf, BodyType::Male) => -0.0,
                (Race::Dwarf, BodyType::Female) => -1.0,
                (Race::Undead, BodyType::Male) => -1.0,
                (Race::Undead, BodyType::Female) => -0.5,
                (Race::Danari, BodyType::Male) => 0.5,
                (Race::Danari, BodyType::Female) => -0.5,
                _ => 1.0,
            },
            neck_forward: match (body.race, body.body_type) {
                (Race::Orc, BodyType::Male) => 1.0,
                (Race::Orc, BodyType::Female) => -1.0,
                (Race::Human, BodyType::Male) => 0.0,
                (Race::Human, BodyType::Female) => -1.0,
                (Race::Elf, BodyType::Male) => 1.75,
                (Race::Elf, BodyType::Female) => -0.5,
                (Race::Dwarf, BodyType::Male) => 2.0,
                (Race::Dwarf, BodyType::Female) => 0.0,
                (Race::Undead, BodyType::Male) => 1.0,
                (Race::Undead, BodyType::Female) => 1.0,
                (Race::Danari, BodyType::Male) => 0.5,
                (Race::Danari, BodyType::Female) => 0.0,
                _ => 1.0,
            },
            neck_right: match (body.race, body.body_type) {
                (Race::Orc, BodyType::Male) => 0.0,
                (Race::Orc, BodyType::Female) => 0.0,
                (Race::Human, BodyType::Male) => 0.0,
                (Race::Human, BodyType::Female) => 0.0,
                (Race::Elf, BodyType::Male) => -1.0,
                (Race::Elf, BodyType::Female) => 0.25,
                (Race::Dwarf, BodyType::Male) => 0.0,
                (Race::Dwarf, BodyType::Female) => 0.0,
                (Race::Undead, BodyType::Male) => -0.5,
                (Race::Undead, BodyType::Female) => 0.0,
                (Race::Danari, BodyType::Male) => 0.0,
                (Race::Danari, BodyType::Female) => 0.0,
                _ => 1.0,
            },
            weapon_x: match body.weapon {
                Weapon::Sword => 0.0,
                Weapon::Axe => 3.0,
                Weapon::Hammer => 0.0,
                Weapon::SwordShield => 3.0,
                Weapon::Staff => 3.0,
                Weapon::Bow => 0.0,
                Weapon::Daggers => 0.0,

                _ => 1.0,
            },
            weapon_y: match body.weapon {
                Weapon::Sword => -1.25,
                Weapon::Axe => 0.0,
                Weapon::Hammer => -2.0,
                Weapon::SwordShield => 0.0,
                Weapon::Staff => 0.0,
                Weapon::Bow => -2.0,
                Weapon::Daggers => -2.0,

                _ => 1.0,
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
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton;
}
