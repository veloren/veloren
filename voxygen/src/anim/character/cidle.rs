use super::{super::Animation, CharacterSkeleton, SkeletonAttr};

use common::comp::item::Tool;
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct Input {
    pub attack: bool,
}
pub struct CidleAnimation;

impl Animation for CidleAnimation {
    type Dependency = (Option<Tool>, f64);
    type Skeleton = CharacterSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (active_tool_kind, global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave_ultra_slow = (anim_time as f32 * 3.0 + PI).sin();
        let wave_ultra_slow_cos = (anim_time as f32 * 3.0 + PI).cos();
        let wave_slow_cos = (anim_time as f32 * 6.0 + PI).cos();
        let wave_slow = (anim_time as f32 * 6.0 + PI).sin();

        let head_look = Vec2::new(
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
            0.0 + skeleton_attr.neck_right + wave_slow_cos * 0.5,
            -2.0 + skeleton_attr.neck_forward,
            skeleton_attr.neck_height + 21.0 + wave_ultra_slow * 0.6,
        );
        next.head.ori =
            Quaternion::rotation_z(head_look.x) * Quaternion::rotation_x(head_look.y.abs());
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(0.0 + wave_slow_cos * 0.5, 0.0, 7.0 + wave_ultra_slow * 0.5);
        next.chest.ori = Quaternion::rotation_y(wave_ultra_slow_cos * 0.04);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0 + wave_slow_cos * 0.5, 0.0, 5.0 + wave_ultra_slow * 0.5);
        next.belt.ori = Quaternion::rotation_y(wave_ultra_slow_cos * 0.03);
        next.belt.scale = Vec3::one() * 1.02;

        next.shorts.offset = Vec3::new(0.0 + wave_slow_cos * 0.5, 0.0, 2.0 + wave_ultra_slow * 0.5);
        next.shorts.ori = Quaternion::rotation_x(0.0);
        next.shorts.scale = Vec3::one();

