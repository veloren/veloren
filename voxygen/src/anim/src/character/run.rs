use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use std::{f32::consts::PI, ops::Mul};

pub struct RunAnimation;

type RunAnimationDependency = (
    Option<ToolKind>,
    Option<ToolKind>,
    Vec3<f32>,
    Vec3<f32>,
    Vec3<f32>,
    f64,
    Vec3<f32>,
);

impl Animation for RunAnimation {
    type Dependency = RunAnimationDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_run")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, velocity, orientation, last_ori, global_time, avg_vel): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let speed = Vec2::<f32>::from(velocity).magnitude();
        *rate = 1.0;
        let impact = (avg_vel.z).max(-8.0);

        let walkintensity = if speed > 5.0 { 1.0 } else { 0.45 };
        let walk = if speed > 5.0 { 1.0 } else { 0.5 };
        let lower = if speed > 5.0 { 0.0 } else { 1.0 };
        let _snapfoot = if speed > 5.0 { 1.1 } else { 2.0 };
        let lab = 1.0;
        let foothoril = (anim_time as f32 * 16.0 * walk * lab as f32 + PI * 1.45).sin();
        let foothorir = (anim_time as f32 * 16.0 * walk * lab as f32 + PI * (0.45)).sin();

        let footvertl = (anim_time as f32 * 16.0 * walk * lab as f32).sin();
        let footvertr = (anim_time as f32 * 16.0 * walk * lab as f32 + PI).sin();

