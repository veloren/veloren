use super::{
    super::{Animation, SkeletonAttr},
    CharacterSkeleton,
};
use common::comp::item::Tool;
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct Input {
    pub attack: bool,
}
pub struct BlockAnimation;

impl Animation for BlockAnimation {
    type Skeleton = CharacterSkeleton;
    type Dependency = f64;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        global_time: f64,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave_ultra_slow = (anim_time as f32 * 3.0 + PI).sin();
        let wave_ultra_slow_cos = (anim_time as f32 * 3.0 + PI).cos();
        let wave_slow_cos = (anim_time as f32 * 6.0 + PI).cos();
        let _wave_slow = (anim_time as f32 * 6.0 + PI).sin();

        let _head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 1.5)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.3,
            ((global_time + anim_time) as f32 / 1.5)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.15,
        );
        next.head.offset = Vec3::new(
            0.0 + skeleton_attr.neck_right + wave_slow_cos * 0.2,
            -1.0 + skeleton_attr.neck_forward,
            skeleton_attr.neck_height + 19.5 + wave_ultra_slow * 0.2,
        );
        next.head.ori = Quaternion::rotation_x(-0.25);
        next.head.scale = Vec3::one() * 1.01 * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(0.0 + wave_slow_cos * 0.2, 0.0, 5.0 + wave_ultra_slow * 0.2);
        next.chest.ori =
            Quaternion::rotation_x(-0.15) * Quaternion::rotation_y(wave_ultra_slow_cos * 0.01);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0 + wave_slow_cos * 0.2, 0.0, 3.0 + wave_ultra_slow * 0.2);
        next.belt.ori =
            Quaternion::rotation_x(0.0) * Quaternion::rotation_y(wave_ultra_slow_cos * 0.008);
        next.belt.scale = Vec3::one() * 1.01;

        next.shorts.offset = Vec3::new(0.0 + wave_slow_cos * 0.2, 0.0, 1.0 + wave_ultra_slow * 0.2);
        next.shorts.ori = Quaternion::rotation_x(0.1);
        next.shorts.scale = Vec3::one();

        match Tool::Hammer {
            //TODO: Inventory
            Tool::Sword => {
                next.l_hand.offset = Vec3::new(-6.0, 3.5, 0.0 + wave_ultra_slow * 1.0);
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
            Tool::Axe => {
                next.l_hand.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    3.5 + wave_ultra_slow_cos * 0.5,
                    0.0 + wave_ultra_slow * 1.0,
                );
                next.l_hand.ori = Quaternion::rotation_x(-0.3);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    3.0 + wave_ultra_slow_cos * 0.5,
                    -2.0 + wave_ultra_slow * 1.0,
                );
                next.r_hand.ori = Quaternion::rotation_x(-0.3);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.weapon.offset = Vec3::new(
                    -6.0 + skeleton_attr.weapon_x,
                    4.5 + skeleton_attr.weapon_y,
                    0.0 + wave_ultra_slow * 1.0,
                );
                next.weapon.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.weapon.scale = Vec3::one();
            }
            Tool::Hammer => {
                next.l_hand.offset = Vec3::new(-7.0, 3.5, 6.5);
                next.l_hand.ori = Quaternion::rotation_x(2.07)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(-0.2);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(7.0, 2.5, 3.75);
                next.r_hand.ori = Quaternion::rotation_x(2.07)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(-0.2);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.weapon.offset = Vec3::new(
                    5.0 + skeleton_attr.weapon_x,
                    8.75 + skeleton_attr.weapon_y,
                    5.5,
                );
                next.weapon.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(-1.35)
                    * Quaternion::rotation_z(-0.85);
                next.weapon.scale = Vec3::one();
            }
            Tool::Staff => {
                next.l_hand.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    3.5 + wave_ultra_slow_cos * 0.5,
                    0.0 + wave_ultra_slow * 1.0,
                );
                next.l_hand.ori = Quaternion::rotation_x(-0.3);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    3.0 + wave_ultra_slow_cos * 0.5,
                    -2.0 + wave_ultra_slow * 1.0,
                );
                next.r_hand.ori = Quaternion::rotation_x(-0.3);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.weapon.offset = Vec3::new(
                    -6.0 + skeleton_attr.weapon_x + wave_ultra_slow_cos * 1.0,
                    4.5 + skeleton_attr.weapon_y + wave_ultra_slow_cos * 0.5,
                    0.0 + wave_ultra_slow * 1.0,
                );
                next.weapon.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.weapon.scale = Vec3::one();
            }
            Tool::SwordShield => {
                next.l_hand.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    3.5 + wave_ultra_slow_cos * 0.5,
                    0.0 + wave_ultra_slow * 1.0,
                );
                next.l_hand.ori = Quaternion::rotation_x(-0.3);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    3.0 + wave_ultra_slow_cos * 0.5,
                    -2.0 + wave_ultra_slow * 1.0,
                );
                next.r_hand.ori = Quaternion::rotation_x(-0.3);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.weapon.offset = Vec3::new(
                    -6.0 + skeleton_attr.weapon_x + wave_ultra_slow_cos * 1.0,
                    4.5 + skeleton_attr.weapon_y + wave_ultra_slow_cos * 0.5,
                    0.0 + wave_ultra_slow * 1.0,
                );
                next.weapon.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.weapon.scale = Vec3::one();
            }
            Tool::Bow => {
                next.l_hand.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    3.5 + wave_ultra_slow_cos * 0.5,
                    0.0 + wave_ultra_slow * 1.0,
                );
                next.l_hand.ori = Quaternion::rotation_x(-0.3);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    3.0 + wave_ultra_slow_cos * 0.5,
                    -2.0 + wave_ultra_slow * 1.0,
                );
                next.r_hand.ori = Quaternion::rotation_x(-0.3);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.weapon.offset = Vec3::new(
                    -6.0 + skeleton_attr.weapon_x + wave_ultra_slow_cos * 1.0,
                    4.5 + skeleton_attr.weapon_y + wave_ultra_slow_cos * 0.5,
                    0.0 + wave_ultra_slow * 1.0,
                );
                next.weapon.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.weapon.scale = Vec3::one();
            }
            Tool::Daggers => {
                next.l_hand.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    3.5 + wave_ultra_slow_cos * 0.5,
                    0.0 + wave_ultra_slow * 1.0,
                );
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
        //next.l_foot.offset = Vec3::new(-3.4, 0.3, 8.0 + wave_ultra_slow_cos * 0.1);
        //next.l_foot.ori = Quaternion::rotation_x(-0.3);
        //next.l_foot.scale = Vec3::one();

        //next.r_foot.offset = Vec3::new(3.4, 1.2, 8.0 + wave_ultra_slow * 0.1);
        //next.r_foot.ori = Quaternion::rotation_x(0.3);
        //next.r_foot.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(-5.0, 0.0, 5.0);
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(5.0, 0.0, 5.0);
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.draw.offset = Vec3::new(0.0, 5.0, 0.0);
        next.draw.ori = Quaternion::rotation_y(0.0);
        next.draw.scale = Vec3::one() * 0.0;

        next.torso.offset = Vec3::new(0.0, -0.2, 0.1) * skeleton_attr.scaler;
        next.torso.ori = Quaternion::rotation_x(0.0);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next
    }
}
