use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::{Hands, ToolKind};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct GlideWieldAnimation;

type GlideWieldAnimationDependency = (
    Option<ToolKind>,
    Option<ToolKind>,
    Vec3<f32>,
    Vec3<f32>,
    Vec3<f32>,
    f64,
);

impl Animation for GlideWieldAnimation {
    type Dependency = GlideWieldAnimationDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_glidewield\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_glidewield")]
    #[allow(clippy::useless_conversion)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, velocity, orientation, last_ori, global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();
        *rate = 1.0;
        let slow = (anim_time as f32 * 1.0).sin();
        let breathe = ((anim_time as f32 * 0.5).sin()).abs();
        let walkintensity = if speed > 5.0 { 1.0 } else { 0.45 };
        let walk = if speed > 5.0 { 1.0 } else { 0.5 };
        let lower = if speed > 5.0 { 0.0 } else { 1.0 };
        let _snapfoot = if speed > 5.0 { 1.1 } else { 2.0 };
        let lab = 1.0;
        let foothoril = (anim_time as f32 * 16.0 * walk * lab as f32 + PI * 1.45).sin();
        let foothorir = (anim_time as f32 * 16.0 * walk * lab as f32 + PI * (0.45)).sin();

        let footvertl = (anim_time as f32 * 16.0 * walk * lab as f32).sin();
        let footvertr = (anim_time as f32 * 16.0 * walk * lab as f32 + PI).sin();

