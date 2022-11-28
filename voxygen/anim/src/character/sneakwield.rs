use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{AbilitySpec, Hands, ToolKind};
use core::{f32::consts::PI, ops::Mul};

pub struct SneakWieldAnimation;

impl Animation for SneakWieldAnimation {
    type Dependency<'a> = (
        (Option<ToolKind>, Option<&'a AbilitySpec>),
        Option<ToolKind>,
        (Option<Hands>, Option<Hands>),
        Vec3<f32>,
        Vec3<f32>,
        Vec3<f32>,
        f32,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_sneakwield\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_sneakwield")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            (active_tool_kind, active_tool_spec),
            second_tool_kind,
            hands,
            velocity,
            orientation,
            last_ori,
            global_time,
        ): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();
        *rate = 1.0;
        let slow = (anim_time * 3.0).sin();
        let breathe = ((anim_time * 0.5).sin()).abs();
        let walkintensity = if speed > 5.0 { 1.0 } else { 0.45 };
        let lower = if speed > 5.0 { 0.0 } else { 1.0 };
        let _snapfoot = if speed > 5.0 { 1.1 } else { 2.0 };
        let lab: f32 = 1.0;
        let foothoril = (anim_time * 7.0 * lab + PI * 1.45).sin();
        let foothorir = (anim_time * 7.0 * lab + PI * (0.45)).sin();
        let speednorm = speed / 4.0;

        let footvertl = (anim_time * 7.0 * lab).sin();
        let footvertr = (anim_time * 7.0 * lab + PI).sin();

        let footrotl = ((5.0 / (2.5 + (2.5) * ((anim_time * 7.0 * lab + PI * 1.4).sin()).powi(2)))
            .sqrt())
            * ((anim_time * 7.0 * lab + PI * 1.4).sin());

        let footrotr = ((5.0 / (1.0 + (4.0) * ((anim_time * 7.0 * lab + PI * 0.4).sin()).powi(2)))
            .sqrt())
            * ((anim_time * 7.0 * lab + PI * 0.4).sin());

        let short = (anim_time * lab * 7.0).sin();
        let noisea = (anim_time * 11.0 + PI / 6.0).sin();
        let noiseb = (anim_time * 19.0 + PI / 4.0).sin();

        let shorte = ((5.0 / (4.0 + 1.0 * ((anim_time * lab * 7.0).sin()).powi(2))).sqrt())
            * ((anim_time * lab * 7.0).sin());

        let shortalt = (anim_time * lab * 7.0 + PI / 2.0).sin();

        let head_look = Vec2::new(
            (global_time + anim_time / 18.0).floor().mul(7331.0).sin() * 0.2,
            (global_time + anim_time / 18.0).floor().mul(1337.0).sin() * 0.1,
        );

        let orientation: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if vek::Vec2::new(orientation, last_ori)
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
        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_x(0.0);
        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation = Quaternion::rotation_x(0.0);
        next.hold.scale = Vec3::one() * 0.0;

