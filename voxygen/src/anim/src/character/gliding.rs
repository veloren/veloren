use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::ToolKind;
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct GlidingAnimation;

impl Animation for GlidingAnimation {
    type Dependency = (Option<ToolKind>, Vec3<f32>, Vec3<f32>, Vec3<f32>, f64);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_gliding\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_gliding")]
    #[allow(clippy::useless_conversion)] // TODO: Pending review in #587
    fn update_skeleton_inner(
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
            .map(|m| m > 0.0001 && m.is_finite())
            .reduce_and()
            && ori.angle_between(last_ori).is_finite()
        {
            ori.angle_between(last_ori).min(0.05)
                * last_ori.determine_side(Vec2::zero(), ori).signum()
        } else {
            0.0
        };

        let tiltcancel = if anim_time > 1.0 {
            1.0
        } else {
            anim_time as f32
        };

        next.head.offset = Vec3::new(0.0, -2.0 + skeleton_attr.head.0, skeleton_attr.head.1);
        next.head.ori = Quaternion::rotation_x(0.35 - slow * 0.10 + head_look.y)
            * Quaternion::rotation_z(head_look.x + slowa * 0.15);

        next.chest.offset = Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1);
        next.chest.ori = Quaternion::rotation_z(slowa * 0.02);

        next.belt.offset = Vec3::new(0.0, 0.0, -2.0);
        next.belt.ori = Quaternion::rotation_z(slowa * 0.1 + tilt * tiltcancel * 12.0);

        next.shorts.offset = Vec3::new(0.0, skeleton_attr.shorts.0, skeleton_attr.shorts.1);
        next.shorts.ori = Quaternion::rotation_z(slowa * 0.12 + tilt * tiltcancel * 16.0);

        next.l_hand.offset = Vec3::new(-9.5, -3.0, 10.0);
        next.l_hand.ori = Quaternion::rotation_x(-2.7 + slowa * -0.1);

        next.r_hand.offset = Vec3::new(9.5, -3.0, 10.0);
        next.r_hand.ori = Quaternion::rotation_x(-2.7 + slowa * -0.10);

        next.l_foot.offset = Vec3::new(
            -skeleton_attr.foot.0,
            skeleton_attr.foot.1 + slowa * -1.0 + tilt * tiltcancel * -35.0,
            -1.0 + skeleton_attr.foot.2,
        );
        next.l_foot.ori = Quaternion::rotation_x(
            (wave_stop * -0.7 - quicka * -0.21 + slow * 0.19) * speed * 0.04,
        ) * Quaternion::rotation_z(tilt * tiltcancel * 20.0);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(
            skeleton_attr.foot.0,
            skeleton_attr.foot.1 + slowa * 1.0 + tilt * tiltcancel * 35.0,
            -1.0 + skeleton_attr.foot.2,
        );
        next.r_foot.ori = Quaternion::rotation_x(
            (wave_stop * -0.8 + quick * -0.25 + slowb * 0.13) * speed * 0.04,
        ) * Quaternion::rotation_z(tilt * tiltcancel * 20.0);
        next.r_foot.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(
            -skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.glider.offset = Vec3::new(0.0, -13.0 + slow * 0.10, 8.0);
        next.glider.ori = Quaternion::rotation_x(0.8) * Quaternion::rotation_y(slowa * 0.04);
        next.glider.scale = Vec3::one();

        next.main.offset = Vec3::new(-7.0, -5.0, 15.0);
        next.main.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
        next.main.scale = Vec3::one();

        next.second.scale = Vec3::one() * 0.0;

        next.lantern.offset = Vec3::new(
            skeleton_attr.lantern.0,
            skeleton_attr.lantern.1,
            skeleton_attr.lantern.2,
        );
        next.lantern.scale = Vec3::one() * 0.65;

        next.torso.offset = Vec3::new(0.0, -4.0, 0.0) / 11.0 * skeleton_attr.scaler;
        next.torso.ori = Quaternion::rotation_x(-0.06 * speed.max(12.0) + slow * 0.04)
            * Quaternion::rotation_y(tilt * tiltcancel * 32.0);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next.control.scale = Vec3::one();

        next.l_control.scale = Vec3::one();

        next.r_control.scale = Vec3::one();
        next
    }
}
