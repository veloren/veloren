use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::{AbilityInfo, StageSection},
};
use core::f32::consts::PI;

pub struct ComboAnimation;
impl Animation for ComboAnimation {
    type Dependency<'a> = (
        (Option<Hands>, Option<Hands>),
        Option<&'a str>,
        Option<StageSection>,
        Option<AbilityInfo>,
        usize,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_combo\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_combo")]
    fn update_skeleton_inner<'a>(
        skeleton: &Self::Skeleton,
        (hands, ability_id, stage_section, ability_info, current_strike): Self::Dependency<'a>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        for strike in 0..=current_strike {
            let (move1, move2, move3, _move2h) = if strike == current_strike {
                match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0, 0.0),
                    Some(StageSection::Action) => {
                        (1.0, anim_time.powi(2), 0.0, anim_time.powf(0.25))
                    },
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4), 1.0),
                    _ => (0.0, 0.0, 0.0, 0.0),
                }
            } else {
                (1.0, 1.0, 1.0, 1.0)
            };
            let pullback = 1.0 - move3;
            let move2 = move2 * pullback;
            match ability_id {
                Some("common.abilities.sword.balanced_combo") => match strike {
                    0 => {
                        next.head.orientation = Quaternion::rotation_x(move1);
                    },
                    1 => {
                        next.head.orientation.rotate_z(move1);
                    },
                    _ => {},
                },
                _ => match strike {
                    0 => {
                        let (move1, move2, move3, move2h) = match stage_section {
                            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0, 0.0),
                            Some(StageSection::Action) => {
                                (1.0, anim_time.powi(2), 0.0, anim_time.powf(0.25))
                            },
                            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4), 1.0),
                            _ => (0.0, 0.0, 0.0, 0.0),
                        };

                        next.main.position = Vec3::new(0.0, 0.0, 0.0);
                        next.main.orientation = Quaternion::rotation_x(0.0);
                        next.second.position = Vec3::new(0.0, 0.0, 0.0);
                        next.second.orientation = Quaternion::rotation_z(0.0);
                        next.torso.position = Vec3::new(0.0, 0.0, 1.1);
                        next.torso.orientation = Quaternion::rotation_z(0.0);

                        match ability_info.and_then(|a| a.tool) {
                            Some(ToolKind::Sword | ToolKind::Dagger) => {
                                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                                next.main.orientation = Quaternion::rotation_x(0.0);

                                next.chest.orientation = Quaternion::rotation_z(
                                    move1 * 1.1 + move2 * -2.0 + move3 * 0.2,
                                );

                                next.head.position =
                                    Vec3::new(0.0 + move2 * 2.0, s_a.head.0, s_a.head.1);
                                next.head.orientation = Quaternion::rotation_z(
                                    move1 * -0.9 + move2 * 1.8 + move3 * -0.2,
                                );
                            },

                            Some(ToolKind::Axe) => {
                                let (move1, move2, move3) = match stage_section {
                                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                                    Some(StageSection::Action) => (1.0, anim_time, 0.0),
                                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                                    _ => (0.0, 0.0, 0.0),
                                };
                                next.head.position =
                                    Vec3::new(move2 * 2.0, s_a.head.0 + move2 * 2.0, s_a.head.1);
                                next.chest.orientation = Quaternion::rotation_x(
                                    0.0 + move1 * 0.6 + move2 * -0.6 + move3 * 0.4,
                                ) * Quaternion::rotation_y(
                                    0.0 + move1 * 0.0 + move2 * 0.0 + move3 * 0.0,
                                ) * Quaternion::rotation_z(
                                    0.0 + move1 * 1.5 + move2 * -2.5 + move3 * 1.5,
                                );
                                next.head.orientation = Quaternion::rotation_z(
                                    0.0 + move1 * -1.5 + move2 * 2.5 + move3 * -1.0,
                                );
                            },

                            Some(ToolKind::Hammer) | Some(ToolKind::Pick) => {
                                let (move1, move2, move3) = match stage_section {
                                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                                    Some(StageSection::Action) => (1.0, anim_time.powf(0.25), 0.0),
                                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                                    _ => (0.0, 0.0, 0.0),
                                };
                                let pullback = 1.0 - move3;
                                let moveret1 = move1 * pullback;
                                let moveret2 = move2 * pullback;

                                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
                                next.head.orientation =
                                    Quaternion::rotation_x(moveret1 * 0.1 + moveret2 * 0.3)
                                        * Quaternion::rotation_z(move1 * -0.2 + moveret2 * 0.2);
                                next.chest.position =
                                    Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + moveret2 * -2.0);
                                next.chest.orientation =
                                    Quaternion::rotation_x(moveret1 * 0.4 + moveret2 * -0.7)
                                        * Quaternion::rotation_y(moveret1 * 0.3 + moveret2 * -0.4)
                                        * Quaternion::rotation_z(moveret1 * 0.5 + moveret2 * -0.5);
                            },
                            Some(ToolKind::Debug) => {
                                next.hand_l.position = Vec3::new(-7.0, 4.0, 3.0);
                                next.hand_l.orientation = Quaternion::rotation_x(1.27);
                                next.main.position = Vec3::new(-5.0, 5.0, 23.0);
                                next.main.orientation = Quaternion::rotation_x(PI);
                            },
                            _ => {},
                        }

                        match hands {
                            (Some(Hands::Two), _) | (None, Some(Hands::Two)) => match ability_info
                                .and_then(|a| a.tool)
                            {
                                Some(ToolKind::Sword) => {
                                    next.hand_l.position =
                                        Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                                    next.hand_l.orientation = Quaternion::rotation_x(s_a.shl.3)
                                        * Quaternion::rotation_y(s_a.shl.4);
                                    next.hand_r.position =
                                        Vec3::new(s_a.shr.0, s_a.shr.1, s_a.shr.2);
                                    next.hand_r.orientation = Quaternion::rotation_x(s_a.shr.3)
                                        * Quaternion::rotation_y(s_a.shr.4);

                                    next.control.position = Vec3::new(
                                        s_a.sc.0 + move2 * 10.0,
                                        s_a.sc.1 + move1 * -4.0 + move2 * 16.0 + move3 * -4.0,
                                        s_a.sc.2 + move1 * 1.0,
                                    );
                                    next.control.orientation =
                                        Quaternion::rotation_x(s_a.sc.3 + move1 * -1.3)
                                            * Quaternion::rotation_y(
                                                s_a.sc.4 + move1 * -0.7 + move2 * 1.2,
                                            )
                                            * Quaternion::rotation_z(
                                                s_a.sc.5 + move1 * -PI / 2.0 + move3 * -PI / 2.0,
                                            );
                                },
                                Some(ToolKind::Axe) => {
                                    next.hand_l.position =
                                        Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                                    next.hand_l.orientation = Quaternion::rotation_x(s_a.ahl.3)
                                        * Quaternion::rotation_y(s_a.ahl.4);
                                    next.hand_r.position =
                                        Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                                    next.hand_r.orientation = Quaternion::rotation_x(s_a.ahr.3)
                                        * Quaternion::rotation_z(s_a.ahr.5);

                                    let (move1, move2, move3) = match stage_section {
                                        Some(StageSection::Buildup) => {
                                            (anim_time.powf(0.25), 0.0, 0.0)
                                        },
                                        Some(StageSection::Action) => (1.0, anim_time, 0.0),
                                        Some(StageSection::Recover) => {
                                            (1.0, 1.0, anim_time.powi(4))
                                        },
                                        _ => (0.0, 0.0, 0.0),
                                    };
                                    next.control.position = Vec3::new(
                                        s_a.ac.0 + move1 * -1.0 + move2 * -2.0 + move3 * 0.0,
                                        s_a.ac.1 + move1 * -3.0 + move2 * 3.0 + move3 * -3.5,
                                        s_a.ac.2 + move1 * 6.0 + move2 * -15.0 + move3 * -2.0,
                                    );
                                    next.control.orientation = Quaternion::rotation_x(
                                        s_a.ac.3 + move1 * 0.0 + move2 * -3.0 + move3 * 0.4,
                                    ) * Quaternion::rotation_y(
                                        s_a.ac.4 + move1 * -0.0 + move2 * -0.6 + move3 * 0.8,
                                    ) * Quaternion::rotation_z(
                                        s_a.ac.5 + move1 * -2.0 + move2 * -1.0 + move3 * 2.5,
                                    )
                                },
                                Some(ToolKind::Hammer) | Some(ToolKind::Pick) => {
                                    let (move1, move2, move3) = match stage_section {
                                        Some(StageSection::Buildup) => {
                                            (anim_time.powf(0.25), 0.0, 0.0)
                                        },
                                        Some(StageSection::Action) => (1.0, anim_time, 0.0),
                                        Some(StageSection::Recover) => {
                                            (1.0, 1.0, anim_time.powi(4))
                                        },
                                        _ => (0.0, 0.0, 0.0),
                                    };
                                    let pullback = 1.0 - move3;
                                    let moveret1 = move1 * pullback;
                                    let moveret2 = move2 * pullback;
                                    next.hand_l.position = Vec3::new(
                                        s_a.hhl.0,
                                        s_a.hhl.1,
                                        s_a.hhl.2 + moveret2 * -7.0,
                                    );
                                    next.hand_l.orientation = Quaternion::rotation_x(s_a.hhl.3)
                                        * Quaternion::rotation_y(s_a.hhl.4)
                                        * Quaternion::rotation_z(s_a.hhl.5);
                                    next.hand_r.position =
                                        Vec3::new(s_a.hhr.0, s_a.hhr.1, s_a.hhr.2);
                                    next.hand_r.orientation = Quaternion::rotation_x(s_a.hhr.3)
                                        * Quaternion::rotation_y(s_a.hhr.4)
                                        * Quaternion::rotation_z(s_a.hhr.5);

                                    next.control.position = Vec3::new(
                                        s_a.hc.0 + moveret1 * -13.0 + moveret2 * 3.0,
                                        s_a.hc.1 + (moveret2 * 5.0),
                                        s_a.hc.2 + moveret1 * 8.0 + moveret2 * -6.0,
                                    );
                                    next.control.orientation =
                                        Quaternion::rotation_x(
                                            s_a.hc.3 + (moveret1 * 1.5 + moveret2 * -2.55),
                                        ) * Quaternion::rotation_y(
                                            s_a.hc.4 + moveret1 * PI / 2.0 + moveret2 * 0.5,
                                        ) * Quaternion::rotation_z(s_a.hc.5 + (moveret2 * -0.5));
                                },
                                _ => {},
                            },
                            (_, _) => {},
                        };

                        match hands {
                            (Some(Hands::One), _) => match ability_info.and_then(|a| a.tool) {
                                Some(ToolKind::Sword | ToolKind::Dagger) => {
                                    next.control_l.position = Vec3::new(-7.0, 8.0, 2.0);
                                    next.control_l.orientation =
                                        Quaternion::rotation_x(-0.3 + move2 * 2.0)
                                            * Quaternion::rotation_y(move1 * -1.2 + move2 * -1.5)
                                            * Quaternion::rotation_z(move2 * 1.5);
                                    next.hand_l.position = Vec3::new(0.0, -0.5, 0.0);
                                    next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
                                },
                                Some(ToolKind::Axe) => {
                                    next.control_l.position = Vec3::new(
                                        -7.0 + move2 * 5.0,
                                        8.0 + move1 * 3.0 + move2 * 7.0,
                                        2.0 + move1 * -6.0 + move2 * 10.0,
                                    );
                                    next.control_l.orientation =
                                        Quaternion::rotation_x(-0.3 + move2 * 2.0)
                                            * Quaternion::rotation_y(move1 * -1.2 + move2 * -2.5)
                                            * Quaternion::rotation_z(move2 * 1.5);
                                    next.hand_l.position = Vec3::new(0.0, -0.5, 0.0);
                                    next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
                                },
                                Some(ToolKind::Hammer) | Some(ToolKind::Pick) => {
                                    next.control_l.position = Vec3::new(
                                        -7.0,
                                        8.0 + move1 * -4.0 + move2 * 4.0,
                                        2.0 + move1 * 16.0 + move2 * -19.0,
                                    );
                                    next.control_l.orientation =
                                        Quaternion::rotation_x(-0.3 + move1 * 1.9 + move2 * -3.0)
                                            * Quaternion::rotation_y(0.0)
                                            * Quaternion::rotation_z(0.0);
                                    next.hand_l.position = Vec3::new(0.0, -0.5, 0.0);
                                    next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
                                },

                                _ => {},
                            },
                            (_, _) => {},
                        };
                        match hands {
                            (None | Some(Hands::One), Some(Hands::One)) => {
                                match ability_info.and_then(|a| a.tool) {
                                    Some(ToolKind::Sword | ToolKind::Dagger) => {
                                        next.control_r.position =
                                            Vec3::new(7.0 + move2 * 8.0, 8.0, 2.0);
                                        next.control_r.orientation =
                                            Quaternion::rotation_x(-0.3 + move2 * 2.0)
                                                * Quaternion::rotation_y(
                                                    move1 * -1.8 + move2 * -1.5,
                                                )
                                                * Quaternion::rotation_z(move2 * 1.5);
                                        next.hand_r.position = Vec3::new(0.0, -0.5, 0.0);
                                        next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
                                    },
                                    Some(ToolKind::Axe) => {
                                        next.control_r.position = Vec3::new(
                                            7.0 + move2 * 5.0,
                                            8.0 + move1 * 3.0 + move2 * 7.0,
                                            2.0 + move1 * -6.0 + move2 * 8.0,
                                        );
                                        next.control_r.orientation =
                                            Quaternion::rotation_x(-0.3 + move2 * 2.0)
                                                * Quaternion::rotation_y(
                                                    move1 * -1.8 + move2 * -1.5,
                                                )
                                                * Quaternion::rotation_z(move2 * 1.5);
                                        next.hand_r.position = Vec3::new(0.0, -0.5, 0.0);
                                        next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
                                    },
                                    Some(ToolKind::Hammer) | Some(ToolKind::Pick) => {
                                        next.control_r.position = Vec3::new(
                                            7.0,
                                            8.0 + move1 * -4.0 + move2h * 4.0,
                                            2.0 + move1 * 12.0 + move2h * -16.0,
                                        );
                                        next.control_r.orientation =
                                            Quaternion::rotation_x(
                                                -0.3 + move1 * 2.3 + move2h * -3.5,
                                            ) * Quaternion::rotation_y(0.0)
                                                * Quaternion::rotation_z(0.0);
                                        next.hand_r.position = Vec3::new(0.0, -0.5, 0.0);
                                        next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
                                    },
                                    _ => {},
                                }
                            },
                            (_, _) => {},
                        };

                        match hands {
                            (None, None) | (None, Some(Hands::One)) => {
                                next.hand_l.position = Vec3::new(-4.5, 8.0, 5.0);
                                next.hand_l.orientation =
                                    Quaternion::rotation_x(1.9) * Quaternion::rotation_y(-0.5)
                            },
                            (_, _) => {},
                        };
                        match hands {
                            (None, None) | (Some(Hands::One), None) => {
                                next.hand_r.position = Vec3::new(4.5, 8.0, 5.0);
                                next.hand_r.orientation =
                                    Quaternion::rotation_x(1.9) * Quaternion::rotation_y(0.5)
                            },
                            (_, _) => {},
                        };

                        if let (None, Some(Hands::Two)) = hands {
                            next.second = next.main;
                        }

                        if skeleton.holding_lantern {
                            next.hand_r.position =
                                Vec3::new(s_a.hand.0, s_a.hand.1 + 5.0, s_a.hand.2 + 12.0);
                            next.hand_r.orientation =
                                Quaternion::rotation_x(2.25) * Quaternion::rotation_z(0.9);

                            next.lantern.position = Vec3::new(-0.5, -0.5, -1.5);
                            next.lantern.orientation = next.hand_r.orientation.inverse();
                        }
                    },
                    1 => {
                        let (movement1base, movement2base, movement3) = match stage_section {
                            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                            Some(StageSection::Action) => (1.0, anim_time, 0.0),
                            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                            _ => (0.0, 0.0, 0.0),
                        };
                        let pullback = 1.0 - movement3;
                        let move1 = movement1base * pullback;
                        let move2 = movement2base * pullback;
                        next.second.position = Vec3::new(0.0, 0.0, 0.0);
                        next.second.orientation = Quaternion::rotation_z(0.0);
                        next.main.position = Vec3::new(0.0, 0.0, 0.0);
                        next.main.orientation = Quaternion::rotation_x(0.0);
                        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

                        match ability_info.and_then(|a| a.tool) {
                            Some(ToolKind::Sword) => {
                                next.head.position = Vec3::new(
                                    0.0 + 2.0 + move2 * -2.0,
                                    2.0 + move2 * -2.0 + s_a.head.0,
                                    s_a.head.1,
                                );

                                next.chest.orientation = Quaternion::rotation_x(move2 * -0.15)
                                    * Quaternion::rotation_y(move1 * -0.1 + move2 * 0.15)
                                    * Quaternion::rotation_z(-1.0 + move1 * -0.6 + move2 * 1.6);

                                next.belt.orientation = Quaternion::rotation_x(move1 * 0.1)
                                    * Quaternion::rotation_z(move2.sin() * 0.5);

                                next.shorts.orientation = Quaternion::rotation_x(move1 * 0.1)
                                    * Quaternion::rotation_z(move2.sin() * 1.5);

                                next.head.orientation = Quaternion::rotation_x(move2 * 0.15)
                                    * Quaternion::rotation_z(1.07 + move1 * 0.4 + move2 * -1.5);
                            },

                            Some(ToolKind::Axe) => {
                                let (movement1, movement2, movement3) = match stage_section {
                                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                                    Some(StageSection::Action) => (1.0, anim_time, 0.0),
                                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                                    _ => (0.0, 0.0, 0.0),
                                };

                                next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);

                                next.chest.orientation =
                                    Quaternion::rotation_x(0.4 + movement2 * -0.5)
                                        * Quaternion::rotation_y(
                                            movement1 * -0.1 + movement2 * 0.0,
                                        )
                                        * Quaternion::rotation_z(
                                            0.5 + movement1 * -0.6 + movement2 * 0.6,
                                        );

                                next.belt.orientation =
                                    Quaternion::rotation_x(movement1 * -0.2 + movement2 * 0.2);

                                next.shorts.orientation =
                                    Quaternion::rotation_x(movement1 * -0.2 + movement2 * 0.2);

                                next.head.orientation =
                                    Quaternion::rotation_y(movement1 * 0.0 + movement3 * -0.0)
                                        * Quaternion::rotation_z(
                                            1.0 + movement1 * -0.5 + movement2 * 0.0,
                                        );
                                next.torso.position = Vec3::new(
                                    0.0,
                                    0.0,
                                    -11.0
                                        + 11.0 * (movement1 * 0.5 * PI).sin()
                                        + 11.0 * (movement2 * 0.5 * PI + 0.5 * PI).sin(),
                                );
                                next.torso.orientation = Quaternion::rotation_z(
                                    movement1.powi(2) * -6.0 + movement2 * -1.7,
                                );

                                next.foot_l.position = Vec3::new(
                                    -s_a.foot.0 + (movement1 * -1.0 + movement2 * -3.0),
                                    s_a.foot.1,
                                    s_a.foot.2 + (movement2 * 6.0),
                                );
                                next.foot_l.orientation =
                                    Quaternion::rotation_x(movement1 * 0.2 + movement2 * 0.5)
                                        * Quaternion::rotation_y(movement2 * 0.5);

                                next.foot_r.position = Vec3::new(
                                    s_a.foot.0,
                                    s_a.foot.1 + (movement1 * -2.0 + movement2 * -3.0),
                                    s_a.foot.2,
                                );
                                next.foot_r.orientation =
                                    Quaternion::rotation_x(movement1 * -0.5 + movement2 * -0.5);
                                next.head.orientation = Quaternion::rotation_x(movement2 * 0.25)
                                    * Quaternion::rotation_z(movement2 * 0.8);
                            },
                            _ => {},
                        }
                        match hands {
                            (Some(Hands::Two), _) | (None, Some(Hands::Two)) => {
                                match ability_info.and_then(|a| a.tool) {
                                    Some(ToolKind::Sword) => {
                                        next.main.position = Vec3::new(0.0, 0.0, 0.0);
                                        next.main.orientation = Quaternion::rotation_x(0.0);

                                        next.hand_l.position =
                                            Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                                        next.hand_l.orientation = Quaternion::rotation_x(s_a.shl.3)
                                            * Quaternion::rotation_y(s_a.shl.4);
                                        next.hand_r.position =
                                            Vec3::new(s_a.shr.0, s_a.shr.1, s_a.shr.2);
                                        next.hand_r.orientation = Quaternion::rotation_x(s_a.shr.3)
                                            * Quaternion::rotation_y(s_a.shr.4);

                                        next.control.position = Vec3::new(
                                            s_a.sc.0 + movement1base * 2.0 + movement2base * -7.0,
                                            s_a.sc.1
                                                + 8.0
                                                + movement1base * 0.6
                                                + movement2base * -15.0,
                                            s_a.sc.2
                                                + 1.0
                                                + movement1base * 0.6
                                                + movement2base * 1.5,
                                        );
                                        next.control.orientation = Quaternion::rotation_x(
                                            -0.5 + s_a.sc.3 + movement1base * -1.2,
                                        ) * Quaternion::rotation_y(
                                            s_a.sc.4 - 0.6 + movement2base * -0.2,
                                        ) * Quaternion::rotation_z(
                                            s_a.sc.5 - PI / 2.0 + movement1base * PI,
                                        );
                                    },
                                    Some(ToolKind::Axe) => {
                                        next.hand_l.position =
                                            Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                                        next.hand_l.orientation = Quaternion::rotation_x(s_a.ahl.3)
                                            * Quaternion::rotation_y(s_a.ahl.4);
                                        next.hand_r.position =
                                            Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                                        next.hand_r.orientation = Quaternion::rotation_x(s_a.ahr.3)
                                            * Quaternion::rotation_z(s_a.ahr.5);
                                        let (move1, move2, _move3) = match stage_section {
                                            Some(StageSection::Buildup) => {
                                                (anim_time.powf(0.25), 0.0, 0.0)
                                            },
                                            Some(StageSection::Action) => (1.0, anim_time, 0.0),
                                            Some(StageSection::Recover) => {
                                                (1.0, 1.0, anim_time.powi(4))
                                            },
                                            _ => (0.0, 0.0, 0.0),
                                        };
                                        next.control.position = Vec3::new(
                                            s_a.ac.0 + move1 * -1.0 + move2 * -2.0,
                                            s_a.ac.1 + move1 * -3.0 + move2 * 3.0,
                                            s_a.ac.2 + move1 * 6.0 + move2 * -15.0,
                                        );
                                        next.control.orientation = Quaternion::rotation_x(
                                            s_a.ac.3 + move1 * 0.0 + move2 * -3.0,
                                        ) * Quaternion::rotation_y(
                                            s_a.ac.4 + move1 * -0.0 + move2 * -0.4,
                                        ) * Quaternion::rotation_z(
                                            s_a.ac.5 + move1 * -2.0 + move2 * -1.0,
                                        )
                                    },
                                    _ => {},
                                }
                            },
                            (_, _) => {},
                        };

                        match hands {
                            (Some(Hands::One), _) => match ability_info.and_then(|a| a.tool) {
                                Some(ToolKind::Sword) => {
                                    next.control_l.position =
                                        Vec3::new(-7.0 + movement2base * -5.0, 8.0, 2.0);
                                    next.control_l.orientation = Quaternion::rotation_x(1.7)
                                        * Quaternion::rotation_y(
                                            -2.7 + movement1base * -1.0 + movement2base * 2.0,
                                        )
                                        * Quaternion::rotation_z(1.5 + movement1base * PI);
                                    next.hand_l.position = Vec3::new(0.0, -0.5, 0.0);
                                    next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
                                },
                                Some(ToolKind::Axe) => {
                                    next.control_l.position = Vec3::new(
                                        -2.0 + movement1base * -5.0,
                                        18.0 + movement1base * -10.0,
                                        6.0 + movement1base * -10.0,
                                    );
                                    next.control_l.orientation =
                                        Quaternion::rotation_x(1.7 + movement2base * 1.5)
                                            * Quaternion::rotation_y(-3.7)
                                            * Quaternion::rotation_z(
                                                1.5 + movement2base * PI / 2.0,
                                            );
                                    next.hand_l.position = Vec3::new(0.0, -0.5, 0.0);
                                    next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
                                },

                                _ => {},
                            },
                            (_, _) => {},
                        };
                        match hands {
                            (None | Some(Hands::One), Some(Hands::One)) => {
                                match ability_info.and_then(|a| a.tool) {
                                    Some(ToolKind::Sword) => {
                                        next.control_r.position =
                                            Vec3::new(15.0 + move2 * -15.0, 8.0 + move2 * 5.0, 2.0);
                                        next.control_r.orientation = Quaternion::rotation_x(1.7)
                                            * Quaternion::rotation_y(
                                                -3.3 + move1 * -1.0 + move2 * 2.0,
                                            )
                                            * Quaternion::rotation_z(1.5 + move1 * PI);
                                        next.hand_r.position = Vec3::new(0.0, -0.5, 0.0);
                                        next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
                                    },
                                    Some(ToolKind::Axe) => {
                                        next.control_r.position = Vec3::new(
                                            12.0 + move1 * -10.0,
                                            18.0 + move1 * -10.0,
                                            4.0 + move1 * -2.0,
                                        );
                                        next.control_r.orientation =
                                            Quaternion::rotation_x(1.7 + move2 * 1.5)
                                                * Quaternion::rotation_y(-3.3)
                                                * Quaternion::rotation_z(1.5 + move2 * PI / 2.0);
                                        next.hand_r.position = Vec3::new(0.0, -0.5, 0.0);
                                        next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
                                    },

                                    _ => {},
                                }
                            },
                            (_, _) => {},
                        };

                        match hands {
                            (None, None) | (None, Some(Hands::One)) => {
                                next.hand_l.position = Vec3::new(-4.5, 8.0, 5.0);
                                next.hand_l.orientation =
                                    Quaternion::rotation_x(1.9) * Quaternion::rotation_y(-0.5)
                            },
                            (_, _) => {},
                        };
                        match hands {
                            (None, None) | (Some(Hands::One), None) => {
                                next.hand_r.position = Vec3::new(4.5, 8.0, 5.0);
                                next.hand_r.orientation =
                                    Quaternion::rotation_x(1.9) * Quaternion::rotation_y(0.5)
                            },
                            (_, _) => {},
                        };

                        if let (None, Some(Hands::Two)) = hands {
                            next.second = next.main;
                        }
                    },
                    2 => {
                        let (move1, move2, move3) = match stage_section {
                            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                            Some(StageSection::Action) => (1.0, anim_time, 0.0),
                            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                            _ => (0.0, 0.0, 0.0),
                        };
                        next.second.position = Vec3::new(0.0, 0.0, 0.0);
                        next.second.orientation = Quaternion::rotation_z(0.0);
                        next.main.position = Vec3::new(0.0, 0.0, 0.0);
                        next.main.orientation = Quaternion::rotation_x(0.0);
                        let pullback = 1.0 - move3;
                        if let Some(ToolKind::Sword) = ability_info.and_then(|a| a.tool) {
                            next.chest.orientation = Quaternion::rotation_x(0.15)
                                * Quaternion::rotation_y((-0.1) * pullback)
                                * Quaternion::rotation_z(
                                    (0.4 + move1 * 1.5 + move2 * -2.5) * pullback,
                                );
                            next.head.orientation =
                                Quaternion::rotation_z(-0.4 + move1 * -1.0 + move2 * 1.5);
                        }
                        #[allow(clippy::single_match)]
                        match hands {
                            (Some(Hands::Two), _) | (None, Some(Hands::Two)) => match ability_info
                                .and_then(|a| a.tool)
                            {
                                Some(ToolKind::Sword) => {
                                    next.hand_l.position =
                                        Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                                    next.hand_l.orientation = Quaternion::rotation_x(s_a.shl.3)
                                        * Quaternion::rotation_y(s_a.shl.4);
                                    next.hand_r.position =
                                        Vec3::new(s_a.shr.0, s_a.shr.1, s_a.shr.2);
                                    next.hand_r.orientation = Quaternion::rotation_x(s_a.shr.3)
                                        * Quaternion::rotation_y(s_a.shr.4);

                                    next.control.position = Vec3::new(
                                        s_a.sc.0 + (-1.4 + move1 * -3.0 + move2 * -2.0) * pullback,
                                        s_a.sc.1 + (-1.4 + move1 * 3.0 + move2 * 3.0) * pullback,
                                        s_a.sc.2 + (-1.9 + move1 * 2.5 * pullback),
                                    );
                                    next.control.orientation =
                                        Quaternion::rotation_x(s_a.sc.3 + (-1.7) * pullback)
                                            * Quaternion::rotation_y(
                                                s_a.sc.4
                                                    + (0.4 + move1 * 1.5 + move2 * -2.5) * pullback,
                                            )
                                            * Quaternion::rotation_z(
                                                s_a.sc.5 + (1.67 + move2 * PI / 2.0) * pullback,
                                            );
                                },
                                _ => {},
                            },
                            (_, _) => {},
                        };

                        match hands {
                            #[allow(clippy::single_match)]
                            (Some(Hands::One), _) => match ability_info.and_then(|a| a.tool) {
                                Some(ToolKind::Sword) => {
                                    next.control_l.position = Vec3::new(-12.0, 8.0, 2.0);
                                    next.control_l.orientation = Quaternion::rotation_x(1.7)
                                        * Quaternion::rotation_y(
                                            -3.7 + move1 * -0.75 + move2 * 1.0,
                                        )
                                        * Quaternion::rotation_z(3.69);
                                    next.hand_l.position = Vec3::new(0.0, -0.5, 0.0);
                                    next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
                                },

                                _ => {},
                            },
                            (_, _) => {},
                        };
                        match hands {
                            #[allow(clippy::single_match)]
                            (None | Some(Hands::One), Some(Hands::One)) => {
                                match ability_info.and_then(|a| a.tool) {
                                    Some(ToolKind::Sword) => {
                                        next.control_r.position =
                                            Vec3::new(0.0 + move1 * -8.0, 13.0, 2.0);
                                        next.control_r.orientation = Quaternion::rotation_x(1.7)
                                            * Quaternion::rotation_y(
                                                -2.3 + move1 * -2.3 + move2 * 1.0,
                                            )
                                            * Quaternion::rotation_z(3.69);
                                        next.hand_r.position = Vec3::new(0.0, -0.5, 0.0);
                                        next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
                                    },

                                    _ => {},
                                }
                            },
                            (_, _) => {},
                        };

                        match hands {
                            (None, None) | (None, Some(Hands::One)) => {
                                next.hand_l.position = Vec3::new(-4.5, 8.0, 5.0);
                                next.hand_l.orientation =
                                    Quaternion::rotation_x(1.9) * Quaternion::rotation_y(-0.5)
                            },
                            (_, _) => {},
                        };
                        match hands {
                            (None, None) | (Some(Hands::One), None) => {
                                next.hand_r.position = Vec3::new(4.5, 8.0, 5.0);
                                next.hand_r.orientation =
                                    Quaternion::rotation_x(1.9) * Quaternion::rotation_y(0.5)
                            },
                            (_, _) => {},
                        };

                        if let (None, Some(Hands::Two)) = hands {
                            next.second = next.main;
                        }
                    },
                    _ => {},
                },
            }
        }
        next
    }
}