        if speed > 0.5 {
            next.hand_l.position = Vec3::new(1.0 - s_a.hand.0, 4.0 + s_a.hand.1, 1.0 + s_a.hand.2);
            next.hand_l.orientation = Quaternion::rotation_x(1.0);

            next.hand_r.position = Vec3::new(-1.0 + s_a.hand.0, -1.0 + s_a.hand.1, s_a.hand.2);
            next.hand_r.orientation = Quaternion::rotation_x(0.4);
            next.head.position = Vec3::new(0.0, 1.0 + s_a.head.0, -1.0 + s_a.head.1 + short * 0.06);
            next.head.orientation =
                Quaternion::rotation_z(tilt * -2.5 + head_look.x * 0.2 - short * 0.06)
                    * Quaternion::rotation_x(head_look.y + 0.45);

            next.chest.position = Vec3::new(0.0, s_a.chest.0, -1.0 + s_a.chest.1 + shortalt * -0.5);
            next.chest.orientation = Quaternion::rotation_z(0.3 + short * 0.08 + tilt * -0.2)
                * Quaternion::rotation_y(tilt * 0.8)
                * Quaternion::rotation_x(-0.5);

            next.belt.position = Vec3::new(0.0, 0.5 + s_a.belt.0, 0.7 + s_a.belt.1);
            next.belt.orientation = Quaternion::rotation_z(short * 0.1 + tilt * -1.1)
                * Quaternion::rotation_y(tilt * 0.5)
                * Quaternion::rotation_x(0.2);

            next.back.orientation =
                Quaternion::rotation_x(-0.25 + short * 0.1 + noisea * 0.1 + noiseb * 0.1);

            next.shorts.position = Vec3::new(0.0, 1.0 + s_a.shorts.0, 1.0 + s_a.shorts.1);
            next.shorts.orientation = Quaternion::rotation_z(short * 0.16 + tilt * -1.5)
                * Quaternion::rotation_y(tilt * 0.7)
                * Quaternion::rotation_x(0.3);

            next.foot_l.position = Vec3::new(
                -s_a.foot.0,
                s_a.foot.1 + foothoril * -10.5 * walkintensity - lower * 1.0,
                1.0 + s_a.foot.2 + ((footvertl * -1.7).max(-1.0)) * walkintensity,
            );
            next.foot_l.orientation =
                Quaternion::rotation_x(-0.2 + footrotl * -0.8 * walkintensity)
                    * Quaternion::rotation_y(tilt * 1.8);

            next.foot_r.position = Vec3::new(
                s_a.foot.0,
                s_a.foot.1 + foothorir * -10.5 * walkintensity - lower * 1.0,
                1.0 + s_a.foot.2 + ((footvertr * -1.7).max(-1.0)) * walkintensity,
            );
            next.foot_r.orientation =
                Quaternion::rotation_x(-0.2 + footrotr * -0.8 * walkintensity)
                    * Quaternion::rotation_y(tilt * 1.8);

            next.shoulder_l.orientation = Quaternion::rotation_x(short * 0.15 * walkintensity);

            next.shoulder_r.orientation = Quaternion::rotation_x(short * -0.15 * walkintensity);

            next.lantern.orientation =
                Quaternion::rotation_x(shorte * 0.2 + 0.4) * Quaternion::rotation_y(shorte * 0.1);
        } else {
            next.head.position = Vec3::new(
                0.0,
                1.0 + s_a.head.0,
                -2.0 + s_a.head.1 + slow * 0.1 + breathe * -0.05,
            );
            next.head.orientation = Quaternion::rotation_z(head_look.x)
                * Quaternion::rotation_x(0.6 + head_look.y.abs());

            next.chest.position = Vec3::new(0.0, s_a.chest.0, -3.0 + s_a.chest.1 + slow * 0.1);
            next.chest.orientation = Quaternion::rotation_x(-0.7);

            next.belt.position = Vec3::new(0.0, s_a.belt.0, s_a.belt.1);
            next.belt.orientation = Quaternion::rotation_z(0.3 + head_look.x * -0.1);

            next.hand_l.position = Vec3::new(1.0 - s_a.hand.0, 5.0 + s_a.hand.1, 0.0 + s_a.hand.2);
            next.hand_l.orientation = Quaternion::rotation_x(1.35);

            next.hand_r.position = Vec3::new(-1.0 + s_a.hand.0, s_a.hand.1, s_a.hand.2);
            next.hand_r.orientation = Quaternion::rotation_x(0.4);

            next.shorts.position = Vec3::new(0.0, s_a.shorts.0, s_a.shorts.1);
            next.shorts.orientation = Quaternion::rotation_z(0.6 + head_look.x * -0.2);

            next.foot_l.position = Vec3::new(-s_a.foot.0, -6.0 + s_a.foot.1, 1.0 + s_a.foot.2);
            next.foot_l.orientation = Quaternion::rotation_x(-0.5);

            next.foot_r.position = Vec3::new(s_a.foot.0, 4.0 + s_a.foot.1, s_a.foot.2);
        }

