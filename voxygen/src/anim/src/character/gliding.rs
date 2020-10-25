use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::ToolKind;
use std::{f32::consts::PI, ops::Mul};

pub struct GlidingAnimation;

type GlidingAnimationDependency = (
    Option<ToolKind>,
    Option<ToolKind>,
    Vec3<f32>,
    Vec3<f32>,
    Vec3<f32>,
    f64,
);

impl Animation for GlidingAnimation {
    type Dependency = GlidingAnimationDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_gliding\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_gliding")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, _second_tool_kind, velocity, orientation, last_ori, global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
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

        let ori: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if ::vek::Vec2::new(ori, last_ori)
            .map(|o| o.magnitude_squared())
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

        next.head.orientation = Quaternion::rotation_x(0.35 - slow * 0.10 + head_look.y)
            * Quaternion::rotation_z(head_look.x + slowa * 0.15);

        next.chest.orientation = Quaternion::rotation_z(slowa * 0.02);

        next.belt.orientation = Quaternion::rotation_z(slowa * 0.1 + tilt * tiltcancel * 12.0);

        next.shorts.orientation = Quaternion::rotation_z(slowa * 0.12 + tilt * tiltcancel * 16.0);

        next.hand_l.position = Vec3::new(-9.5, -3.0, 10.0);
        next.hand_l.orientation = Quaternion::rotation_x(-2.7 + slowa * -0.1);

        next.hand_r.position = Vec3::new(9.5, -3.0, 10.0);
        next.hand_r.orientation = Quaternion::rotation_x(-2.7 + slowa * -0.10);

        next.foot_l.position = Vec3::new(
            -s_a.foot.0,
            s_a.foot.1 + slowa * -1.0 + tilt * tiltcancel * -35.0,
            -1.0 + s_a.foot.2,
        );
        next.foot_l.orientation = Quaternion::rotation_x(
            (wave_stop * -0.7 - quicka * -0.21 + slow * 0.19) * speed * 0.04,
        ) * Quaternion::rotation_z(tilt * tiltcancel * 20.0);

        next.foot_r.position = Vec3::new(
            s_a.foot.0,
            s_a.foot.1 + slowa * 1.0 + tilt * tiltcancel * 35.0,
            -1.0 + s_a.foot.2,
        );
        next.foot_r.orientation = Quaternion::rotation_x(
            (wave_stop * -0.8 + quick * -0.25 + slowb * 0.13) * speed * 0.04,
        ) * Quaternion::rotation_z(tilt * tiltcancel * 20.0);

        next.glider.position = Vec3::new(0.0, -13.0 + slow * 0.10, 8.0);
        next.glider.orientation =
            Quaternion::rotation_x(0.8) * Quaternion::rotation_y(slowa * 0.04);
        next.glider.scale = Vec3::one();

        next.torso.position = Vec3::new(0.0, -4.0, 10.0) / 11.0 * s_a.scaler;
        next.torso.orientation = Quaternion::rotation_x(-0.06 * speed.max(12.0) + slow * 0.04)
            * Quaternion::rotation_y(tilt * tiltcancel * 32.0);

        next
    }
}
