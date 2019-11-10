use super::{
    super::{Animation, SkeletonAttr},
    CharacterSkeleton,
};
use common::comp::item::Tool;
use std::f32::consts::PI;
use std::ops::Mul;
use vek::*;

pub struct WieldAnimation;

impl Animation<'_>for WieldAnimation {
    type Skeleton = CharacterSkeleton;
    type Dependency = (f32, f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (velocity, global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 12.0).sin();
        let wave_cos = (anim_time as f32 * 12.0).cos();
        let wave_diff = (anim_time as f32 * 12.0 + PI / 2.0).sin();
        let wave_cos_dub = (anim_time as f32 * 24.0).cos();
        let wave_stop = (anim_time as f32 * 5.0).min(PI / 2.0).sin();

        let head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 2.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.2,
            ((global_time + anim_time) as f32 / 2.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.1,
        );

        match Tool::Bow {
            //TODO: Inventory
            Tool::Sword => {
                next.l_hand.offset = Vec3::new(-6.0, 3.75, 0.25);
                next.l_hand.ori = Quaternion::rotation_x(-0.3);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(-6.0, 3.0, -2.0);
                next.r_hand.ori = Quaternion::rotation_x(-0.3);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.weapon.offset = Vec3::new(
                    -6.0 + skeleton_attr.weapon_x,
                    4.0 + skeleton_attr.weapon_y,
                    0.0,
                );
                next.weapon.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.weapon.scale = Vec3::one();
            }
            Tool::Axe => {
                next.l_hand.offset = Vec3::new(-6.0, 3.5, 0.0);
                next.l_hand.ori = Quaternion::rotation_x(-0.3);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(-6.0, 3.0, -2.0);
                next.r_hand.ori = Quaternion::rotation_x(-0.3);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.weapon.offset = Vec3::new(
                    -6.0 + skeleton_attr.weapon_x,
                    4.5 + skeleton_attr.weapon_y,
                    0.0,
                );
                next.weapon.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.weapon.scale = Vec3::one();
            }
            Tool::Hammer => {
                next.l_hand.offset = Vec3::new(-7.0, 8.25, 3.0);
                next.l_hand.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(-1.2)
                    * Quaternion::rotation_z(wave * -0.25);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(7.0, 7.0, -1.5);
                next.r_hand.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(-1.2)
                    * Quaternion::rotation_z(wave * -0.25);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.weapon.offset = Vec3::new(
                    5.0 + skeleton_attr.weapon_x,
                    8.75 + skeleton_attr.weapon_y,
                    -2.0,
                );
                next.weapon.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(-1.2)
                    * Quaternion::rotation_z(wave * -0.25);
                next.weapon.scale = Vec3::one();
            }
            Tool::Staff => {
                next.l_hand.offset = Vec3::new(-6.0, 3.5, 0.0);
                next.l_hand.ori = Quaternion::rotation_x(-0.3);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(-6.0, 3.0, -2.0);
                next.r_hand.ori = Quaternion::rotation_x(-0.3);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.weapon.offset = Vec3::new(
                    -6.0 + skeleton_attr.weapon_x,
                    4.5 + skeleton_attr.weapon_y,
                    0.0,
                );
                next.weapon.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.weapon.scale = Vec3::one();
            }
            Tool::SwordShield => {
                next.l_hand.offset = Vec3::new(-6.0, 3.5, 0.0);
                next.l_hand.ori = Quaternion::rotation_x(-0.3);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(-6.0, 3.0, -2.0);
                next.r_hand.ori = Quaternion::rotation_x(-0.3);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.weapon.offset = Vec3::new(
                    -6.0 + skeleton_attr.weapon_x,
                    4.5 + skeleton_attr.weapon_y,
                    0.0,
                );
                next.weapon.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.weapon.scale = Vec3::one();
            }
            Tool::Bow => {
                next.l_hand.offset = Vec3::new(-4.0, 5.0, 0.0);
                next.l_hand.ori = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(-1.9)
                    * Quaternion::rotation_z(0.85);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(2.0, 8.0, -3.5);
                next.r_hand.ori = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(-1.7)
                    * Quaternion::rotation_z(0.85);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.weapon.offset = Vec3::new(
                    9.0 + skeleton_attr.weapon_x,
                    10.0 + skeleton_attr.weapon_y,
                    -3.0,
                );
                next.weapon.ori = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(-1.7)
                    * Quaternion::rotation_z(0.85 + 3.14);
                next.weapon.scale = Vec3::one();
            }
            Tool::Daggers => {
                next.l_hand.offset = Vec3::new(-6.0, 3.5, 0.0);
                next.l_hand.ori = Quaternion::rotation_x(-0.3);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(-6.0, 3.0, -2.0);
                next.r_hand.ori = Quaternion::rotation_x(-0.3);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.weapon.offset = Vec3::new(
                    -6.0 + skeleton_attr.weapon_x,
                    4.5 + skeleton_attr.weapon_y,
                    0.0,
                );
                next.weapon.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.weapon.scale = Vec3::one();
            }
        }

        next
    }
}