        if skeleton.holding_lantern {
            next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1 + 5.0, s_a.hand.2 + 9.0);
            next.hand_r.orientation = Quaternion::rotation_x(2.5);

            next.lantern.position = Vec3::new(0.0, 1.5, -5.5);
            next.lantern.orientation = next.hand_r.orientation.inverse();
        }

        match (hands, active_tool_kind, second_tool_kind) {
            ((Some(Hands::Two), _), tool, _) | ((None, Some(Hands::Two)), _, tool) => match tool {
                Some(ToolKind::Sword) => {
                    next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                    next.hand_l.orientation =
                        Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                    next.hand_r.position = Vec3::new(s_a.shr.0, s_a.shr.1, s_a.shr.2);
                    next.hand_r.orientation =
                        Quaternion::rotation_x(s_a.shr.3) * Quaternion::rotation_y(s_a.shr.4);

                    next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1 - 3.0, s_a.sc.2);
                    next.control.orientation =
                        Quaternion::rotation_x(s_a.sc.3) * Quaternion::rotation_z(0.0);
                },
                Some(ToolKind::Axe) => {
                    next.main.position = Vec3::new(0.0, 0.0, 0.0);
                    next.main.orientation = Quaternion::rotation_x(0.0);

                    if speed < 0.5 {
                        next.head.position = Vec3::new(0.0, 0.0 + s_a.head.0, s_a.head.1);
                        next.head.orientation = Quaternion::rotation_z(head_look.x)
                            * Quaternion::rotation_x(0.35 + head_look.y.abs());
                        next.chest.orientation = Quaternion::rotation_x(-0.35)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(0.15);
                        next.belt.position = Vec3::new(0.0, 1.0 + s_a.belt.0, s_a.belt.1);
                        next.belt.orientation = Quaternion::rotation_x(0.15)
                            * Quaternion::rotation_y(0.0)
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

                    next.control.position = Vec3::new(s_a.ac.0, s_a.ac.1, s_a.ac.2);
                    next.control.orientation = Quaternion::rotation_x(s_a.ac.3)
                        * Quaternion::rotation_y(s_a.ac.4)
                        * Quaternion::rotation_z(s_a.ac.5);
                },
                Some(ToolKind::Hammer) | Some(ToolKind::Pick) => {
                    next.hand_l.position = Vec3::new(s_a.hhl.0, s_a.hhl.1, s_a.hhl.2);
                    next.hand_l.orientation = Quaternion::rotation_x(s_a.hhl.3)
                        * Quaternion::rotation_y(s_a.hhl.4)
                        * Quaternion::rotation_z(s_a.hhl.5);
                    next.hand_r.position = Vec3::new(s_a.hhr.0, s_a.hhr.1, s_a.hhr.2);
                    next.hand_r.orientation = Quaternion::rotation_x(s_a.hhr.3)
                        * Quaternion::rotation_y(s_a.hhr.4)
                        * Quaternion::rotation_z(s_a.hhr.5);

                    next.control.position =
                        Vec3::new(s_a.hc.0, s_a.hc.1 + speed * 0.2, s_a.hc.2 + 6.0);
                    next.control.orientation = Quaternion::rotation_x(s_a.hc.3)
                        * Quaternion::rotation_y(s_a.hc.4 + speed * -0.04)
                        * Quaternion::rotation_z(s_a.hc.5);
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

                    next.control.position = Vec3::new(s_a.stc.0, s_a.stc.1 - 2.0, s_a.stc.2 + 4.0);

                    next.hand_l.position = Vec3::new(s_a.sthl.0, s_a.sthl.1, s_a.sthl.2);
                    next.hand_l.orientation = Quaternion::rotation_x(s_a.sthl.3);

                    next.control.orientation = Quaternion::rotation_x(s_a.stc.3)
                        * Quaternion::rotation_y(s_a.stc.4)
                        * Quaternion::rotation_z(s_a.stc.5);
                },
                Some(ToolKind::Bow) => {
                    next.main.position = Vec3::new(0.0, 0.0, 0.0);
                    next.main.orientation = Quaternion::rotation_x(0.0);
                    next.hand_l.position = Vec3::new(s_a.bhl.0, s_a.bhl.1, s_a.bhl.2);
                    next.hand_l.orientation = Quaternion::rotation_x(s_a.bhl.3);
                    next.hand_r.position = Vec3::new(s_a.bhr.0, s_a.bhr.1, s_a.bhr.2);
                    next.hand_r.orientation = Quaternion::rotation_x(s_a.bhr.3);

                    next.hold.position = Vec3::new(0.0, -1.0, -5.2);
                    next.hold.orientation = Quaternion::rotation_x(-PI / 2.0);
                    next.hold.scale = Vec3::one() * 1.0;

                    next.control.position = Vec3::new(s_a.bc.0 + 2.0, s_a.bc.1, s_a.bc.2 + 5.0);
                    next.control.orientation = Quaternion::rotation_x(speednorm * -0.5 + 0.8)
                        * Quaternion::rotation_y(s_a.bc.4)
                        * Quaternion::rotation_z(s_a.bc.5);
                },
                Some(ToolKind::Instrument) => {
                    if let Some(AbilitySpec::Custom(spec)) = active_tool_spec {
                        match spec.as_str() {
                            "Washboard" => {
                                next.hand_l.position = Vec3::new(-7.0, 0.0, 3.0);
                                next.hand_l.orientation = Quaternion::rotation_x(1.27);
                                next.main.position = Vec3::new(-5.0, -4.5, -5.0);
                                next.main.orientation = Quaternion::rotation_x(5.5);
                            },
                            _ => {
                                next.hand_l.position = Vec3::new(-7.0, 4.0, 3.0);
                                next.hand_l.orientation = Quaternion::rotation_x(1.27);
                                next.main.position = Vec3::new(-5.0, 5.0, 23.0);
                                next.main.orientation = Quaternion::rotation_x(PI);
                            },
                        }
                    }
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
                            * Quaternion::rotation_x(-0.2 + head_look.y.abs());
                    }
                    next.hand_l.position = Vec3::new(9.0, 1.0, 1.0);
                    next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0);
                    next.hand_r.position = Vec3::new(9.0, 1.0, 11.0);
                    next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0);
                    next.main.position = Vec3::new(7.5, 7.5, 13.2);
                    next.main.orientation = Quaternion::rotation_y(PI);

                    next.control.position = Vec3::new(-11.0 + slow * 2.0, 1.8, 4.0);
                    next.control.orientation = Quaternion::rotation_x(0.0)
                        * Quaternion::rotation_y(0.6)
                        * Quaternion::rotation_z(0.0);
                },
                _ => {},
            },
            ((_, _), _, _) => {},
        };
        match hands {
            (Some(Hands::One), _) => {
                next.control_l.position = Vec3::new(-7.0, 6.0, 5.0);
                next.control_l.orientation =
                    Quaternion::rotation_x(-0.3) * Quaternion::rotation_y(0.2);
                next.hand_l.position = Vec3::new(-1.0, -0.5, 0.0);
                next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
            },
            (_, _) => {},
        };
        match hands {
            (None | Some(Hands::One), Some(Hands::One)) => {
                next.control_r.position = Vec3::new(7.0, 6.0, 5.0);
                next.control_r.orientation =
                    Quaternion::rotation_x(-0.3) * Quaternion::rotation_y(-0.2);
                next.hand_r.position = Vec3::new(1.0, -0.5, 0.0);
                next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
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