        let footrotl = (((1.0)
            / (0.5
                + (0.5)
                    * ((anim_time as f32 * 16.0 * walk * lab as f32 + PI * 1.4).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * walk * lab as f32 + PI * 1.4).sin());

        let footrotr = (((1.0)
            / (0.5
                + (0.5)
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

        let ori: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if ::vek::Vec2::new(ori, last_ori)
            .map(|o| o.magnitude_squared())
            .map(|m| m > 0.001 && m.is_finite())
            .reduce_and()
            && ori.angle_between(last_ori).is_finite()
        {
            ori.angle_between(last_ori).min(0.2)
                * last_ori.determine_side(Vec2::zero(), ori).signum()
        } else {
            0.0
        } * 1.3;

        next.head.position = Vec3::new(0.0, -1.0 + s_a.head.0, s_a.head.1 + short * 0.1);
        next.head.orientation =
            Quaternion::rotation_z(tilt * -2.5 + head_look.x * 0.2 - short * 0.1)
                * Quaternion::rotation_x(head_look.y + 0.45 - lower * 0.35);
        next.head.scale = Vec3::one() * s_a.head_scale;

        next.chest.position = Vec3::new(
            0.0,
            s_a.chest.0,
            s_a.chest.1 + 2.0 + shortalt * -1.5 - lower,
        );
        next.chest.orientation = Quaternion::rotation_z(short * 0.18 * walkintensity + tilt * -0.6)
            * Quaternion::rotation_y(tilt * 1.6)
            * Quaternion::rotation_x(
                impact * 0.06 + shortalter * 0.035 + wave_stop * speed * -0.09 + (tilt.abs()),
            );

        next.belt.position = Vec3::new(0.0, 0.25 + s_a.belt.0, 0.25 + s_a.belt.1);
        next.belt.orientation = Quaternion::rotation_x(0.1)
            * Quaternion::rotation_z(short * 0.1 + tilt * -1.1)
            * Quaternion::rotation_y(tilt * 0.5);

        next.back.position = Vec3::new(0.0, s_a.back.0, s_a.back.1);
        next.back.orientation =
            Quaternion::rotation_x(-0.25 + short * 0.1 + noisea * 0.1 + noiseb * 0.1);

        next.shorts.position = Vec3::new(0.0, 0.65 + s_a.shorts.0, 0.65 + s_a.shorts.1);
        next.shorts.orientation = Quaternion::rotation_x(0.2)
            * Quaternion::rotation_z(short * 0.25 + tilt * -1.5)
            * Quaternion::rotation_y(tilt * 0.7);

        next.hand_l.position = Vec3::new(
            -s_a.hand.0 + foothorir * -1.3,
            3.0 + s_a.hand.1 + foothorir * -7.0 * walkintensity,
            1.5 + s_a.hand.2 - foothorir * 5.5 * walkintensity,
        );
        next.hand_l.orientation = Quaternion::rotation_x(0.6 + footrotr * -1.2 * walkintensity)
            * Quaternion::rotation_y(footrotr * 0.4 * walkintensity);

        next.hand_r.position = Vec3::new(
            s_a.hand.0 + foothoril * 1.3,
            3.0 + s_a.hand.1 + foothoril * -6.5 * walkintensity,
            1.5 + s_a.hand.2 - foothoril * 7.0 * walkintensity,
        );
        next.hand_r.orientation = Quaternion::rotation_x(0.6 + footrotl * -1.2 * walkintensity)
            * Quaternion::rotation_y(footrotl * -0.4 * walkintensity);

        next.foot_l.position = Vec3::new(
            -s_a.foot.0,
            -1.5 + s_a.foot.1 + foothoril * -8.5 * walkintensity - lower * 1.0,
            2.0 + s_a.foot.2 + ((footvertl * -2.7).max(-1.0)) * walkintensity,
        );
        next.foot_l.orientation = Quaternion::rotation_x(-0.2 + footrotl * -1.2 * walkintensity)
            * Quaternion::rotation_y(tilt * 1.8);

        next.foot_r.position = Vec3::new(
            s_a.foot.0,
            -1.5 + s_a.foot.1 + foothorir * -8.5 * walkintensity - lower * 1.0,
            2.0 + s_a.foot.2 + ((footvertr * -2.7).max(-1.0)) * walkintensity,
        );
        next.foot_r.orientation = Quaternion::rotation_x(-0.2 + footrotr * -1.2 * walkintensity)
            * Quaternion::rotation_y(tilt * 1.8);

        next.shoulder_l.position = Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_l.orientation = Quaternion::rotation_x(short * 0.15 * walkintensity);
        next.shoulder_l.scale = Vec3::one() * 1.1;

        next.shoulder_r.position = Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_r.orientation = Quaternion::rotation_x(short * -0.15 * walkintensity);
        next.shoulder_r.scale = Vec3::one() * 1.1;

        next.glider.position = Vec3::new(0.0, 0.0, 10.0);
        next.glider.scale = Vec3::one() * 0.0;

        match active_tool_kind {
            Some(ToolKind::Dagger(_)) => {
                next.main.position = Vec3::new(-4.0, -5.0, 7.0);
                next.main.orientation =
                    Quaternion::rotation_y(0.25 * PI) * Quaternion::rotation_z(1.5 * PI);
            },
            Some(ToolKind::Shield(_)) => {
                next.main.position = Vec3::new(-0.0, -5.0, 3.0);
                next.main.orientation =
                    Quaternion::rotation_y(0.25 * PI) * Quaternion::rotation_z(-1.5 * PI);
            },
            Some(ToolKind::Staff(_)) | Some(ToolKind::Sceptre(_)) => {
                next.main.position = Vec3::new(2.0, -5.0, -1.0);
                next.main.orientation = Quaternion::rotation_y(-0.5) * Quaternion::rotation_z(1.57);
            },
            _ => {
                next.main.position = Vec3::new(-7.0, -5.0, 15.0);
                next.main.orientation =
                    Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57 + shorte * -0.2);
            },
        }

        match second_tool_kind {
            Some(ToolKind::Dagger(_)) => {
                next.second.position = Vec3::new(4.0, -6.0, 7.0);
                next.second.orientation =
                    Quaternion::rotation_y(-0.25 * PI) * Quaternion::rotation_z(-1.5 * PI);
            },
            Some(ToolKind::Shield(_)) => {
                next.second.position = Vec3::new(0.0, -4.0, 3.0);
                next.second.orientation =
                    Quaternion::rotation_y(-0.25 * PI) * Quaternion::rotation_z(1.5 * PI);
            },
            _ => {
                next.second.position = Vec3::new(-7.0, -5.0, 15.0);
                next.second.orientation =
                    Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
            },
        }

        next.lantern.position = Vec3::new(s_a.lantern.0, s_a.lantern.1, s_a.lantern.2);
        next.lantern.orientation =
            Quaternion::rotation_x(shorte * 0.7 + 0.4) * Quaternion::rotation_y(shorte * 0.4);
        next.lantern.scale = Vec3::one() * 0.65;
        next.hold.scale = Vec3::one() * 0.0;

        next.torso.position = Vec3::new(0.0, -0.3, 0.0) * s_a.scaler;
        next.torso.scale = Vec3::one() / 11.0 * s_a.scaler;

        next.second.scale = match (
            active_tool_kind.map(|tk| tk.hands()),
            second_tool_kind.map(|tk| tk.hands()),
        ) {
            (Some(Hands::OneHand), Some(Hands::OneHand)) => Vec3::one(),
            (_, _) => Vec3::zero(),
        };

        next
    }
}
