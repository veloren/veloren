use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::ToolKind;
use std::{f32::consts::PI, ops::Mul};

pub struct SneakAnimation;

impl Animation for SneakAnimation {
    type Dependency = (Option<ToolKind>, Vec3<f32>, Vec3<f32>, Vec3<f32>, f64);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_sneak\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_sneak")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, velocity, orientation, last_ori, global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();
        *rate = 1.0;
        let slow = (anim_time as f32 * 3.0).sin();
        let breathe = ((anim_time as f32 * 0.5).sin()).abs();
        let walkintensity = if speed > 5.0 { 1.0 } else { 0.45 };
        let lower = if speed > 5.0 { 0.0 } else { 1.0 };
        let _snapfoot = if speed > 5.0 { 1.1 } else { 2.0 };
        let lab = 1.0;
        let foothoril = (anim_time as f32 * 7.0 * lab as f32 + PI * 1.45).sin();
        let foothorir = (anim_time as f32 * 7.0 * lab as f32 + PI * (0.45)).sin();

        let footvertl = (anim_time as f32 * 7.0 * lab as f32).sin();
        let footvertr = (anim_time as f32 * 7.0 * lab as f32 + PI).sin();

        let footrotl = (((5.0)
            / (2.5
                + (2.5)
                    * ((anim_time as f32 * 7.0 * lab as f32 + PI * 1.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 7.0 * lab as f32 + PI * 1.4).sin());

        let footrotr = (((5.0)
            / (1.0
                + (4.0)
                    * ((anim_time as f32 * 7.0 * lab as f32 + PI * 0.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 7.0 * lab as f32 + PI * 0.4).sin());

        let short = (anim_time as f32 * lab as f32 * 7.0).sin();
        let noisea = (anim_time as f32 * 11.0 + PI / 6.0).sin();
        let noiseb = (anim_time as f32 * 19.0 + PI / 4.0).sin();

        let shorte = (((5.0)
            / (4.0 + 1.0 * ((anim_time as f32 * lab as f32 * 7.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 7.0).sin());

        let shortalt = (anim_time as f32 * lab as f32 * 7.0 + PI / 2.0).sin();

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

        let orientation: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if ::vek::Vec2::new(orientation, last_ori)
            .map(|o| o.magnitude_squared())
            .map(|m| m > 0.001 && m.is_finite())
            .reduce_and()
            && orientation.angle_between(last_ori).is_finite()
        {
            orientation.angle_between(last_ori).min(0.2)
                * last_ori.determine_side(Vec2::zero(), orientation).signum()
        } else {
            0.0
        } * 1.3;

        next.hold.scale = Vec3::one() * 0.0;

        if speed > 0.5 {
            next.hand_l.position = Vec3::new(
                1.0 - skeleton_attr.hand.0,
                4.0 + skeleton_attr.hand.1,
                1.0 + skeleton_attr.hand.2,
            );
            next.hand_l.orientation = Quaternion::rotation_x(1.0);
            next.hand_l.scale = Vec3::one();

            next.hand_r.position = Vec3::new(
                -1.0 + skeleton_attr.hand.0,
                -1.0 + skeleton_attr.hand.1,
                skeleton_attr.hand.2,
            );
            next.hand_r.orientation = Quaternion::rotation_x(0.4);
            next.hand_r.scale = Vec3::one();
            next.head.position = Vec3::new(
                0.0,
                -4.0 + skeleton_attr.head.0,
                -1.0 + skeleton_attr.head.1 + short * 0.06,
            );
            next.head.orientation =
                Quaternion::rotation_z(tilt * -2.5 + head_look.x * 0.2 - short * 0.06)
                    * Quaternion::rotation_x(head_look.y + 0.45);
            next.head.scale = Vec3::one() * skeleton_attr.head_scale;

            next.chest.position = Vec3::new(
                0.0,
                skeleton_attr.chest.0,
                -1.0 + skeleton_attr.chest.1 + shortalt * -0.5,
            );
            next.chest.orientation = Quaternion::rotation_z(0.3 + short * 0.08 + tilt * -0.2)
                * Quaternion::rotation_y(tilt * 0.8)
                * Quaternion::rotation_x(-0.5);
            next.chest.scale = Vec3::one();

            next.belt.position =
                Vec3::new(0.0, 0.5 + skeleton_attr.belt.0, 0.7 + skeleton_attr.belt.1);
            next.belt.orientation = Quaternion::rotation_z(short * 0.1 + tilt * -1.1)
                * Quaternion::rotation_y(tilt * 0.5)
                * Quaternion::rotation_x(0.2);
            next.belt.scale = Vec3::one();

            next.glider.orientation = Quaternion::rotation_x(0.0);
            next.glider.position = Vec3::new(0.0, 0.0, 10.0);
            next.glider.scale = Vec3::one() * 0.0;

            next.back.position = Vec3::new(0.0, skeleton_attr.back.0, skeleton_attr.back.1);
            next.back.orientation =
                Quaternion::rotation_x(-0.25 + short * 0.1 + noisea * 0.1 + noiseb * 0.1);
            next.back.scale = Vec3::one() * 1.02;

            next.shorts.position = Vec3::new(
                0.0,
                1.0 + skeleton_attr.shorts.0,
                1.0 + skeleton_attr.shorts.1,
            );
            next.shorts.orientation = Quaternion::rotation_z(short * 0.16 + tilt * -1.5)
                * Quaternion::rotation_y(tilt * 0.7)
                * Quaternion::rotation_x(0.3);
            next.shorts.scale = Vec3::one();

            next.foot_l.position = Vec3::new(
                -skeleton_attr.foot.0,
                skeleton_attr.foot.1 + foothoril * -10.5 * walkintensity - lower * 1.0,
                1.0 + skeleton_attr.foot.2 + ((footvertl * -1.7).max(-1.0)) * walkintensity,
            );
            next.foot_l.orientation =
                Quaternion::rotation_x(-0.2 + footrotl * -0.8 * walkintensity)
                    * Quaternion::rotation_y(tilt * 1.8);
            next.foot_l.scale = Vec3::one();

            next.foot_r.position = Vec3::new(
                skeleton_attr.foot.0,
                skeleton_attr.foot.1 + foothorir * -10.5 * walkintensity - lower * 1.0,
                1.0 + skeleton_attr.foot.2 + ((footvertr * -1.7).max(-1.0)) * walkintensity,
            );
            next.foot_r.orientation =
                Quaternion::rotation_x(-0.2 + footrotr * -0.8 * walkintensity)
                    * Quaternion::rotation_y(tilt * 1.8);
            next.foot_r.scale = Vec3::one();

            next.shoulder_l.position = Vec3::new(
                -skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_l.orientation = Quaternion::rotation_x(short * 0.15 * walkintensity);
            next.shoulder_l.scale = Vec3::one() * 1.1;

            next.shoulder_r.position = Vec3::new(
                skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_r.orientation = Quaternion::rotation_x(short * -0.15 * walkintensity);
            next.shoulder_r.scale = Vec3::one() * 1.1;

            next.main.position = Vec3::new(-7.0, -6.5, 15.0);
            next.main.orientation = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
            next.main.scale = Vec3::one();

            next.second.scale = Vec3::one() * 0.0;

            next.lantern.position = Vec3::new(
                skeleton_attr.lantern.0,
                skeleton_attr.lantern.1,
                skeleton_attr.lantern.2,
            );
            next.lantern.orientation =
                Quaternion::rotation_x(shorte * 0.2 + 0.4) * Quaternion::rotation_y(shorte * 0.1);
            next.lantern.scale = Vec3::one() * 0.65;

            next.torso.position = Vec3::new(0.0, 0.0, 0.0) * skeleton_attr.scaler;
            next.torso.orientation = Quaternion::rotation_y(0.0);
            next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

            next.control.position = Vec3::new(0.0, 0.0, 0.0);
            next.control.orientation = Quaternion::rotation_x(0.0);
            next.control.scale = Vec3::one();

            next.control_l.scale = Vec3::one();

            next.control_r.scale = Vec3::one();
        } else {
            next.head.position = Vec3::new(
                0.0,
                -4.0 + skeleton_attr.head.0,
                -2.0 + skeleton_attr.head.1 + slow * 0.1 + breathe * -0.05,
            );
            next.head.orientation = Quaternion::rotation_z(head_look.x)
                * Quaternion::rotation_x(0.6 + head_look.y.abs());
            next.head.scale = Vec3::one() * skeleton_attr.head_scale + breathe * -0.05;

            next.chest.position = Vec3::new(
                0.0,
                skeleton_attr.chest.0,
                -3.0 + skeleton_attr.chest.1 + slow * 0.1,
            );
            next.chest.orientation = Quaternion::rotation_x(-0.7);
            next.chest.scale = Vec3::one() * 1.01 + breathe * 0.03;

            next.belt.position = Vec3::new(0.0, skeleton_attr.belt.0, skeleton_attr.belt.1);
            next.belt.orientation = Quaternion::rotation_z(0.3 + head_look.x * -0.1);
            next.belt.scale = Vec3::one() + breathe * -0.03;

            next.hand_l.position = Vec3::new(
                1.0 - skeleton_attr.hand.0,
                5.0 + skeleton_attr.hand.1,
                0.0 + skeleton_attr.hand.2,
            );
            next.hand_l.orientation = Quaternion::rotation_x(1.35);
            next.hand_l.scale = Vec3::one();

            next.hand_r.position = Vec3::new(
                -1.0 + skeleton_attr.hand.0,
                skeleton_attr.hand.1,
                skeleton_attr.hand.2,
            );
            next.hand_r.orientation = Quaternion::rotation_x(0.4);
            next.hand_r.scale = Vec3::one();

            next.glider.orientation = Quaternion::rotation_x(0.35);
            next.glider.position = Vec3::new(0.0, 0.0, 10.0);
            next.glider.scale = Vec3::one() * 0.0;

            next.back.position = Vec3::new(0.0, skeleton_attr.back.0, skeleton_attr.back.1);
            next.back.scale = Vec3::one() * 1.02;

            next.shorts.position = Vec3::new(0.0, skeleton_attr.shorts.0, skeleton_attr.shorts.1);
            next.shorts.orientation = Quaternion::rotation_z(0.6 + head_look.x * -0.2);
            next.shorts.scale = Vec3::one() + breathe * -0.03;

            next.foot_l.position = Vec3::new(
                -skeleton_attr.foot.0,
                -6.0 + skeleton_attr.foot.1,
                1.0 + skeleton_attr.foot.2,
            );
            next.foot_l.orientation = Quaternion::rotation_x(-0.5);
            next.foot_l.scale = Vec3::one();

            next.foot_r.position = Vec3::new(
                skeleton_attr.foot.0,
                4.0 + skeleton_attr.foot.1,
                skeleton_attr.foot.2,
            );
            next.foot_r.orientation = Quaternion::rotation_x(0.0);
            next.foot_r.scale = Vec3::one();

            next.shoulder_l.position = Vec3::new(
                -skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_l.scale = (Vec3::one() + breathe * -0.05) * 1.15;

            next.shoulder_r.position = Vec3::new(
                skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_r.scale = (Vec3::one() + breathe * -0.05) * 1.15;

            next.main.position = Vec3::new(-7.0, -5.0, 15.0);
            next.main.orientation = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
            next.main.scale = Vec3::one();

            next.second.position = Vec3::new(0.0, 0.0, 0.0);
            next.second.scale = Vec3::one() * 0.0;

            next.lantern.position = Vec3::new(
                skeleton_attr.lantern.0,
                skeleton_attr.lantern.1,
                skeleton_attr.lantern.2,
            );
            next.lantern.orientation = Quaternion::rotation_x(0.1) * Quaternion::rotation_y(0.1);
            next.lantern.scale = Vec3::one() * 0.65;

            next.torso.position = Vec3::new(0.0, 0.0, 0.0) * skeleton_attr.scaler;
            next.torso.orientation = Quaternion::rotation_x(0.0);
            next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

            next.control.scale = Vec3::one();

            next.control_l.scale = Vec3::one();

            next.control_r.scale = Vec3::one();
        }
        next
    }
}
