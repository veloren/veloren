use super::{
    super::{Animation, vek::*},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{AbilitySpec, Hands, ToolKind};
use core::f32::consts::PI;

pub struct WallrunAnimation;

impl Animation for WallrunAnimation {
    type Dependency<'a> = (
        (Option<ToolKind>, Option<&'a AbilitySpec>),
        Option<ToolKind>,
        (Option<Hands>, Option<Hands>),
        Vec3<f32>,
        f32,
        Option<Vec3<f32>>,
        bool,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_wallrun\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "character_wallrun"))]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            (active_tool_kind, active_tool_spec),
            second_tool_kind,
            hands,
            orientation,
            acc_vel,
            wall,
            was_wielded,
        ): Self::Dependency<'_>,
        _anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        *rate = 1.0;

        let lab: f32 = 0.8;

        let footrotl = ((1.0 / (0.5 + (0.5) * ((acc_vel * 1.6 * lab + PI * 1.4).sin()).powi(2)))
            .sqrt())
            * ((acc_vel * 1.6 * lab + PI * 1.4).sin());
        let footrotr = ((1.0 / (0.5 + (0.5) * ((acc_vel * 1.6 * lab + PI * 0.4).sin()).powi(2)))
            .sqrt())
            * ((acc_vel * 1.6 * lab + PI * 0.4).sin());

        let foothoril = (acc_vel * 2.2 * lab + PI * 1.45).sin();
        let foothorir = (acc_vel * 2.2 * lab + PI * (0.45)).sin();

        let shortalt = (acc_vel * lab * 2.2 + PI / 1.0).sin();

        let short = ((5.0 / (1.5 + 3.5 * ((acc_vel * lab * 1.6).sin()).powi(2))).sqrt())
            * ((acc_vel * lab * 1.6).sin());

        next.shorts.position = Vec3::new(0.0, s_a.shorts.0 + 2.0, s_a.shorts.1 + 1.0);
        next.belt.position = Vec3::new(0.0, s_a.belt.0 + 1.0, s_a.belt.1);

        next.foot_l.position = Vec3::new(
            -s_a.foot.0,
            s_a.foot.1 + foothorir * 6.0,
            s_a.foot.2 + shortalt * 2.0 + 2.0,
        );
        next.foot_l.orientation = Quaternion::rotation_x(0.6 + shortalt * 0.8);

        next.foot_r.position = Vec3::new(
            s_a.foot.0,
            s_a.foot.1 + foothoril * 6.0,
            s_a.foot.2 + shortalt * -2.0 + 2.0,
        );
        next.foot_r.orientation = Quaternion::rotation_x(0.6 - shortalt * 0.8);
        next.belt.orientation = Quaternion::rotation_x(0.3);
        next.shorts.orientation = Quaternion::rotation_x(0.5);

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + shortalt * 0.0);

        next.shoulder_l.orientation = Quaternion::rotation_x(short * 0.15);

        next.shoulder_r.orientation = Quaternion::rotation_x(short * -0.15);

        if wall.is_some_and(|e| e.y > 0.5) {
            let push = (1.0 - orientation.x.abs()).powi(2);
            let right_sub = -(orientation.x).min(0.0);
            let left_sub = (orientation.x).max(0.0);
            next.hand_l.position = Vec3::new(
                -s_a.hand.0,
                s_a.hand.1,
                s_a.hand.2 + push * 5.0 + 2.0 * left_sub,
            );
            next.hand_r.position = Vec3::new(
                s_a.hand.0,
                s_a.hand.1,
                s_a.hand.2 + push * 5.0 + 2.0 * right_sub,
            );
            next.hand_l.orientation =
                Quaternion::rotation_x(push * 2.0 + footrotr * -0.2 * right_sub)
                    * Quaternion::rotation_y(1.0 * left_sub)
                    * Quaternion::rotation_z(2.5 * left_sub + 1.0 * right_sub);
            next.hand_r.orientation =
                Quaternion::rotation_x(push * 2.0 + footrotl * -0.2 * left_sub)
                    * Quaternion::rotation_y(-1.0 * right_sub)
                    * Quaternion::rotation_z(-2.5 * right_sub - 1.0 * left_sub);
            next.torso.orientation = Quaternion::rotation_y(orientation.x / 1.5);
            next.chest.orientation = Quaternion::rotation_y(orientation.x / -3.0)
                * Quaternion::rotation_z(shortalt * -0.2);
            next.head.orientation = Quaternion::rotation_z(shortalt * 0.25)
                * Quaternion::rotation_z(orientation.x / -2.0)
                * Quaternion::rotation_x(-0.1);
        } else if wall.is_some_and(|e| e.y < -0.5) {
            let push = (1.0 - orientation.x.abs()).powi(2);
            let right_sub = (orientation.x).max(0.0);
            let left_sub = -(orientation.x).min(0.0);
            next.hand_l.position = Vec3::new(
                -s_a.hand.0,
                s_a.hand.1,
                s_a.hand.2 + push * 5.0 + 2.0 * left_sub,
            );
            next.hand_r.position = Vec3::new(
                s_a.hand.0,
                s_a.hand.1,
                s_a.hand.2 + push * 5.0 + 2.0 * right_sub,
            );
            next.hand_l.orientation =
                Quaternion::rotation_x(push * 2.0 + footrotr * -0.2 * right_sub)
                    * Quaternion::rotation_y(1.0 * left_sub)
                    * Quaternion::rotation_z(2.5 * left_sub + 1.0 * right_sub);
            next.hand_r.orientation =
                Quaternion::rotation_x(push * 2.0 + footrotl * -0.2 * left_sub)
                    * Quaternion::rotation_y(-1.0 * right_sub)
                    * Quaternion::rotation_z(-2.5 * right_sub - 1.0 * left_sub);
            next.chest.orientation = Quaternion::rotation_y(orientation.x);
            next.torso.orientation = Quaternion::rotation_y(orientation.x / -1.5);
            next.chest.orientation = Quaternion::rotation_y(orientation.x / 3.0)
                * Quaternion::rotation_z(shortalt * -0.2);
            next.head.orientation = Quaternion::rotation_z(shortalt * 0.25)
                * Quaternion::rotation_z(orientation.x / 2.0)
                * Quaternion::rotation_x(-0.1);
        } else if wall.is_some_and(|e| e.x < -0.5) {
            let push = (1.0 - orientation.y.abs()).powi(2);
            let right_sub = -(orientation.y).min(0.0);
            let left_sub = (orientation.y).max(0.0);
            next.hand_l.position = Vec3::new(
                -s_a.hand.0,
                s_a.hand.1,
                s_a.hand.2 + push * 5.0 + 2.0 * left_sub,
            );
            next.hand_r.position = Vec3::new(
                s_a.hand.0,
                s_a.hand.1,
                s_a.hand.2 + push * 5.0 + 2.0 * right_sub,
            );
            next.hand_l.orientation =
                Quaternion::rotation_x(push * 2.0 + footrotr * -0.2 * right_sub)
                    * Quaternion::rotation_y(1.0 * left_sub)
                    * Quaternion::rotation_z(2.5 * left_sub + 1.0 * right_sub);
            next.hand_r.orientation =
                Quaternion::rotation_x(push * 2.0 + footrotl * -0.2 * left_sub)
                    * Quaternion::rotation_y(-1.0 * right_sub)
                    * Quaternion::rotation_z(-2.5 * right_sub - 1.0 * left_sub);
            next.torso.orientation = Quaternion::rotation_y(orientation.y / 1.5);
            next.chest.orientation = Quaternion::rotation_y(orientation.y / -3.0)
                * Quaternion::rotation_z(shortalt * -0.2);
            next.head.orientation = Quaternion::rotation_z(shortalt * 0.25)
                * Quaternion::rotation_z(orientation.y / -2.0)
                * Quaternion::rotation_x(-0.1);
        } else if wall.is_some_and(|e| e.x > 0.5) {
            let push = (1.0 - orientation.y.abs()).powi(2);
            let right_sub = (orientation.y).max(0.0);
            let left_sub = -(orientation.y).min(0.0);
            next.hand_l.position = Vec3::new(
                -s_a.hand.0,
                s_a.hand.1,
                s_a.hand.2 + push * 5.0 + 2.0 * left_sub,
            );
            next.hand_r.position = Vec3::new(
                s_a.hand.0,
                s_a.hand.1,
                s_a.hand.2 + push * 5.0 + 2.0 * right_sub,
            );
            next.hand_l.orientation =
                Quaternion::rotation_x(push * 2.0 + footrotr * -0.2 * right_sub)
                    * Quaternion::rotation_y(1.0 * left_sub)
                    * Quaternion::rotation_z(2.5 * left_sub + 1.0 * right_sub);
            next.hand_r.orientation =
                Quaternion::rotation_x(push * 2.0 + footrotl * -0.2 * left_sub)
                    * Quaternion::rotation_y(-1.0 * right_sub)
                    * Quaternion::rotation_z(-2.5 * right_sub - 1.0 * left_sub);
            next.torso.orientation = Quaternion::rotation_y(orientation.y / -1.5);
            next.chest.orientation = Quaternion::rotation_y(orientation.y / 3.0)
                * Quaternion::rotation_z(shortalt * -0.2);
            next.head.orientation = Quaternion::rotation_z(shortalt * 0.25)
                * Quaternion::rotation_z(orientation.y / 2.0)
                * Quaternion::rotation_x(-0.1);
        };

        if was_wielded {
            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_x(0.0);
            next.second.position = Vec3::new(0.0, 0.0, 0.0);
            next.second.orientation = Quaternion::rotation_x(0.0);
            next.hold.scale = Vec3::one() * 0.0;

            match (hands, active_tool_kind, second_tool_kind) {
                ((Some(Hands::Two), _), tool, _) | ((None, Some(Hands::Two)), _, tool) => {
                    match tool {
                        Some(ToolKind::Sword) => {
                            next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                            next.hand_l.orientation = Quaternion::rotation_x(s_a.shl.3)
                                * Quaternion::rotation_y(s_a.shl.4);
                            next.hand_r.position = Vec3::new(s_a.shr.0, s_a.shr.1, s_a.shr.2);
                            next.hand_r.orientation = Quaternion::rotation_x(s_a.shr.3)
                                * Quaternion::rotation_y(s_a.shr.4);

                            next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1 - 3.0, s_a.sc.2);
                            next.control.orientation =
                                Quaternion::rotation_x(s_a.sc.3) * Quaternion::rotation_z(0.0);
                        },
                        Some(ToolKind::Axe) => {
                            next.main.position = Vec3::new(0.0, 0.0, 0.0);
                            next.main.orientation = Quaternion::rotation_x(0.0);

                            next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                            next.hand_l.orientation = Quaternion::rotation_x(s_a.ahl.3)
                                * Quaternion::rotation_y(s_a.ahl.4);
                            next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                            next.hand_r.orientation = Quaternion::rotation_x(s_a.ahr.3)
                                * Quaternion::rotation_z(s_a.ahr.5);

                            next.control.position = Vec3::new(s_a.ac.0, s_a.ac.1, s_a.ac.2);
                            next.control.orientation = Quaternion::rotation_x(s_a.ac.3)
                                * Quaternion::rotation_y(s_a.ac.4)
                                * Quaternion::rotation_z(s_a.ac.5);
                        },
                        Some(ToolKind::Hammer | ToolKind::Pick | ToolKind::Shovel) => {
                            next.hand_l.position = Vec3::new(s_a.hhl.0, s_a.hhl.1, s_a.hhl.2);
                            next.hand_l.orientation = Quaternion::rotation_x(s_a.hhl.3)
                                * Quaternion::rotation_y(s_a.hhl.4)
                                * Quaternion::rotation_z(s_a.hhl.5);
                            next.hand_r.position = Vec3::new(s_a.hhr.0, s_a.hhr.1, s_a.hhr.2);
                            next.hand_r.orientation = Quaternion::rotation_x(s_a.hhr.3)
                                * Quaternion::rotation_y(s_a.hhr.4)
                                * Quaternion::rotation_z(s_a.hhr.5);

                            next.control.position = Vec3::new(s_a.hc.0, s_a.hc.1, s_a.hc.2 + 6.0);
                            next.control.orientation = Quaternion::rotation_x(s_a.hc.3)
                                * Quaternion::rotation_y(s_a.hc.4)
                                * Quaternion::rotation_z(s_a.hc.5);
                        },
                        Some(ToolKind::Staff) | Some(ToolKind::Sceptre) => {
                            next.hand_r.position = Vec3::new(s_a.sthr.0, s_a.sthr.1, s_a.sthr.2);
                            next.hand_r.orientation = Quaternion::rotation_x(s_a.sthr.3)
                                * Quaternion::rotation_y(s_a.sthr.4);

                            next.control.position =
                                Vec3::new(s_a.stc.0, s_a.stc.1 - 2.0, s_a.stc.2 + 4.0);

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

                            next.control.position =
                                Vec3::new(s_a.bc.0 + 2.0, s_a.bc.1, s_a.bc.2 + 5.0);
                            next.control.orientation = Quaternion::rotation_x(0.8)
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
                            next.head.orientation = Quaternion::rotation_x(-0.2);
                            next.hand_l.position = Vec3::new(9.0, 1.0, 1.0);
                            next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0);
                            next.hand_r.position = Vec3::new(9.0, 1.0, 11.0);
                            next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0);
                            next.main.position = Vec3::new(7.5, 7.5, 13.2);
                            next.main.orientation = Quaternion::rotation_y(PI);

                            next.control.position = Vec3::new(-11.0, 1.8, 4.0);
                            next.control.orientation = Quaternion::rotation_x(0.0)
                                * Quaternion::rotation_y(0.6)
                                * Quaternion::rotation_z(0.0);
                        },
                        _ => {},
                    }
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
        }

        next
    }
}