        match active_tool_kind {
            //TODO: Inventory
            Some(Tool::Sword) => {
                next.l_hand.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    -2.0 + wave_ultra_slow_cos * 0.5,
                    1.0 + wave_ultra_slow * 1.0,
                );
                next.l_hand.ori = Quaternion::rotation_x(1.27);
                next.l_hand.scale = Vec3::one() * 1.0;
                next.r_hand.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    -2.5 + wave_ultra_slow_cos * 0.5,
                    -1.0 + wave_ultra_slow * 1.0,
                );
                next.r_hand.ori = Quaternion::rotation_x(1.27);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.main.offset = Vec3::new(
                    -6.0 + skeleton_attr.weapon_x + wave_ultra_slow_cos * 1.0,
                    5.5 + skeleton_attr.weapon_y + wave_ultra_slow_cos * 0.5,
                    1.0 + wave_ultra_slow * 1.0,
                );
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.main.scale = Vec3::one();
            },
            Some(Tool::Axe) => {
                next.l_hand.offset = Vec3::new(
                    -6.5 + wave_ultra_slow_cos * 1.0,
                    -0.5 + wave_ultra_slow_cos * 0.5,
                    6.0 + wave_ultra_slow * 1.0,
                );
                next.l_hand.ori = Quaternion::rotation_x(0.13) * Quaternion::rotation_z(-0.25);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(
                    -3.0 + wave_ultra_slow_cos * 1.0,
                    6.5 + wave_ultra_slow_cos * 0.5,
                    6.0 + wave_ultra_slow * 1.0,
                );
                next.r_hand.ori = Quaternion::rotation_x(0.13)
                    * Quaternion::rotation_z(2.98)
                    * Quaternion::rotation_y(-0.50);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.main.offset = Vec3::new(
                    -5.0 + skeleton_attr.weapon_x + wave_ultra_slow_cos * 1.0,
                    8.5 + skeleton_attr.weapon_y + wave_ultra_slow_cos * 0.5,
                    -0.5 + wave_ultra_slow * 1.0,
                );
                next.main.ori = Quaternion::rotation_x(1.70)
                    * Quaternion::rotation_y(-0.25)
                    * Quaternion::rotation_z(0.0);
                next.main.scale = Vec3::one();
            },
            Some(Tool::Hammer) => {
                next.l_hand.offset = Vec3::new(-7.0, 4.0, 3.0);
                next.l_hand.ori = Quaternion::rotation_x(1.27 + wave_ultra_slow * -0.1)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(-0.3);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(7.0, 2.5, -1.25);
                next.r_hand.ori = Quaternion::rotation_x(1.27 + wave_ultra_slow * -0.1)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(-0.3);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.main.offset = Vec3::new(
                    5.0 + skeleton_attr.weapon_x,
                    8.75 + skeleton_attr.weapon_y,
                    -2.5,
                );
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(-1.27)
                    * Quaternion::rotation_z(wave_ultra_slow * 0.2);
                next.main.scale = Vec3::one();
            },
            Some(Tool::Staff) => {
                next.l_hand.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    1.0 + wave_ultra_slow_cos * 0.5,
                    5.0 + wave_ultra_slow * 1.0,
                );
                next.l_hand.ori = Quaternion::rotation_x(1.27);
                next.l_hand.scale = Vec3::one() * 1.0;
                next.r_hand.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    -1.5 + wave_ultra_slow_cos * 0.5,
                    -2.0 + wave_ultra_slow * 1.0,
                );
                next.r_hand.ori = Quaternion::rotation_x(1.27);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.main.offset = Vec3::new(
                    -6.0 + skeleton_attr.weapon_x + wave_ultra_slow_cos * 1.0,
                    8.5 + skeleton_attr.weapon_y + wave_ultra_slow_cos * 0.5,
                    17.0 + wave_ultra_slow * 1.0,
                );
                next.main.ori = Quaternion::rotation_x(-0.3 + PI)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.main.scale = Vec3::one();
            },
            Some(Tool::Shield) => {
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
                next.main.offset = Vec3::new(
                    -6.0 + skeleton_attr.weapon_x + wave_ultra_slow_cos * 1.0,
                    4.5 + skeleton_attr.weapon_y + wave_ultra_slow_cos * 0.5,
                    0.0 + wave_ultra_slow * 1.0,
                );
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.main.scale = Vec3::one();
            },
            Some(Tool::Bow) => {
                next.l_hand.offset = Vec3::new(
                    -1.0 + wave_ultra_slow_cos * 1.0,
                    3.0 + wave_ultra_slow_cos * 0.5,
                    5.0 + wave_ultra_slow * 1.0,
                );
                next.l_hand.ori = Quaternion::rotation_x(PI / 2.0)
                    * Quaternion::rotation_y(-0.3)
                    * Quaternion::rotation_z(0.3);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(
                    1.0 + wave_ultra_slow_cos * 1.0,
                    8.0 + wave_ultra_slow_cos * 0.5,
                    2.5 + wave_ultra_slow * 1.0,
                );
                next.r_hand.ori = Quaternion::rotation_x(PI / 2.0)
                    * Quaternion::rotation_y(0.3)
                    * Quaternion::rotation_z(0.3);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.main.offset = Vec3::new(
                    -4.0 + skeleton_attr.weapon_x + wave_ultra_slow_cos * 1.0,
                    15.0 + skeleton_attr.weapon_y + wave_ultra_slow_cos * 0.5,
                    -4.0 + wave_ultra_slow * 1.0,
                );
                next.main.ori = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.4)
                    * Quaternion::rotation_z(0.0);
                next.main.scale = Vec3::one();
            },
            Some(Tool::Dagger) => {
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
                next.main.offset = Vec3::new(
                    -6.0 + skeleton_attr.weapon_x,
                    4.5 + skeleton_attr.weapon_y,
                    0.0,
                );
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.main.scale = Vec3::one();
            },
            Some(Tool::Debug(_)) => {
                next.l_hand.offset = Vec3::new(-7.0, 4.0, 3.0);
                next.l_hand.ori = Quaternion::rotation_x(1.27 + wave_ultra_slow * -0.1)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(-0.3);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(7.0, 2.5, -1.25);
                next.r_hand.ori = Quaternion::rotation_x(1.27 + wave_ultra_slow * -0.1)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(-0.3);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.main.offset = Vec3::new(
                    5.0 + skeleton_attr.weapon_x,
                    8.75 + skeleton_attr.weapon_y,
                    -2.5,
                );
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(-1.27)
                    * Quaternion::rotation_z(wave_ultra_slow * 0.2);
                next.main.scale = Vec3::one();
            },
            _ => {},
        }
        next.l_foot.offset = Vec3::new(-3.4, -1.5, 8.0 + wave_slow * 0.2);
        next.l_foot.ori = Quaternion::rotation_x(wave_ultra_slow_cos * 0.015);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(3.4, 3.0, 8.0 + wave_slow_cos * 0.2);
        next.r_foot.ori = Quaternion::rotation_x(wave_ultra_slow * 0.015);
        next.r_foot.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(-5.0, 0.0, 5.0);
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(5.0, 0.0, 5.0);
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.glider.offset = Vec3::new(0.0, 5.0, 0.0);
        next.glider.ori = Quaternion::rotation_y(0.0);
        next.glider.scale = Vec3::one() * 0.0;

        next.lantern.offset = Vec3::new(0.0, 0.0, 0.0);
        next.lantern.ori = Quaternion::rotation_x(0.0);
        next.lantern.scale = Vec3::one() * 0.0;

        next.torso.offset = Vec3::new(0.0, -0.2, 0.1) * skeleton_attr.scaler;
        next.torso.ori = Quaternion::rotation_x(0.0);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next
    }
}
