use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::ToolKind;
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (Option<ToolKind>, Vec3<f32>, Vec3<f32>, Vec3<f32>, f64);
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
        *rate = 1.0;

        let walkintensity = if speed > 5.0 { 1.0 } else { 0.7 };
        let walk = if speed > 5.0 { 1.0 } else { 0.5 };
        let lower = if speed > 5.0 { 0.0 } else { 2.0 };
        let snapfoot = if speed > 5.0 { 1.1 } else { 2.0 };
        let lab = 1.0;
        let long = (((5.0)
            / (1.5 + 3.5 * ((anim_time as f32 * lab as f32 * 8.0 * walk).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 8.0 * walk).sin());

        let short = (((5.0)
            / (1.5
                + 3.5 * ((anim_time as f32 * lab as f32 * 16.0 * walk).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 16.0 * walk).sin());
        let noisea = (anim_time as f32 * 11.0 + PI / 6.0).sin();
        let noiseb = (anim_time as f32 * 19.0 + PI / 4.0).sin();

        let shorte = (((5.0)
            / (4.0
                + 1.0 * ((anim_time as f32 * lab as f32 * 16.0 * walk).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 16.0 * walk).sin());

        let shortalt = (((5.0)
            / (1.5
                + 3.5
                    * ((anim_time as f32 * lab as f32 * 16.0 * walk + PI / 2.0).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 16.0 * walk + PI / 2.0).sin());

        let foot = (((5.0)
            / (snapfoot
                + (5.0 - snapfoot)
                    * ((anim_time as f32 * lab as f32 * 16.0 * walk).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 16.0 * walk).sin());

        let wave_stop = (anim_time as f32 * 26.0).min(PI / 2.0 / 2.0).sin();

        let head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 18.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.2,
            ((global_time + anim_time) as f32 / 18.0)
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
            skeleton_attr.neck_height + 13.0 + short * 0.3,
        );
        next.head.ori = Quaternion::rotation_z(head_look.x + long * -0.1 - short * 0.3)
            * Quaternion::rotation_x(head_look.y + 0.35);
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(0.0, 0.0, 10.5 + short * 1.1 - lower);
        next.chest.ori = Quaternion::rotation_z(short * 0.3 * walkintensity);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0, 0.0, -2.0);
        next.belt.ori = Quaternion::rotation_z(short * 0.25);
        next.belt.scale = Vec3::one();

        next.back.offset = Vec3::new(0.0, -2.8, 7.25);
        next.back.ori = Quaternion::rotation_x(-0.25 + short * 0.1 + noisea * 0.1 + noiseb * 0.1);
        next.back.scale = Vec3::one() * 1.02;

        next.shorts.offset = Vec3::new(0.0, 0.0, -5.0);
        next.shorts.ori = Quaternion::rotation_z(short * 0.4);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(
            -6.0 + wave_stop * -1.0 * walkintensity,
            -0.25 + short * 3.0 * walkintensity,
            5.0 + short * -1.5 * walkintensity,
        );
        next.l_hand.ori = Quaternion::rotation_x(0.8 + short * 1.2 * walk)
            * Quaternion::rotation_y(wave_stop * 0.1);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(
            6.0 + wave_stop * 1.0 * walkintensity,
            -0.25 + short * -3.0 * walkintensity,
            5.0 + short * 1.5 * walkintensity,
        );
        next.r_hand.ori = Quaternion::rotation_x(0.8 + short * -1.2 * walk)
            * Quaternion::rotation_y(wave_stop * -0.1);
        next.r_hand.scale = Vec3::one();

        next.l_foot.offset = Vec3::new(-3.4, foot * 1.0, 9.5);
        next.l_foot.ori = Quaternion::rotation_x(foot * -1.2 * walkintensity);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(3.4, foot * -1.0, 9.5);
        next.r_foot.ori = Quaternion::rotation_x(foot * 1.2 * walkintensity);
        next.r_foot.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(-5.0, -1.0, 4.7);
        next.l_shoulder.ori = Quaternion::rotation_x(short * 0.15 * walkintensity);
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(5.0, -1.0, 4.7);
        next.r_shoulder.ori = Quaternion::rotation_x(short * -0.15 * walkintensity);
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.glider.offset = Vec3::new(0.0, 0.0, 10.0);
        next.glider.ori = Quaternion::rotation_y(0.0);
        next.glider.scale = Vec3::one() * 0.0;

        next.main.offset = Vec3::new(
            -7.0 + skeleton_attr.weapon_x,
            -6.5 + skeleton_attr.weapon_y,
            15.0,
        );
        next.main.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57 + short * 0.25);
        next.main.scale = Vec3::one();

        next.second.offset = Vec3::new(
            0.0 + skeleton_attr.weapon_x,
            0.0 + skeleton_attr.weapon_y,
            0.0,
        );
        next.second.ori = Quaternion::rotation_y(0.0);
        next.second.scale = Vec3::one() * 0.0;

        next.lantern.offset = Vec3::new(-5.0, 2.5, 5.5);
        next.lantern.ori =
            Quaternion::rotation_x(shorte * -0.7 + 0.4) * Quaternion::rotation_y(shorte * 0.4);
        next.lantern.scale = Vec3::one() * 0.65;

        next.torso.offset = Vec3::new(0.0, -0.3 + shortalt * -0.065, 0.0) * skeleton_attr.scaler;
        next.torso.ori =
            Quaternion::rotation_x(wave_stop * speed * -0.05 + wave_stop * speed * -0.005)
                * Quaternion::rotation_y(tilt);
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