        let footrotl = (((5.0)
            / (2.5
                + (2.5)
                    * ((anim_time as f32 * 16.0 * walk * lab as f32 + PI * 1.4).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * walk * lab as f32 + PI * 1.4).sin());

        let footrotr = (((5.0)
            / (1.0
                + (4.0)
                    * ((anim_time as f32 * 16.0 * walk * lab as f32 + PI * 0.4).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * walk * lab as f32 + PI * 0.4).sin());

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

        let shortalt = (anim_time as f32 * lab as f32 * 16.0 * walk + PI / 2.0).sin();
        let shortalter = (anim_time as f32 * lab as f32 * 16.0 * walk + PI / -2.0).sin();

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
            ori.angle_between(last_ori).min(0.2)
                * last_ori.determine_side(Vec2::zero(), ori).signum()
        } else {
            0.0
        } * 1.3;

        next.l_hand.offset = Vec3::new(
            -2.0 - skeleton_attr.hand.0,
            skeleton_attr.hand.1,
            skeleton_attr.hand.2 + 15.0,
        );
        next.l_hand.ori = Quaternion::rotation_x(3.35);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(
            2.0 + skeleton_attr.hand.0,
            skeleton_attr.hand.1,
            skeleton_attr.hand.2 + 15.0,
        );
        next.r_hand.ori = Quaternion::rotation_x(3.35);
        next.r_hand.scale = Vec3::one();

        if speed > 0.5 {
            next.head.offset = Vec3::new(
                0.0,
                -3.0 + skeleton_attr.head.0,
                skeleton_attr.head.1 + short * 0.1,
            );
            next.head.ori = Quaternion::rotation_z(tilt * -2.5 + head_look.x * 0.2 - short * 0.1)
                * Quaternion::rotation_x(head_look.y + 0.45 - lower * 0.35);
            next.head.scale = Vec3::one() * skeleton_attr.head_scale;

            next.chest.offset = Vec3::new(
                0.0,
                skeleton_attr.chest.0,
                skeleton_attr.chest.1 + 2.0 + shortalt * -1.5 - lower,
            );
            next.chest.ori = Quaternion::rotation_z(short * 0.10 * walkintensity + tilt * -1.0)
                * Quaternion::rotation_y(tilt * 2.2)
                * Quaternion::rotation_x(
                    shortalter * 0.035 + wave_stop * speed * -0.1 + (tilt.abs()),
                );
            next.chest.scale = Vec3::one();

            next.belt.offset = Vec3::new(0.0, skeleton_attr.belt.0, skeleton_attr.belt.1);
            next.belt.ori = Quaternion::rotation_z(short * 0.1 + tilt * -1.1)
                * Quaternion::rotation_y(tilt * 0.5);
            next.belt.scale = Vec3::one();

            next.glider.ori = Quaternion::rotation_x(0.8);
            next.glider.offset = Vec3::new(0.0, -10.0, 15.0);
            next.glider.scale = Vec3::one() * 1.0;

            next.back.offset = Vec3::new(0.0, skeleton_attr.back.0, skeleton_attr.back.1);
            next.back.ori =
                Quaternion::rotation_x(-0.25 + short * 0.1 + noisea * 0.1 + noiseb * 0.1);
            next.back.scale = Vec3::one() * 1.02;

            next.shorts.offset = Vec3::new(0.0, skeleton_attr.shorts.0, skeleton_attr.shorts.1);
            next.shorts.ori = Quaternion::rotation_z(short * 0.25 + tilt * -1.5)
                * Quaternion::rotation_y(tilt * 0.7);
            next.shorts.scale = Vec3::one();

            next.l_foot.offset = Vec3::new(
                -skeleton_attr.foot.0,
                -1.5 + skeleton_attr.foot.1 + foothoril * -8.5 * walkintensity - lower * 1.0,
                2.0 + skeleton_attr.foot.2 + ((footvertl * -2.7).max(-1.0)) * walkintensity,
            );
            next.l_foot.ori = Quaternion::rotation_x(-0.2 + footrotl * -1.2 * walkintensity)
                * Quaternion::rotation_y(tilt * 1.8);
            next.l_foot.scale = Vec3::one();

            next.r_foot.offset = Vec3::new(
                skeleton_attr.foot.0,
                -1.5 + skeleton_attr.foot.1 + foothorir * -8.5 * walkintensity - lower * 1.0,
                2.0 + skeleton_attr.foot.2 + ((footvertr * -2.7).max(-1.0)) * walkintensity,
            );
            next.r_foot.ori = Quaternion::rotation_x(-0.2 + footrotr * -1.2 * walkintensity)
                * Quaternion::rotation_y(tilt * 1.8);
            next.r_foot.scale = Vec3::one();

            next.l_shoulder.offset = Vec3::new(
                -skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1,
                skeleton_attr.shoulder.2,
            );
            next.l_shoulder.ori = Quaternion::rotation_x(short * 0.15 * walkintensity);
            next.l_shoulder.scale = Vec3::one() * 1.1;

            next.r_shoulder.offset = Vec3::new(
                skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1,
                skeleton_attr.shoulder.2,
            );
            next.r_shoulder.ori = Quaternion::rotation_x(short * -0.15 * walkintensity);
            next.r_shoulder.scale = Vec3::one() * 1.1;

            match active_tool_kind {
                Some(ToolKind::Dagger(_)) => {
                    next.main.offset = Vec3::new(-4.0, -5.0, 7.0);
                    next.main.ori =
                        Quaternion::rotation_y(0.25 * PI) * Quaternion::rotation_z(1.5 * PI);
                },
                Some(ToolKind::Shield(_)) => {
                    next.main.offset = Vec3::new(-0.0, -5.0, 3.0);
                    next.main.ori =
                        Quaternion::rotation_y(0.25 * PI) * Quaternion::rotation_z(-1.5 * PI);
                },
                _ => {
                    next.main.offset = Vec3::new(-7.0, -5.0, 15.0);
                    next.main.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
                },
            }
            next.main.scale = Vec3::one();

            match second_tool_kind {
                Some(ToolKind::Dagger(_)) => {
                    next.second.offset = Vec3::new(4.0, -6.0, 7.0);
                    next.second.ori =
                        Quaternion::rotation_y(-0.25 * PI) * Quaternion::rotation_z(-1.5 * PI);
                },
                Some(ToolKind::Shield(_)) => {
                    next.second.offset = Vec3::new(0.0, -4.0, 3.0);
                    next.second.ori =
                        Quaternion::rotation_y(-0.25 * PI) * Quaternion::rotation_z(1.5 * PI);
                },
                _ => {
                    next.second.offset = Vec3::new(-7.0, -5.0, 15.0);
                    next.second.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
                },
            }
            next.second.scale = Vec3::one();

            next.lantern.offset = Vec3::new(
                skeleton_attr.lantern.0,
                skeleton_attr.lantern.1,
                skeleton_attr.lantern.2,
            );
            next.lantern.ori =
                Quaternion::rotation_x(shorte * 0.7 + 0.4) * Quaternion::rotation_y(shorte * 0.4);
            next.lantern.scale = Vec3::one() * 0.65;

            next.torso.offset = Vec3::new(0.0, 0.0, 0.0) * skeleton_attr.scaler;
            next.torso.ori = Quaternion::rotation_y(0.0);
            next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

            next.control.offset = Vec3::new(0.0, 0.0, 0.0);
            next.control.ori = Quaternion::rotation_x(0.0);
            next.control.scale = Vec3::one();

            next.l_control.scale = Vec3::one();

            next.r_control.scale = Vec3::one();
        } else {
            next.head.offset = Vec3::new(
                0.0,
                -3.0 + skeleton_attr.head.0,
                skeleton_attr.head.1 + slow * 0.3 + breathe * -0.05,
            );
            next.head.ori =
                Quaternion::rotation_z(head_look.x) * Quaternion::rotation_x(head_look.y.abs());
            next.head.scale = Vec3::one() * skeleton_attr.head_scale + breathe * -0.05;

            next.chest.offset = Vec3::new(
                0.0,
                skeleton_attr.chest.0,
                skeleton_attr.chest.1 + slow * 0.3,
            );
            next.chest.ori = Quaternion::rotation_z(head_look.x * 0.6);
            next.chest.scale = Vec3::one() * 1.01 + breathe * 0.03;

            next.belt.offset = Vec3::new(0.0, skeleton_attr.belt.0, skeleton_attr.belt.1);
            next.belt.ori = Quaternion::rotation_z(head_look.x * -0.1);
            next.belt.scale = Vec3::one() + breathe * -0.03;

            next.glider.ori = Quaternion::rotation_x(0.35);
            next.glider.offset = Vec3::new(0.0, -9.0, 17.0);
            next.glider.scale = Vec3::one() * 1.0;

            next.back.offset = Vec3::new(0.0, skeleton_attr.back.0, skeleton_attr.back.1);
            next.back.scale = Vec3::one() * 1.02;

            next.shorts.offset = Vec3::new(0.0, skeleton_attr.shorts.0, skeleton_attr.shorts.1);
            next.shorts.ori = Quaternion::rotation_z(head_look.x * -0.2);
            next.shorts.scale = Vec3::one() + breathe * -0.03;

            next.l_foot.offset = Vec3::new(
                -skeleton_attr.foot.0,
                skeleton_attr.foot.1,
                skeleton_attr.foot.2,
            );
            next.l_foot.scale = Vec3::one();

            next.r_foot.offset = Vec3::new(
                skeleton_attr.foot.0,
                skeleton_attr.foot.1,
                skeleton_attr.foot.2,
            );
            next.r_foot.scale = Vec3::one();

            next.l_shoulder.offset = Vec3::new(
                -skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1,
                skeleton_attr.shoulder.2,
            );
            next.l_shoulder.scale = (Vec3::one() + breathe * -0.05) * 1.15;

            next.r_shoulder.offset = Vec3::new(
                skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1,
                skeleton_attr.shoulder.2,
            );
            next.r_shoulder.scale = (Vec3::one() + breathe * -0.05) * 1.15;

            match active_tool_kind {
                Some(ToolKind::Dagger(_)) => {
                    next.main.offset = Vec3::new(-4.0, -5.0, 7.0);
                    next.main.ori =
                        Quaternion::rotation_y(0.25 * PI) * Quaternion::rotation_z(1.5 * PI);
                },
                Some(ToolKind::Shield(_)) => {
                    next.main.offset = Vec3::new(-0.0, -5.0, 3.0);
                    next.main.ori =
                        Quaternion::rotation_y(0.25 * PI) * Quaternion::rotation_z(-1.5 * PI);
                },
                _ => {
                    next.main.offset = Vec3::new(-7.0, -5.0, 15.0);
                    next.main.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
                },
            }
            next.main.scale = Vec3::one();

            match second_tool_kind {
                Some(ToolKind::Dagger(_)) => {
                    next.second.offset = Vec3::new(4.0, -6.0, 7.0);
                    next.second.ori =
                        Quaternion::rotation_y(-0.25 * PI) * Quaternion::rotation_z(-1.5 * PI);
                },
                Some(ToolKind::Shield(_)) => {
                    next.second.offset = Vec3::new(0.0, -4.0, 3.0);
                    next.second.ori =
                        Quaternion::rotation_y(-0.25 * PI) * Quaternion::rotation_z(1.5 * PI);
                },
                _ => {
                    next.second.offset = Vec3::new(-7.0, -5.0, 15.0);
                    next.second.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
                },
            }
            next.second.scale = Vec3::one();

            next.lantern.offset = Vec3::new(
                skeleton_attr.lantern.0,
                skeleton_attr.lantern.1,
                skeleton_attr.lantern.2,
            );
            next.lantern.ori = Quaternion::rotation_x(0.1) * Quaternion::rotation_y(0.1);
            next.lantern.scale = Vec3::one() * 0.65;

            next.torso.offset = Vec3::new(0.0, 0.0, 0.0) * skeleton_attr.scaler;
            next.torso.ori = Quaternion::rotation_x(0.0);
            next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

            next.control.scale = Vec3::one();

            next.l_control.scale = Vec3::one();

            next.r_control.scale = Vec3::one();
        }

        next.second.scale = match (
            active_tool_kind.map(|tk| tk.into_hands()),
            second_tool_kind.map(|tk| tk.into_hands()),
        ) {
            (Some(Hands::OneHand), Some(Hands::OneHand)) => Vec3::one(),
            (_, _) => Vec3::zero(),
        };

        next
    }
}
