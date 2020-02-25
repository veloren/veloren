use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::Tool;
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (Option<Tool>, Vec3<f32>, Vec3<f32>, Vec3<f32>, f64);
    type Skeleton = CharacterSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, velocity, orientation, last_ori, global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let speed = Vec2::<f32>::from(velocity).magnitude();
        *rate = speed;

        let lab = 1.0;
        let long = (((5.0)
            / (1.1 + 3.9 * ((anim_time as f32 * lab as f32 * 1.2).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 1.2).sin());
        let short = (((5.0)
            / (1.1 + 3.9 * ((anim_time as f32 * lab as f32 * 2.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 1.5).sin());

        let wave_stop = (anim_time as f32 * 2.6).min(PI / 2.0).sin();

        let head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 4.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.2,
            ((global_time + anim_time) as f32 / 4.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.1,
        );

        let ori = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if Vec2::new(ori, last_ori)
            .map(|o| Vec2::<f32>::from(o).magnitude_squared())
            .map(|m| m > 0.001 && m.is_finite())
            .reduce_and()
            && ori.angle_between(last_ori).is_finite()
        {
            ori.angle_between(last_ori).min(0.5)
                * last_ori.determine_side(Vec2::zero(), ori).signum()
        } else {
            0.0
        } * 1.3;

        next.head.offset = Vec3::new(
            0.0,
            -3.0 + skeleton_attr.neck_forward,
            skeleton_attr.neck_height + 20.0 + short * 1.3,
        );
        next.head.ori = Quaternion::rotation_z(head_look.x + long * 0.1)
            * Quaternion::rotation_x(head_look.y + 0.35);
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(0.0, 0.0, 7.0 + short * 1.1);
        next.chest.ori = Quaternion::rotation_z(long * 0.2);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0, 0.0, 5.0 + short * 1.1);
        next.belt.ori = Quaternion::rotation_z(long * 0.35);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(0.0, 0.0, 2.0 + short * 1.1);
        next.shorts.ori = Quaternion::rotation_z(long * 0.6);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(
            -6.0 + wave_stop * -1.0,
            -0.25 + short * 2.0,
            5.0 - long * 1.5,
        );
        next.l_hand.ori =
            Quaternion::rotation_x(0.8 + short * 1.2) * Quaternion::rotation_y(wave_stop * 0.1);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(
            6.0 + wave_stop * 1.0,
            -0.25 + short * -2.0,
            5.0 + long * 1.5,
        );
        next.r_hand.ori =
            Quaternion::rotation_x(0.8 + short * -1.2) * Quaternion::rotation_y(wave_stop * -0.1);
        next.r_hand.scale = Vec3::one();

        next.l_foot.offset = Vec3::new(-3.4, 0.0 + short * 1.0, 6.0);
        next.l_foot.ori = Quaternion::rotation_x(-0.0 - short * 1.2);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(3.4, short * -1.0, 6.0);
        next.r_foot.ori = Quaternion::rotation_x(short * 1.2);
        next.r_foot.scale = Vec3::one();

        next.main.offset = Vec3::new(
            -7.0 + skeleton_attr.weapon_x,
            -5.0 + skeleton_attr.weapon_y,
            15.0,
        );
        next.main.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57 + short * 0.25);
        next.main.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(-5.0, -0.5, 4.7);
        next.l_shoulder.ori = Quaternion::rotation_x(short * 0.15);
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(5.0, -0.5, 4.7);
        next.r_shoulder.ori = Quaternion::rotation_x(long * 0.15);
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.glider.offset = Vec3::new(0.0, 5.0, 0.0);
        next.glider.ori = Quaternion::rotation_y(0.0);
        next.glider.scale = Vec3::one() * 0.0;

        next.lantern.offset = Vec3::new(0.0, 5.0, 0.0);
        next.lantern.ori = Quaternion::rotation_y(0.0);
        next.lantern.scale = Vec3::one() * 0.0;

        next.torso.offset = Vec3::new(0.0, -0.3 + long * -0.08, 0.4) * skeleton_attr.scaler;
        next.torso.ori =
            Quaternion::rotation_x(wave_stop * speed * -0.06 + wave_stop * speed * -0.005)
                * Quaternion::rotation_y(tilt);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next
    }
}
