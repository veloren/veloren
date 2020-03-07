use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::ToolKind;
use std::f32::consts::PI;

use vek::*;

pub struct WieldAnimation;

impl Animation for WieldAnimation {
    type Dependency = (Option<ToolKind>, f32, f64);
    type Skeleton = CharacterSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (active_tool_kind, velocity, _global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let speed = Vec2::<f32>::from(velocity).magnitude();
        *rate = speed;

        let wave_ultra_slow = (anim_time as f32 * 3.0 + PI).sin();
        let wave_ultra_slow_cos = (anim_time as f32 * 3.0 + PI).cos();

        let wave = (anim_time as f32 * 1.0).sin();
        let wave_stop = (anim_time as f32 * 2.6).min(PI / 2.0).sin();
        match active_tool_kind {
            //TODO: Inventory
            Some(ToolKind::Sword(_)) => {
                next.l_hand.offset = Vec3::new(0.0, -5.0, -5.0);
                next.l_hand.ori = Quaternion::rotation_x(1.27);
                next.l_hand.scale = Vec3::one() * 1.04;
                next.r_hand.offset = Vec3::new(0.0, -6.0, -8.0);
                next.r_hand.ori = Quaternion::rotation_x(1.27);
                next.r_hand.scale = Vec3::one() * 1.05;
                next.main.offset = Vec3::new(0.0, 0.0, -6.0);
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.main.scale = Vec3::one();

                next.control.offset = Vec3::new(-8.0, 4.0, 6.0);
                next.control.ori = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.control.scale = Vec3::one();
            },
            Some(ToolKind::Axe) => {
                next.l_hand.offset = Vec3::new(-6.5, -0.5, 6.0);
                next.l_hand.ori = Quaternion::rotation_x(0.13) * Quaternion::rotation_z(-0.25);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(-3.0, 6.5, 6.0);
                next.r_hand.ori = Quaternion::rotation_x(0.13)
                    * Quaternion::rotation_z(2.98)
                    * Quaternion::rotation_y(-0.50);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.main.offset = Vec3::new(
                    -5.0 + skeleton_attr.weapon_x,
                    8.5 + skeleton_attr.weapon_y,
                    -0.5,
                );
                next.main.ori = Quaternion::rotation_x(1.70)
                    * Quaternion::rotation_y(-0.25)
                    * Quaternion::rotation_z(0.0);
                next.main.scale = Vec3::one();
            },
            Some(ToolKind::Hammer) => {
                next.l_hand.offset = Vec3::new(-7.0, 4.0, 3.0);
                next.l_hand.ori = Quaternion::rotation_x(1.27 + wave * 0.25)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(7.0, 2.5, -1.25);
                next.r_hand.ori = Quaternion::rotation_x(1.27 + wave * 0.25)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(-0.3);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.main.offset = Vec3::new(
                    5.0 + skeleton_attr.weapon_x,
                    8.75 + skeleton_attr.weapon_y,
                    -2.0,
                );
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(-1.27)
                    * Quaternion::rotation_z(wave * -0.25);
                next.main.scale = Vec3::one();
            },
            Some(ToolKind::Staff) => {
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
            Some(ToolKind::Shield) => {
                next.l_hand.offset = Vec3::new(-6.0, 3.5, 0.0);
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
            Some(ToolKind::Bow) => {
                next.l_hand.offset = Vec3::new(
                    -1.0 - wave_ultra_slow_cos * 1.0,
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
            Some(ToolKind::Dagger) => {
                next.l_hand.offset = Vec3::new(-6.0, 3.5, 0.0);
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
            Some(ToolKind::Debug(_)) => {
                next.l_hand.offset = Vec3::new(-7.0, 4.0, 3.0);
                next.l_hand.ori = Quaternion::rotation_x(1.27 + wave * 0.25)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(7.0, 2.5, -1.25);
                next.r_hand.ori = Quaternion::rotation_x(1.27 + wave * 0.25)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(-0.3);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.main.offset = Vec3::new(
                    5.0 + skeleton_attr.weapon_x,
                    8.75 + skeleton_attr.weapon_y,
                    -2.0,
                );
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(-1.27)
                    * Quaternion::rotation_z(wave * -0.25);
                next.main.scale = Vec3::one();
            },
            _ => {},
        }
        next.torso.offset = Vec3::new(0.0, 0.3 + wave * -0.08, 0.4) * skeleton_attr.scaler;
        next.torso.ori = Quaternion::rotation_x(wave_stop * -0.2);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next.l_control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.l_control.ori = Quaternion::rotation_x(0.0);
        next.l_control.scale = Vec3::one();

        next.r_control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.r_control.ori = Quaternion::rotation_x(0.0);
        next.r_control.scale = Vec3::one();
        next
    }
}
