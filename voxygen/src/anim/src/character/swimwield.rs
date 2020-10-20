use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use std::{f32::consts::PI, ops::Mul};

pub struct SwimWieldAnimation;

impl Animation for SwimWieldAnimation {
    type Dependency = (Option<ToolKind>, Option<ToolKind>, f32, f64);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_swimwield\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_swimwield")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, velocity, global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        *rate = 1.0;
        let lab = 1.0;
        let speed = Vec3::<f32>::from(velocity).magnitude();
        *rate = 1.0;
        let intensity = if speed > 0.5 { 1.0 } else { 0.3 };
        let footrotl = (((1.0)
            / (0.2
                + (0.8)
                    * ((anim_time as f32 * 6.0 * lab as f32 + PI * 1.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 6.0 * lab as f32 + PI * 1.4).sin());

        let footrotr = (((1.0)
            / (0.2
                + (0.8)
                    * ((anim_time as f32 * 6.0 * lab as f32 + PI * 0.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 6.0 * lab as f32 + PI * 0.4).sin());

        let head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 3.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.2,
            ((global_time + anim_time) as f32 / 3.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.1,
        );

        let slowalt = (anim_time as f32 * 6.0 + PI).cos();
        let u_slow = (anim_time as f32 * 1.0 + PI).sin();
        let slow = (anim_time as f32 * 3.0 + PI).sin();
        let foothoril = (anim_time as f32 * 6.0 * lab as f32 + PI * 1.45).sin();
        let foothorir = (anim_time as f32 * 6.0 * lab as f32 + PI * (0.45)).sin();
        let u_slowalt = (anim_time as f32 * 3.0 + PI).cos();
        let short = (((5.0)
            / (1.5 + 3.5 * ((anim_time as f32 * lab as f32 * 16.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 16.0).sin());
        let noisea = (anim_time as f32 * 11.0 + PI / 6.0).sin();
        let noiseb = (anim_time as f32 * 19.0 + PI / 4.0).sin();

        next.foot_l.position = Vec3::new(
            -s_a.foot.0,
            s_a.foot.1 + foothoril * 1.5 * intensity,
            -10.0 + s_a.foot.2 + footrotl * 3.0 * intensity,
        );
        next.foot_l.orientation = Quaternion::rotation_x(-0.8 + footrotl * 0.4 * intensity);

        next.foot_r.position = Vec3::new(
            s_a.foot.0,
            s_a.foot.1 + foothorir * 1.5 * intensity,
            -10.0 + s_a.foot.2 + footrotr * 3.0 * intensity,
        );
        next.foot_r.orientation = Quaternion::rotation_x(-0.8 + footrotr * 0.4 * intensity);

        next.hold.scale = Vec3::one() * 0.0;

        if velocity > 0.01 {
            next.torso.position = Vec3::new(0.0, 0.0, 1.0) * s_a.scaler;
            next.torso.orientation = Quaternion::rotation_x(velocity * -0.05);
            next.torso.scale = Vec3::one() / 11.0 * s_a.scaler;

            next.back.position = Vec3::new(0.0, s_a.back.0, s_a.back.1);
            next.back.orientation = Quaternion::rotation_x(
                (-0.5 + short * 0.3 + noisea * 0.3 + noiseb * 0.3).min(-0.1),
            );
            next.back.scale = Vec3::one() * 1.02;
        } else {
            next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1 + u_slow * 0.1);
            next.head.orientation =
                Quaternion::rotation_z(head_look.x) * Quaternion::rotation_x(head_look.y.abs());
            next.head.scale = Vec3::one() * s_a.head_scale;

            next.chest.position =
                Vec3::new(0.0 + slowalt * 0.5, s_a.chest.0, s_a.chest.1 + u_slow * 0.5);
            next.torso.position = Vec3::new(0.0, 0.0, 0.0) * s_a.scaler;
            next.torso.scale = Vec3::one() / 11.0 * s_a.scaler;

            next.foot_l.position = Vec3::new(-s_a.foot.0, -2.0 + s_a.foot.1, s_a.foot.2);

            next.foot_r.position = Vec3::new(s_a.foot.0, 2.0 + s_a.foot.1, s_a.foot.2);

            next.chest.orientation =
                Quaternion::rotation_y(u_slowalt * 0.04) * Quaternion::rotation_z(0.25);

            next.belt.position = Vec3::new(0.0, s_a.belt.0, s_a.belt.1);
            next.belt.orientation =
                Quaternion::rotation_y(u_slowalt * 0.03) * Quaternion::rotation_z(0.22);
            next.belt.scale = Vec3::one() * 1.02;

            next.back.position = Vec3::new(0.0, s_a.back.0, s_a.back.1);
            next.back.orientation = Quaternion::rotation_x(-0.2);
            next.back.scale = Vec3::one() * 1.02;
            next.shorts.position = Vec3::new(0.0, s_a.shorts.0, s_a.shorts.1);
            next.shorts.orientation = Quaternion::rotation_z(0.3);
        }
        match active_tool_kind {
            Some(ToolKind::Sword(_)) => {
                next.hand_l.position = Vec3::new(-0.75, -1.0, -2.5);
                next.hand_l.orientation =
                    Quaternion::rotation_x(1.47) * Quaternion::rotation_y(-0.2);
                next.hand_l.scale = Vec3::one() * 1.04;
                next.hand_r.position = Vec3::new(0.75, -1.5, -5.5);
                next.hand_r.orientation =
                    Quaternion::rotation_x(1.47) * Quaternion::rotation_y(0.3);
                next.hand_r.scale = Vec3::one() * 1.05;
                next.main.position = Vec3::new(0.0, 0.0, -3.0);
                next.main.orientation = Quaternion::rotation_x(-0.1)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);

                next.control.position = Vec3::new(-7.0, 6.0, 6.0);
                next.control.orientation = Quaternion::rotation_x(u_slow * 0.15)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(u_slowalt * 0.08);
                next.control.scale = Vec3::one();
            },
            Some(ToolKind::Dagger(_)) => {
                let hand_scale = 1.12;

                next.control.position = Vec3::new(0.0, 0.0, 0.0);

                next.hand_l.position = Vec3::new(0.0, 0.0, 0.0);
                next.hand_l.orientation = Quaternion::rotation_x(0.0 * PI)
                    * Quaternion::rotation_y(0.0 * PI)
                    * Quaternion::rotation_z(0.0 * PI);
                next.hand_l.scale = Vec3::one() * hand_scale;

                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0 * PI)
                    * Quaternion::rotation_y(0.0 * PI)
                    * Quaternion::rotation_z(0.0 * PI);

                next.control_l.position = Vec3::new(-7.0, 0.0, 0.0);

                next.hand_r.position = Vec3::new(0.0, 0.0, 0.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.0 * PI)
                    * Quaternion::rotation_y(0.0 * PI)
                    * Quaternion::rotation_z(0.0 * PI);
                next.hand_r.scale = Vec3::one() * hand_scale;

                next.second.position = Vec3::new(0.0, 0.0, 0.0);
                next.second.orientation = Quaternion::rotation_x(0.0 * PI)
                    * Quaternion::rotation_y(0.0 * PI)
                    * Quaternion::rotation_z(0.0 * PI);
                next.second.scale = Vec3::one();

                next.control_r.position = Vec3::new(7.0, 0.0, 0.0);
            },
            Some(ToolKind::Axe(_)) => {
                if velocity < 0.5 {
                    next.head.position =
                        Vec3::new(0.0, -3.5 + s_a.head.0, s_a.head.1 + u_slow * 0.1);
                    next.head.orientation = Quaternion::rotation_z(head_look.x)
                        * Quaternion::rotation_x(0.35 + head_look.y.abs());
                    next.head.scale = Vec3::one() * s_a.head_scale;
                    next.chest.orientation = Quaternion::rotation_x(-0.35)
                        * Quaternion::rotation_y(u_slowalt * 0.04)
                        * Quaternion::rotation_z(0.15);
                    next.belt.position = Vec3::new(0.0, 1.0 + s_a.belt.0, s_a.belt.1);
                    next.belt.orientation = Quaternion::rotation_x(0.15)
                        * Quaternion::rotation_y(u_slowalt * 0.03)
                        * Quaternion::rotation_z(0.15);
                    next.shorts.position = Vec3::new(0.0, 1.0 + s_a.shorts.0, s_a.shorts.1);
                    next.shorts.orientation =
                        Quaternion::rotation_x(0.15) * Quaternion::rotation_z(0.25);
                    next.control.orientation = Quaternion::rotation_x(1.8)
                        * Quaternion::rotation_y(-0.5)
                        * Quaternion::rotation_z(PI - 0.2);
                    next.control.scale = Vec3::one();
                } else {
                    next.control.orientation = Quaternion::rotation_x(2.1)
                        * Quaternion::rotation_y(-0.4)
                        * Quaternion::rotation_z(PI - 0.2);
                    next.control.scale = Vec3::one();
                }
                next.hand_l.position = Vec3::new(-0.5, 0.0, 4.0);
                next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
                    * Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_y(0.0);
                next.hand_l.scale = Vec3::one() * 1.08;
                next.hand_r.position = Vec3::new(0.5, 0.0, -2.5);
                next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
                    * Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_y(0.0);
                next.hand_r.scale = Vec3::one() * 1.06;
                next.main.position = Vec3::new(-0.0, -2.0, -1.0);
                next.main.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);

                next.control.position = Vec3::new(-3.0, 11.0, 3.0);
            },
            Some(ToolKind::Hammer(_)) => {
                next.hand_l.position = Vec3::new(-12.0, 0.0, 0.0);
                next.hand_l.orientation =
                    Quaternion::rotation_x(-0.0) * Quaternion::rotation_y(0.0);
                next.hand_l.scale = Vec3::one() * 1.08;
                next.hand_r.position = Vec3::new(2.0, 0.0, 0.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.0) * Quaternion::rotation_y(0.0);
                next.hand_r.scale = Vec3::one() * 1.06;
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(-1.57)
                    * Quaternion::rotation_z(1.57);

                next.control.position = Vec3::new(6.0, 7.0, 1.0);
                next.control.orientation = Quaternion::rotation_x(0.3 + u_slow * 0.15)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(u_slowalt * 0.08);
                next.control.scale = Vec3::one();
            },
            Some(ToolKind::Staff(_)) | Some(ToolKind::Sceptre(_)) => {
                next.hand_l.position = Vec3::new(1.5, 0.5, -4.0);
                next.hand_l.orientation =
                    Quaternion::rotation_x(1.47) * Quaternion::rotation_y(-0.3);
                next.hand_l.scale = Vec3::one() * 1.05;
                next.hand_r.position = Vec3::new(8.0, 4.0, 2.0);
                next.hand_r.orientation = Quaternion::rotation_x(1.8)
                    * Quaternion::rotation_y(0.5)
                    * Quaternion::rotation_z(-0.27);
                next.hand_r.scale = Vec3::one() * 1.05;
                next.main.position = Vec3::new(12.0, 8.4, 13.2);
                next.main.orientation = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(3.14 + 0.3)
                    * Quaternion::rotation_z(0.9);

                next.control.position = Vec3::new(-14.0, 1.8, 3.0);
                next.control.orientation = Quaternion::rotation_x(u_slow * 0.2)
                    * Quaternion::rotation_y(-0.2)
                    * Quaternion::rotation_z(u_slowalt * 0.1);
                next.control.scale = Vec3::one();
            },
            Some(ToolKind::Shield(_)) => {
                let hand_scale = 1.12;

                next.control.position = Vec3::new(0.0, 0.0, 0.0);

                next.hand_l.position = Vec3::new(0.0, 0.0, 0.0);
                next.hand_l.orientation = Quaternion::rotation_x(0.0 * PI)
                    * Quaternion::rotation_y(0.0 * PI)
                    * Quaternion::rotation_z(0.0 * PI);
                next.hand_l.scale = Vec3::one() * hand_scale;

                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0 * PI)
                    * Quaternion::rotation_y(0.0 * PI)
                    * Quaternion::rotation_z(0.0 * PI);

                next.control_l.position = Vec3::new(-7.0, 0.0, 0.0);

                next.hand_r.position = Vec3::new(0.0, 0.0, 0.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.0 * PI)
                    * Quaternion::rotation_y(0.0 * PI)
                    * Quaternion::rotation_z(0.0 * PI);
                next.hand_r.scale = Vec3::one() * hand_scale;

                next.second.position = Vec3::new(0.0, 0.0, 0.0);
                next.second.orientation = Quaternion::rotation_x(0.0 * PI)
                    * Quaternion::rotation_y(0.0 * PI)
                    * Quaternion::rotation_z(0.0 * PI);
                next.second.scale = Vec3::one();

                next.control_r.position = Vec3::new(7.0, 0.0, 0.0);
            },
            Some(ToolKind::Bow(_)) => {
                next.hand_l.position = Vec3::new(2.0, 1.5, 0.0);
                next.hand_l.orientation = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6)
                    * Quaternion::rotation_z(-0.3);
                next.hand_l.scale = Vec3::one() * 1.05;
                next.hand_r.position = Vec3::new(5.9, 4.5, -5.0);
                next.hand_r.orientation = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6)
                    * Quaternion::rotation_z(-0.3);
                next.hand_r.scale = Vec3::one() * 1.05;
                next.main.position = Vec3::new(3.0, 2.0, -13.0);
                next.main.orientation = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.3)
                    * Quaternion::rotation_z(-0.6);

                next.hold.position = Vec3::new(1.2, -1.0, -5.2);
                next.hold.orientation = Quaternion::rotation_x(-1.7)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(-0.1);
                next.hold.scale = Vec3::one() * 1.0;

                next.control.position = Vec3::new(-7.0, 6.0, 6.0);
                next.control.orientation =
                    Quaternion::rotation_x(u_slow * 0.2) * Quaternion::rotation_z(u_slowalt * 0.1);
                next.control.scale = Vec3::one();
            },
            Some(ToolKind::Debug(_)) => {
                next.hand_l.position = Vec3::new(-7.0, 4.0, 3.0);
                next.hand_l.orientation = Quaternion::rotation_x(1.27)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.hand_l.scale = Vec3::one() * 1.01;
                next.hand_r.position = Vec3::new(7.0, 2.5, -1.25);
                next.hand_r.orientation = Quaternion::rotation_x(1.27)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(-0.3);
                next.hand_r.scale = Vec3::one() * 1.01;
                next.main.position = Vec3::new(5.0, 8.75, -2.0);
                next.main.orientation = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(-1.27)
                    * Quaternion::rotation_z(0.0);
                next.main.scale = Vec3::one();
                next.control.position = Vec3::new(0.0, 6.0, 6.0);
                next.control.orientation =
                    Quaternion::rotation_x(u_slow * 0.2) * Quaternion::rotation_z(u_slowalt * 0.1);
                next.control.scale = Vec3::one();
            },
            Some(ToolKind::Farming(_)) => {
                if velocity < 0.5 {
                    next.head.orientation = Quaternion::rotation_z(head_look.x)
                        * Quaternion::rotation_x(-0.2 + head_look.y.abs());
                    next.head.scale = Vec3::one() * s_a.head_scale;
                }
                next.hand_l.position = Vec3::new(9.0, 1.0, 1.0);
                next.hand_l.orientation =
                    Quaternion::rotation_x(1.57) * Quaternion::rotation_y(0.0);
                next.hand_l.scale = Vec3::one() * 1.05;
                next.hand_r.position = Vec3::new(9.0, 1.0, 11.0);
                next.hand_r.orientation = Quaternion::rotation_x(1.57)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.hand_r.scale = Vec3::one() * 1.05;
                next.main.position = Vec3::new(7.5, 7.5, 13.2);
                next.main.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(3.14)
                    * Quaternion::rotation_z(0.0);

                next.control.position = Vec3::new(-11.0 + slow * 2.0, 1.8, 4.0);
                next.control.orientation = Quaternion::rotation_x(u_slow * 0.1)
                    * Quaternion::rotation_y(0.6 + u_slow * 0.1)
                    * Quaternion::rotation_z(u_slowalt * 0.1);
                next.control.scale = Vec3::one();
            },
            _ => {},
        }

        next.control_l.scale = Vec3::one();

        next.control_r.scale = Vec3::one();

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
