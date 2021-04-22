use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    util::Dir,
};
use std::{f32::consts::PI, ops::Mul};

pub struct WieldAnimation;

type WieldAnimationDependency = (
    Option<ToolKind>,
    Option<ToolKind>,
    (Option<Hands>, Option<Hands>),
    Vec3<f32>,
    Vec3<f32>,
    Dir,
    Vec3<f32>,
    f32,
);
impl Animation for WieldAnimation {
    type Dependency = WieldAnimationDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_wield\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_wield")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            active_tool_kind,
            second_tool_kind,
            hands,
            orientation,
            last_ori,
            look_dir,
            velocity,
            global_time,
        ): Self::Dependency,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let lab: f32 = 1.0;
        let speed = Vec2::<f32>::from(velocity).magnitude();
        let speednorm = speed / 9.5;
        let mut next = (*skeleton).clone();
        let head_look = Vec2::new(
            (global_time + anim_time / 3.0).floor().mul(7331.0).sin() * 0.2,
            (global_time + anim_time / 3.0).floor().mul(1337.0).sin() * 0.1,
        );

        let foothoril = (anim_time * 16.0 * lab + PI * 1.45).sin();

        let beltstatic = (anim_time * 10.0 * lab + PI / 2.0).sin();
        let footvertlstatic = (anim_time * 10.0 * lab).sin();
        let footvertrstatic = (anim_time * 10.0 * lab + PI).sin();
        let footrotl =
            ((1.0 / (0.5 + (0.5) * ((anim_time * 16.0 * lab + PI * 1.4).sin()).powi(2))).sqrt())
                * ((anim_time * 16.0 * lab + PI * 1.4).sin());

        let shortalt = (anim_time * lab * 16.0 + PI / 2.0).sin();

        let slowalt = (anim_time * 6.0 + PI).cos();
        let u_slow = (anim_time * 2.5 + PI).sin();
        let slow = (anim_time * 5.0 + PI).sin();

        let u_slowalt = (anim_time * 3.0 + PI).cos();
        let short = ((5.0 / (1.5 + 3.5 * ((anim_time * lab * 16.0).sin()).powi(2))).sqrt())
            * ((anim_time * lab * 16.0).sin());
        let direction = velocity.y * -0.098 * orientation.y + velocity.x * -0.098 * orientation.x;
        let side = velocity.x * -0.098 * orientation.y + velocity.y * 0.098 * orientation.x;
        let strafe = -((1.0 / (direction).abs() - 1.0).min(1.0)).copysign(side);

        let ori: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = (if ::vek::Vec2::new(ori, last_ori)
            .map(|o| o.magnitude_squared())
            .map(|m| m > 0.001 && m.is_finite())
            .reduce_and()
            && ori.angle_between(last_ori).is_finite()
        {
            ori.angle_between(last_ori).min(0.2)
                * last_ori.determine_side(Vec2::zero(), ori).signum()
        } else {
            0.0
        } * 1.25)
            * 4.0;
        let jump = if velocity.z == 0.0 { 0.0 } else { 1.0 };

        // next.second.scale = match hands {
        //     (Some(Hands::One), Some(Hands::One)) => Vec3::one(),
        //    (_, _) => Vec3::zero(),
        // };
        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_z(0.0);
        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation = Quaternion::rotation_z(0.0);
        if speed > 0.2 && velocity.z == 0.0 {
            next.chest.orientation = Quaternion::rotation_z(short * 0.1 + strafe * 0.7 * speednorm)
                * Quaternion::rotation_y(strafe * 0.2)
                * Quaternion::rotation_x(((direction * 0.8).min(0.3)) * (1.0 - tilt.abs()));
            next.head.orientation =
                Quaternion::rotation_z(tilt * -0.5 + strafe * 0.4 * direction + strafe * -0.7)
                    * Quaternion::rotation_x(
                        (0.3 - direction * 0.4) * (1.0 - tilt.abs()) + look_dir.z * 0.7,
                    );

            next.chest.position =
                Vec3::new(short * strafe, s_a.chest.0, s_a.chest.1 + shortalt * -1.5);
        } else {
            next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1 + u_slow * 0.1);
            next.head.orientation = Quaternion::rotation_z(head_look.x + tilt * -0.75)
                * Quaternion::rotation_x(head_look.y.abs() + look_dir.z * 0.7);

            next.chest.position = Vec3::new(slowalt * 0.5, s_a.chest.0, s_a.chest.1 + u_slow * 0.5);
            next.belt.orientation = Quaternion::rotation_z(0.15 + beltstatic * tilt * 0.1);

            next.shorts.orientation = Quaternion::rotation_z(0.3 + beltstatic * tilt * 0.2);
            next.torso.orientation = Quaternion::rotation_z(tilt * 0.4);

            next.foot_l.position = Vec3::new(
                -s_a.foot.0,
                -2.0 + s_a.foot.1 + jump * -4.0,
                s_a.foot.2 + (tilt * footvertlstatic * 1.0).max(0.0),
            );
            next.foot_l.orientation = Quaternion::rotation_x(
                jump * -0.7 + u_slowalt * 0.035 - 0.2 + tilt * footvertlstatic * 0.1
                    - tilt.abs() * 0.3,
            ) * Quaternion::rotation_z(-tilt * 0.3);

            next.foot_r.position = Vec3::new(
                s_a.foot.0,
                2.0 + s_a.foot.1 + jump * 4.0,
                s_a.foot.2 + (tilt * footvertrstatic * 1.0).max(0.0),
            );
            next.foot_r.orientation = Quaternion::rotation_x(
                jump * 0.7 + u_slow * 0.035 + tilt * footvertrstatic * 0.1 - tilt.abs() * 0.3,
            ) * Quaternion::rotation_z(-tilt * 0.3);

            next.chest.orientation = Quaternion::rotation_y(u_slowalt * 0.04)
                * Quaternion::rotation_z(0.15 + tilt * -0.4);

            next.belt.position = Vec3::new(0.0, s_a.belt.0, s_a.belt.1);

            next.back.orientation = Quaternion::rotation_x(-0.2);
            next.shorts.position = Vec3::new(0.0, s_a.shorts.0, s_a.shorts.1);
        }
        match (hands, active_tool_kind, second_tool_kind) {
            ((Some(Hands::Two), _), tool, _) | ((None, Some(Hands::Two)), _, tool) => match tool {
                Some(ToolKind::Sword) | Some(ToolKind::SwordSimple) => {
                    next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                    next.hand_l.orientation =
                        Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                    next.hand_r.position = Vec3::new(s_a.shr.0, s_a.shr.1, s_a.shr.2);
                    next.hand_r.orientation =
                        Quaternion::rotation_x(s_a.shr.3) * Quaternion::rotation_y(s_a.shr.4);

                    next.control.position =
                        Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2 + direction * -5.0);
                    next.control.orientation = Quaternion::rotation_x(s_a.sc.3 + u_slow * 0.15)
                        * Quaternion::rotation_z(u_slowalt * 0.08);
                },
                Some(ToolKind::Dagger) => {
                    next.control.position = Vec3::new(0.0, 0.0, 0.0);

                    next.hand_l.position = Vec3::new(0.0, 0.0, 0.0);
                    next.hand_l.orientation = Quaternion::rotation_x(0.0);

                    next.control_l.position = Vec3::new(-7.0, 0.0, 0.0);

                    next.hand_r.position = Vec3::new(0.0, 0.0, 0.0);
                    next.hand_r.orientation = Quaternion::rotation_x(0.0);

                    next.control_r.position = Vec3::new(7.0, 0.0, 0.0);
                },
                Some(ToolKind::Axe) => {
                    next.main.position = Vec3::new(0.0, 0.0, 0.0);
                    next.main.orientation = Quaternion::rotation_x(0.0);

                    if speed < 0.5 {
                        next.head.position =
                            Vec3::new(0.0, 0.0 + s_a.head.0, s_a.head.1 + u_slow * 0.1);
                        next.head.orientation = Quaternion::rotation_z(head_look.x)
                            * Quaternion::rotation_x(0.35 + head_look.y.abs() + look_dir.z * 0.7);
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
                    } else {
                    }
                    next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                    next.hand_l.orientation =
                        Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                    next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                    next.hand_r.orientation =
                        Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                    next.control.position =
                        Vec3::new(s_a.ac.0, s_a.ac.1, s_a.ac.2 + direction * -5.0);
                    next.control.orientation = Quaternion::rotation_x(s_a.ac.3)
                        * Quaternion::rotation_y(s_a.ac.4)
                        * Quaternion::rotation_z(s_a.ac.5);
                },
                Some(ToolKind::Hammer) | Some(ToolKind::HammerSimple) | Some(ToolKind::Pick) => {
                    next.hand_l.position = Vec3::new(s_a.hhl.0, s_a.hhl.1, s_a.hhl.2);
                    next.hand_l.orientation =
                        Quaternion::rotation_x(s_a.hhl.3) * Quaternion::rotation_y(s_a.hhl.4);
                    next.hand_r.position = Vec3::new(s_a.hhr.0, s_a.hhr.1, s_a.hhr.2);
                    next.hand_r.orientation =
                        Quaternion::rotation_x(s_a.hhr.3) * Quaternion::rotation_y(s_a.hhr.4);

                    next.control.position = Vec3::new(
                        s_a.hc.0,
                        s_a.hc.1 + speed * 0.2,
                        s_a.hc.2 + speed * 0.8 + direction * -5.0,
                    );
                    next.control.orientation = Quaternion::rotation_x(s_a.hc.3 + u_slow * 0.15)
                        * Quaternion::rotation_y(s_a.hc.4 + speed * -0.04)
                        * Quaternion::rotation_z(s_a.hc.5 + u_slowalt * 0.07);
                },
                Some(ToolKind::Staff) | Some(ToolKind::Sceptre) => {
                    if speed > 0.5 && velocity.z == 0.0 {
                        next.hand_r.position = Vec3::new(
                            7.0 + s_a.hand.0 + foothoril * 1.3,
                            -4.0 + s_a.hand.1 + foothoril * -7.0,
                            1.0 + s_a.hand.2 - foothoril * 5.5,
                        );
                        next.hand_r.orientation = Quaternion::rotation_x(0.6 + footrotl * -1.2)
                            * Quaternion::rotation_y(footrotl * -0.4);
                    } else {
                        next.hand_r.position = Vec3::new(s_a.sthr.0, s_a.sthr.1, s_a.sthr.2);
                        next.hand_r.orientation =
                            Quaternion::rotation_x(s_a.sthr.3) * Quaternion::rotation_y(s_a.sthr.4);
                    };

                    next.control.position =
                        Vec3::new(s_a.stc.0, s_a.stc.1, s_a.stc.2 + direction * -5.0);

                    next.hand_l.position = Vec3::new(s_a.sthl.0, s_a.sthl.1, s_a.sthl.2);
                    next.hand_l.orientation = Quaternion::rotation_x(s_a.sthl.3);

                    next.control.orientation = Quaternion::rotation_x(s_a.stc.3 + u_slow * 0.1)
                        * Quaternion::rotation_y(s_a.stc.4)
                        * Quaternion::rotation_z(s_a.stc.5 + u_slowalt * 0.1);
                },
                Some(ToolKind::Bow) => {
                    next.main.position = Vec3::new(0.0, 0.0, 0.0);
                    next.main.orientation = Quaternion::rotation_x(0.0);
                    next.hand_l.position = Vec3::new(s_a.bhl.0, s_a.bhl.1, s_a.bhl.2);
                    next.hand_l.orientation = Quaternion::rotation_x(s_a.bhl.3);
                    next.hand_r.position = Vec3::new(s_a.bhr.0, s_a.bhr.1, s_a.bhr.2);
                    next.hand_r.orientation = Quaternion::rotation_x(s_a.bhr.3);

                    next.hold.position = Vec3::new(0.0, -1.0, -5.2);
                    next.hold.orientation = Quaternion::rotation_x(-1.57);
                    next.hold.scale = Vec3::one() * 1.0;

                    next.control.position =
                        Vec3::new(s_a.bc.0, s_a.bc.1, s_a.bc.2 + direction * -5.0);
                    next.control.orientation = Quaternion::rotation_x(u_slow * 0.06)
                        * Quaternion::rotation_y(s_a.bc.4)
                        * Quaternion::rotation_z(s_a.bc.5 + u_slowalt * 0.1);
                },
                Some(ToolKind::Debug) => {
                    next.hand_l.position = Vec3::new(-7.0, 4.0, 3.0);
                    next.hand_l.orientation = Quaternion::rotation_x(1.27);
                    next.main.position = Vec3::new(-5.0, 5.0, 23.0);
                    next.main.orientation = Quaternion::rotation_x(PI);
                },
                Some(ToolKind::Farming) => {
                    if speed < 0.5 {
                        next.head.orientation = Quaternion::rotation_z(head_look.x)
                            * Quaternion::rotation_x(-0.2 + head_look.y.abs() + look_dir.z * 0.7);
                    }
                    next.hand_l.position = Vec3::new(9.0, 1.0, 1.0);
                    next.hand_l.orientation = Quaternion::rotation_x(1.57);
                    next.hand_r.position = Vec3::new(9.0, 1.0, 11.0);
                    next.hand_r.orientation = Quaternion::rotation_x(1.57);
                    next.main.position = Vec3::new(7.5, 7.5, 13.2);
                    next.main.orientation = Quaternion::rotation_y(3.14);

                    next.control.position = Vec3::new(-11.0 + slow * 2.0, 1.8, 4.0);
                    next.control.orientation = Quaternion::rotation_x(u_slow * 0.1)
                        * Quaternion::rotation_y(0.6 + u_slow * 0.1)
                        * Quaternion::rotation_z(u_slowalt * 0.1);
                },
                _ => {},
            },
            ((_, _), _, _) => {},
        };
        match hands {
            (Some(Hands::One), _) => {
                next.control_l.position = Vec3::new(-7.0, 8.0, 2.0);
                next.control_l.orientation = Quaternion::rotation_x(-0.3);
                next.hand_l.position = Vec3::new(0.0, -0.5, 0.0);
                next.hand_l.orientation = Quaternion::rotation_x(1.57)
            },
            (_, _) => {},
        };
        match hands {
            (None | Some(Hands::One), Some(Hands::One)) => {
                next.control_r.position = Vec3::new(7.0, 8.0, 2.0);
                next.control_r.orientation = Quaternion::rotation_x(-0.3);
                next.hand_r.position = Vec3::new(0.0, -0.5, 0.0);
                next.hand_r.orientation = Quaternion::rotation_x(1.57)
            },
            (_, _) => {},
        };
        match hands {
            (None, None) | (None, Some(Hands::One)) => {
                next.hand_l.position = Vec3::new(-8.0, 2.0, 1.0);
                next.hand_l.orientation =
                    Quaternion::rotation_x(0.5) * Quaternion::rotation_y(0.25);
            },
            (_, _) => {},
        };
        match hands {
            (None, None) | (Some(Hands::One), None) => {
                next.hand_r.position = Vec3::new(8.0, 2.0, 1.0);
                next.hand_r.orientation =
                    Quaternion::rotation_x(0.5) * Quaternion::rotation_y(-0.25);
            },
            (_, _) => {},
        };

        if let (None, Some(Hands::Two)) = hands {
            next.second = next.main;
        }

        if skeleton.holding_lantern {
            next.hand_r.position = Vec3::new(
                s_a.hand.0 - head_look.x * 6.0,
                s_a.hand.1 + 5.0 - head_look.y * 10.0 + slow * 0.15,
                s_a.hand.2 + 12.0 + head_look.y * 6.0 + slow * 0.5,
            );
            next.hand_r.orientation = Quaternion::rotation_x(2.25 + slow * -0.06)
                * Quaternion::rotation_z(0.9)
                * Quaternion::rotation_y(head_look.x * 1.5)
                * Quaternion::rotation_x(head_look.y * 1.5);

            let fast = (anim_time * 8.0).sin();
            let fast2 = (anim_time * 6.0 + 8.0).sin();

            next.lantern.position = Vec3::new(-0.5, -0.5, -2.5);
            next.lantern.orientation = next.hand_r.orientation.inverse()
                * Quaternion::rotation_x((fast + 0.5) * 1.0 * speednorm + fast * 0.1)
                * Quaternion::rotation_y(
                    tilt * 1.0 * fast + tilt * 1.0 + fast2 * speednorm * 0.25 + fast2 * 0.1,
                );
        }

        next
    }
}
