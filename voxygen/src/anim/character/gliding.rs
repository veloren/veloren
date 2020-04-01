use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::ToolKind;
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct GlidingAnimation;

impl Animation for GlidingAnimation {
    type Dependency = (Option<ToolKind>, Vec3<f32>, Vec3<f32>, Vec3<f32>, f64);
    type Skeleton = CharacterSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, velocity, orientation, last_ori, global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let speed = Vec2::<f32>::from(velocity).magnitude();

        let quick = (anim_time as f32 * 7.0).sin();
        let quicka = (anim_time as f32 * 7.0 + PI / 2.0).sin();
        let wave_stop = (anim_time as f32 * 1.5).min(PI / 2.0).sin();
        let slow = (anim_time as f32 * 3.0).sin();
        let slowb = (anim_time as f32 * 3.0 + PI).sin();
        let slowa = (anim_time as f32 * 3.0 + PI / 2.0).sin();

        let head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 4.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.5,
            ((global_time + anim_time) as f32 / 4.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.25,
        );

        let ori = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);

        let tilt = if Vec2::new(ori, last_ori)
            .map(|o| Vec2::<f32>::from(o).magnitude_squared())
            .map(|m| m > 0.001 && m.is_finite())
            .reduce_and()
            && ori.angle_between(last_ori).is_finite()
        {
            ori.angle_between(last_ori).min(0.15)
                * last_ori.determine_side(Vec2::zero(), ori).signum()
        } else {
            0.0
        } * 0.8;

        next.head.offset = Vec3::new(
            0.0 + skeleton_attr.neck_right,
            -2.0 + skeleton_attr.neck_forward,
            skeleton_attr.neck_height + 12.0,
        );
        next.head.ori = Quaternion::rotation_x(0.35 - slow * 0.10 + head_look.y)
            * Quaternion::rotation_z(head_look.x + slowa * 0.15);
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(0.0, 0.0, -2.0);
        next.chest.ori = Quaternion::rotation_z(slowa * 0.2);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0, 0.0, -2.0);
        next.belt.ori = Quaternion::rotation_z(slowa * 0.25);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(0.0, 0.0, -5.0);
        next.shorts.ori = Quaternion::rotation_z(slowa * 0.35);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(-9.5 + slowa * -1.5, -3.0 + slowa * 1.5, 6.0);
        next.l_hand.ori = Quaternion::rotation_x(-2.7 + slowa * -0.1);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(9.5 + slowa * -1.5, -3.0 + slowa * -1.5, 6.0);
        next.r_hand.ori = Quaternion::rotation_x(-2.7 + slowa * -0.10);
        next.r_hand.scale = Vec3::one();

        next.l_foot.offset = Vec3::new(-3.4, 1.0, -2.0);
        next.l_foot.ori = Quaternion::rotation_x(
            (wave_stop * -0.7 - quicka * -0.21 + slow * 0.19) * speed * 0.04,
        );

        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(3.4, 1.0, -2.0);
        next.r_foot.ori = Quaternion::rotation_x(
            (wave_stop * -0.8 + quick * -0.25 + slowb * 0.13) * speed * 0.04,
        );
        next.r_foot.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(-5.0, 0.0, 4.7);
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(5.0, 0.0, 4.7);
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.glider.offset = Vec3::new(0.0, -13.0 + slow * 0.10, 6.0);
        next.glider.ori = Quaternion::rotation_x(1.0) * Quaternion::rotation_y(slowa * 0.04);
        next.glider.scale = Vec3::one();

        next.main.offset = Vec3::new(
            -7.0 + skeleton_attr.weapon_x,
            -5.0 + skeleton_attr.weapon_y,
            15.0,
        );
        next.main.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
        next.main.scale = Vec3::one();

        next.second.offset = Vec3::new(
            0.0 + skeleton_attr.weapon_x,
            0.0 + skeleton_attr.weapon_y,
            0.0,
        );
        next.second.ori = Quaternion::rotation_y(0.0);
        next.second.scale = Vec3::one() * 0.0;

        next.lantern.offset = Vec3::new(0.0, 0.0, 0.0);
        next.lantern.ori = Quaternion::rotation_x(0.0);
        next.lantern.scale = Vec3::one() * 0.0;

        next.torso.offset = Vec3::new(0.0, 6.0, 15.0) / 11.0 * skeleton_attr.scaler;
        next.torso.ori = Quaternion::rotation_x(-0.05 * speed.max(12.0) + slow * 0.10)
            * Quaternion::rotation_y(tilt * 16.0);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next.control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.control.ori = Quaternion::rotation_x(0.0);
        next.control.scale = Vec3::one();

        next.l_control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.l_control.ori = Quaternion::rotation_x(0.0);
        next.l_control.scale = Vec3::one();

        next.r_control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.r_control.ori = Quaternion::rotation_x(0.0);
        next.r_control.scale = Vec3::one();
        next
    }
}
