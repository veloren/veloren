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
            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_z(0.0);
            match ability_id {
                Some("common.abilities.sword.balanced_combo") => {
                    next.main_weapon_trail = true;
                    next.off_weapon_trail = true;
                    let (move1, move2, move3, move2alt) = if strike == current_strike {
                        match stage_section {
                            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0, 0.0),
                            Some(StageSection::Action) => {
                                (1.0, anim_time.powi(2), 0.0, anim_time.powf(0.25))
                            },
                            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4), 1.0),
                            _ => (0.0, 0.0, 0.0, 0.0),
                        }
                    } else {
                        (1.0, 1.0, 0.0, 1.0)
                    };
                    let pullback = 1.0 - move3;
                    let move2 = move2 * pullback;
                    let move2alt = move2alt * pullback;

                    match strike {
                        0 => {
                            next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                            next.hand_l.orientation = Quaternion::rotation_x(s_a.shl.3)
                                * Quaternion::rotation_y(s_a.shl.4);
                            next.chest.orientation =
                                Quaternion::rotation_z(move1 * 0.3 + move2alt * -1.0);
                            next.head.orientation =
                                Quaternion::rotation_z(move1 * -0.15 + move2alt * 0.5);
                            next.belt.orientation =
                                Quaternion::rotation_z(move1 * -0.2 + move2alt * 0.5);
                            next.shorts.orientation =
                                Quaternion::rotation_z(move1 * -0.25 + move2alt * 0.7);
                            next.hand_r.position = Vec3::new(
                                -s_a.sc.0 + 6.0 + move1 * -12.0,
                                -4.0 + move1 * 3.0,
                                -2.0,
                            );
                            next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                            next.control.position = Vec3::new(
                                s_a.sc.0 + move1 * -3.0 + move2 * 20.0,
                                s_a.sc.1,
                                s_a.sc.2 + move1 * 10.0 + move2alt * -10.0,
                            );
                            next.control.orientation =
                                Quaternion::rotation_x(s_a.sc.3 + move2alt * -1.2)
                                    * Quaternion::rotation_y(move1 * -0.9 + move2 * 2.3)
                                    * Quaternion::rotation_z(move2alt * -1.5);
                        },
                        1 => {
                            next.chest
                                .orientation
                                .rotate_z(move1 * -0.2 + move2alt * 1.4);
                            next.head
                                .orientation
                                .rotate_z(move1 * 0.1 + move2alt * -0.4);
                            next.belt
                                .orientation
                                .rotate_z(move1 * 0.1 + move2alt * -0.4);
                            next.shorts
                                .orientation
                                .rotate_z(move1 * 0.2 + move2alt * -0.8);
                            next.control.position += Vec3::new(move2 * -25.0, 0.0, move2 * 10.0);
                            next.control.orientation.rotate_x(move2alt * 0.4);
                            next.control.orientation.rotate_y(move2 * -0.6);
                            next.control.orientation.rotate_z(move2alt * 3.0);
                        },
                        _ => {},
                    }
                },
                Some("common.abilities.sword.offensive_combo") => {
                    let (move1, move2, move3, move2alt) = if strike == current_strike {
                        match stage_section {
                            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0, 0.0),
                            Some(StageSection::Action) => {
                                (1.0, anim_time.powi(2), 0.0, anim_time.powf(0.25))
                            },
                            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4), 1.0),
                            _ => (0.0, 0.0, 0.0, 0.0),
                        }
                    } else {
                        (1.0, 1.0, 0.0, 1.0)
                    };

                    if move2 > 0.0 {
                        next.main_weapon_trail = true;
                    } else {
                        next.main_weapon_trail = false;
                    }

                    match strike {
                        0 => {
                            next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                            next.hand_l.orientation = Quaternion::rotation_x(s_a.shl.3)
                                * Quaternion::rotation_y(s_a.shl.4);
                            next.hand_r.position = Vec3::new(
                                -s_a.sc.0 + 6.0 + move1 * -12.0,
                                -4.0 + move1 * 3.0,
                                -2.0,
                            );
                            next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                            next.control.position = Vec3::new(
                                s_a.sc.0 + move1 * 13.0,
                                s_a.sc.1 - move1 * 3.0,
                                s_a.sc.2 + move1 * 9.0,
                            );
                            next.control.orientation =
                                Quaternion::rotation_x(s_a.sc.3 + move1 * 0.5)
                                    * Quaternion::rotation_y(move1 * 1.4)
                                    * Quaternion::rotation_z(0.0);
                            next.chest.orientation = Quaternion::rotation_z(move1 * -0.6);
                            next.head.orientation = Quaternion::rotation_z(move1 * 0.35);
                            next.belt.orientation = Quaternion::rotation_z(move1 * 0.25);
                            next.shorts.orientation = Quaternion::rotation_z(move1 * 0.4);

                            next.chest.orientation.rotate_z(move2 * 1.1);
                            next.head.orientation.rotate_z(move2 * -0.75);
                            next.belt.orientation.rotate_z(move2 * -0.6);
                            next.shorts.orientation.rotate_z(move2 * -0.8);
                            next.control.orientation.rotate_z(move2 * 2.9);
                            next.control.position += Vec3::new(
                                move2 * -16.0,
                                (1.0 - (move2 - 0.6)).abs() * 6.0,
                                move2 * -6.0,
                            );
                        },
                        1 => {
                            next.chest.orientation.rotate_z(move1 * -0.15);
                            next.head.orientation.rotate_z(move1 * 0.12);
                            next.belt.orientation.rotate_z(move1 * 0.08);
                            next.shorts.orientation.rotate_z(move1 * 0.12);
                            next.control.orientation.rotate_z(move1 * 0.2);
                            next.control.orientation.rotate_x(move1 * PI);
                            next.control.orientation.rotate_y(move1 * 0.05);

                            next.chest.orientation.rotate_z(move2 * -0.9);
                            next.head.orientation.rotate_z(move2 * 0.65);
                            next.belt.orientation.rotate_z(move2 * 0.45);
                            next.shorts.orientation.rotate_z(move2 * 0.7);
                            next.control.orientation.rotate_z(move2 * -3.0);
                            next.control.orientation.rotate_y(move2 * -0.4);
                            next.control.position += Vec3::new(move2 * 17.0, 0.0, move2 * 6.0);
                        },
                        2 => {
                            next.chest.orientation.rotate_z(move1 * 0.5);
                            next.chest.orientation.rotate_x(move1 * 0.2);
                            next.head.orientation.rotate_z(move1 * -0.4);
                            next.belt.orientation.rotate_z(move1 * -0.1);
                            next.shorts.orientation.rotate_z(move1 * -0.45);
                            next.control.orientation.rotate_z(move1 * -0.2);
                            next.control.orientation.rotate_y(move1 * -1.4);
                            next.control.orientation.rotate_z(move1 * 0.15);
                            next.control.orientation.rotate_x(move1 * 0.5);
                            next.control.position += Vec3::new(
                                move1 * -8.0,
                                (move1 - 0.5).max(0.0) * -10.0,
                                move1.powi(3) * 16.0,
                            );
                            next.foot_l.position += Vec3::new(0.0, move1 * 3.0, move1 * 3.0);
                            next.foot_l.orientation.rotate_x(move1 * 0.2);

                            next.foot_l.orientation.rotate_x(move2 * -0.2);
                            next.foot_l.position += Vec3::new(0.0, 0.0, move2 * -3.0);
                            next.chest.orientation.rotate_x(move2 * -0.5);
                            next.control.orientation.rotate_x(move2 * -2.3);
                            next.control.position += Vec3::new(0.0, move2 * 16.0, move2 * -25.0);
                        },
                        _ => {},
                    }
                },
                Some("common.abilities.sword.offensive_advance") => {
                    let (move1, move2, move3) = if strike == current_strike {
                        match stage_section {
                            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                            Some(StageSection::Action) => (1.0, anim_time.powi(2), 0.0),
                            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                            _ => (0.0, 0.0, 0.0),
                        }
                    } else {
                        (1.0, 1.0, 0.0)
                    };

                    next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                    next.hand_l.orientation =
                        Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                    next.hand_r.position =
                        Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                    next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                    next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                    next.control.orientation = Quaternion::rotation_x(s_a.sc.3)
                        * Quaternion::rotation_z(move2.signum() * -PI / 2.0);

                    next.control.orientation.rotate_x(move1 * 1.6 + move2 * 0.5);
                    next.chest.orientation = Quaternion::rotation_z(move1 * 1.0);
                    next.head.orientation = Quaternion::rotation_z(move1 * -0.7);
                    next.belt.orientation = Quaternion::rotation_z(move1 * -0.2);
                    next.shorts.orientation = Quaternion::rotation_z(move1 * -0.5);
                    next.control.position += Vec3::new(0.0, 0.0, move1 * 5.0);

                    next.chest.orientation.rotate_z(move2 * -1.9);
                    next.head.orientation.rotate_z(move2 * 1.4);
                    next.belt.orientation.rotate_z(move2 * 0.6);
                    next.shorts.orientation.rotate_z(move2 * 1.2);
                    next.control.orientation.rotate_z(move2 * -3.5);
                    next.control.position += Vec3::new(move2 * 9.0, move2 * 4.0, 0.0);

                    next.chest.orientation.rotate_z(move3 * 0.7);
                    next.head.orientation.rotate_z(move3 * -0.4);
                    next.belt.orientation.rotate_z(move3 * -0.2);
                    next.shorts.orientation.rotate_z(move3 * -0.5);
                    next.control.orientation.rotate_z(move3 * 0.4);
                    next.control.orientation.rotate_x(move3 * 1.4);
                    next.control.position += Vec3::new(move3 * -9.0, 0.0, 0.0);
                },
                Some("common.abilities.sword.crippling_combo") => {
                    let (move1, move2, move3) = if strike == current_strike {
                        match stage_section {
                            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                            Some(StageSection::Action) => (1.0, anim_time, 0.0),
                            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                            _ => (0.0, 0.0, 0.0),
                        }
                    } else {
                        (1.0, 1.0, 0.0)
                    };

                    match strike {
                        0 => {
                            next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                            next.hand_l.orientation = Quaternion::rotation_x(s_a.shl.3)
                                * Quaternion::rotation_y(s_a.shl.4);
                            next.hand_r.position = Vec3::new(
                                -s_a.sc.0 + 6.0 + move1 * -12.0,
                                -4.0 + move1 * 3.0,
                                -2.0,
                            );
                            next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                            next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                            next.control.orientation = Quaternion::rotation_x(s_a.sc.3)
                                * Quaternion::rotation_z(move1 * PI / 2.0);

                            next.foot_r.position += Vec3::new(0.0, move1 * -3.0, 0.0);
                            next.foot_r.orientation.rotate_z(move1 * -1.2);
                            next.chest.orientation = Quaternion::rotation_z(move1 * -1.3);
                            next.head.orientation = Quaternion::rotation_z(move1 * 0.7);
                            next.belt.orientation = Quaternion::rotation_z(move1 * 0.4);
                            next.shorts.orientation = Quaternion::rotation_z(move1 * 0.8);
                            next.control.orientation.rotate_y(move1 * -1.5);
                            next.control.orientation.rotate_z(move1 * 0.0);
                            next.control.position += Vec3::new(move1 * 12.0, 0.0, 0.0);

                            next.chest.orientation.rotate_z(move2 * 1.2);
                            next.head.orientation.rotate_z(move2 * -0.7);
                            next.belt.orientation.rotate_z(move2 * -0.3);
                            next.shorts.orientation.rotate_z(move2 * -0.8);
                            next.foot_r.orientation.rotate_z(move2 * 1.2);
                            next.foot_r.orientation.rotate_x(move2 * -0.6);
                            next.control.orientation.rotate_z(move2 * -1.2);
                            next.control.position += Vec3::new(0.0, move2 * 4.0, move2 * 3.0);
                        },
                        1 => {
                            next.control.orientation.rotate_y(move1 * -2.0);
                            next.chest.orientation.rotate_z(move1 * -0.4 + move2 * -1.4);
                            next.control
                                .orientation
                                .rotate_z(move1 * 0.3 + move2 * -1.2);
                            next.head.orientation.rotate_z(move1 * 0.2 + move2 * 0.7);
                            next.belt.orientation.rotate_z(move2 * 0.3);
                            next.shorts.orientation.rotate_z(move1 * 0.2 + move2 * 0.7);
                            next.chest.orientation.rotate_y(move1 * -0.3);
                            next.foot_r.orientation.rotate_z(move2 * -1.5);
                        },
                        _ => {},
                    }
                },
                Some("common.abilities.sword.crippling_gouge") => {
                    let (move1, move2, move3) = if strike == current_strike {
                        match stage_section {
                            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                            Some(StageSection::Action) => (1.0, anim_time.powi(2), 0.0),
                            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                            _ => (0.0, 0.0, 0.0),
                        }
                    } else {
                        (1.0, 1.0, 0.0)
                    };

                    next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                    next.hand_l.orientation =
                        Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                    next.hand_r.position =
                        Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                    next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                    next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                    next.control.orientation = Quaternion::rotation_x(s_a.sc.3);

                    next.chest.orientation = Quaternion::rotation_z(move1 * -1.5);
                    next.head.orientation = Quaternion::rotation_z(move1 * 1.1);
                    next.belt.orientation = Quaternion::rotation_z(move1 * 0.4);
                    next.shorts.orientation = Quaternion::rotation_z(move1 * 1.0);
                    next.control.orientation.rotate_y(move1 * -1.9);
                    next.control.orientation.rotate_z(move1 * 0.7);
                    next.control.position += Vec3::new(move1 * 10.0, 0.0, move1 * 9.0);

                    next.chest.orientation.rotate_z(move2 * 1.4);
                    next.head.orientation.rotate_z(move2 * -0.9);
                    next.belt.orientation.rotate_z(move2 * -0.4);
                    next.shorts.orientation.rotate_z(move2 * -0.9);
                    next.control.orientation.rotate_z(move2 * -1.4);
                    next.control.position += Vec3::new(0.0, move2 * 6.0, move2 * -3.0);
                },
                Some("common.abilities.sword.crippling_strike") => {
                    let (move1, move2, move3) = if strike == current_strike {
                        match stage_section {
                            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                            Some(StageSection::Action) => (1.0, anim_time.powi(2), 0.0),
                            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                            _ => (0.0, 0.0, 0.0),
                        }
                    } else {
                        (1.0, 1.0, 0.0)
                    };

                    let move2alt = move2.min(0.5) * 2.0;
                    let move2 = (move2.max(0.5) - 0.5) * 2.0;

                    next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                    next.hand_l.orientation =
                        Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                    next.hand_r.position =
                        Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                    next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                    next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                    next.control.orientation = Quaternion::rotation_x(s_a.sc.3)
                        * Quaternion::rotation_z((move2alt + move2) * -PI / 4.0);

                    next.chest.orientation = Quaternion::rotation_z(move1 * 1.3)
                        * Quaternion::rotation_x(move2alt * -0.3);
                    next.head.orientation = Quaternion::rotation_z(move1 * -0.8 + move2 * 1.0)
                        * Quaternion::rotation_x(move2alt * 0.1);
                    next.belt.orientation = Quaternion::rotation_z(move1 * -0.4)
                        * Quaternion::rotation_x(move2alt * 0.3);
                    next.shorts.orientation = Quaternion::rotation_z(move1 * -1.0 + move2 * 1.0)
                        * Quaternion::rotation_x(move2alt * 0.5);
                    next.foot_l.orientation = Quaternion::rotation_z(move1 * 0.8);
                    next.foot_l.position += Vec3::new(0.0, move1 * -4.0, 0.0);
                    next.control.orientation.rotate_x(move1 * 0.4);

                    next.foot_r.position += Vec3::new(0.0, move2alt * 4.0, 0.0);
                    next.shorts.position +=
                        Vec3::new(move2alt * 1.0, move2alt * 2.0, move2alt * 0.0);
                    next.control
                        .orientation
                        .rotate_x(move2alt * -0.8 + move2 * -0.6);
                    next.chest.orientation.rotate_z(move2 * -1.7);
                    next.control.orientation.rotate_z(move2 * -1.1);
                    next.control.position += Vec3::new(move2 * 14.0, move2 * 3.0, move2 * 6.0);
                },
                Some("common.abilities.sword.cleaving_combo") => {
                    let (move1, move2, move3) = if strike == current_strike {
                        match stage_section {
                            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                            Some(StageSection::Action) => (1.0, anim_time.powf(0.5), 0.0),
                            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                            _ => (0.0, 0.0, 0.0),
                        }
                    } else {
                        (1.0, 1.0, 0.0)
                    };

                    match strike {
                        0 => {
                            let s1_move1_hack = if current_strike == 1 { move1 } else { 0.0 };
                            next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                            next.hand_l.orientation = Quaternion::rotation_x(s_a.shl.3)
                                * Quaternion::rotation_y(s_a.shl.4);
                            next.hand_r.position = Vec3::new(
                                -s_a.sc.0 + 6.0 + move1 * -12.0,
                                -4.0 + move1 * 3.0,
                                -2.0,
                            );
                            next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                            next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                            next.control.orientation = Quaternion::rotation_x(s_a.sc.3)
                                * Quaternion::rotation_z(move1 * -0.2 + s1_move1_hack * 3.2);

                            next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2);
                            next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2);
                            next.foot_l.orientation = Quaternion::identity();
                            next.foot_r.orientation = Quaternion::identity();

                            next.chest.orientation = Quaternion::rotation_z(move1 * 1.2);
                            next.head.orientation = Quaternion::rotation_z(move1 * -0.7);
                            next.belt.orientation = Quaternion::rotation_z(move1 * -0.4);
                            next.shorts.orientation = Quaternion::rotation_z(move1 * -0.9);
                            next.control.orientation.rotate_x(move1 * 0.4);
                            next.foot_r.position += Vec3::new(0.0, move1 * 2.0, 0.0);
                            next.foot_l.orientation.rotate_z(move1 * 0.6);
                            next.chest.position += Vec3::new(0.0, move1 * -2.0, 0.0);
                            next.foot_l.position += Vec3::new(0.0, move1 * -4.0, 0.0);
                            next.control.orientation.rotate_y(move1 * -1.4);
                            next.chest.orientation.rotate_y(move1 * -0.3);
                            next.belt.orientation.rotate_y(move1 * 0.3);
                            next.shorts.orientation.rotate_y(move1 * 0.35);
                            next.belt.position += Vec3::new(move1 * -1.0, 0., 0.0);
                            next.shorts.position += Vec3::new(move1 * -2.0, move1 * 0.0, 0.0);
                            next.control.position += Vec3::new(0.0, 0.0, move1 * 4.0);

                            next.chest.orientation.rotate_z(move2 * -2.3);
                            next.head.orientation.rotate_z(move2 * 1.5);
                            next.belt.orientation.rotate_z(move2 * 1.2);
                            next.shorts.orientation.rotate_z(move2 * 2.2);
                            next.shorts.orientation.rotate_x(move2 * 0.5);
                            next.belt.orientation.rotate_y(move2 * -0.3);
                            next.belt.orientation.rotate_x(move2 * 0.3);
                            next.belt.position += Vec3::new(0.0, move2 * -1.0, move2 * -1.0);
                            next.shorts.position += Vec3::new(move2 * 0.5, move2 * 0.0, 0.0);
                            next.control.orientation.rotate_z(move2 * -1.8);
                            next.control.position += Vec3::new(move2 * 14.0, 0.0, 0.0);
                        },
                        1 => {
                            next.chest.position += Vec3::new(0.0, move1 * 5.0, 0.0);
                            next.foot_l.position +=
                                Vec3::new(0.0, move1 * 3.0 + move2.powf(0.5) * 6.0, 0.0);
                            next.foot_r.position += Vec3::new(0.0, move1 * -2.0, 0.0);
                            next.foot_r.orientation.rotate_x(move1 * -0.2);
                            next.shorts.orientation.rotate_z(move1 * -0.8);
                            next.shorts.orientation.rotate_x(move1 * 0.3);
                            next.belt.orientation.rotate_z(move1 * -0.3);

                            next.chest.orientation.rotate_z(move2 * 2.5);
                            next.head.orientation.rotate_z(move2 * -2.0);
                            next.belt.orientation.rotate_z(move2 * -0.9);
                            next.shorts.orientation.rotate_z(move2 * -2.1);
                            next.shorts.orientation.rotate_y(move2 * 0.5);
                            next.shorts.orientation.rotate_x(move2 * 0.3);
                            next.belt.orientation.rotate_y(move2 * 0.2);
                            next.belt.position += Vec3::new(0.0, 0.0, move2 * 1.0);
                            next.control.orientation.rotate_z(move2 * 0.9);
                            next.control.position += Vec3::new(move2 * -14.0, 0.0, 0.0);
                        },
                        _ => {},
                    }
                },
                Some("common.abilities.sword.cleaving_spin") => {
                    let (move1, move2, move3) = if strike == current_strike {
                        match stage_section {
                            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                            Some(StageSection::Action) => (1.0, anim_time.powi(2), 0.0),
                            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                            _ => (0.0, 0.0, 0.0),
                        }
                    } else {
                        (1.0, 1.0, 0.0)
                    };

                    let move2_pre = move2.min(0.3) * 10.0 / 3.0;

                    next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                    next.hand_l.orientation =
                        Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                    next.hand_r.position =
                        Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                    next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                    next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                    next.control.orientation = Quaternion::rotation_x(s_a.sc.3);

                    next.chest.orientation = Quaternion::rotation_z(move1 * 1.2);
                    next.head.orientation = Quaternion::rotation_z(move1 * -0.7);
                    next.belt.orientation = Quaternion::rotation_z(move1 * -0.3);
                    next.shorts.orientation = Quaternion::rotation_z(move1 * -0.8);
                    next.control.orientation.rotate_x(move1 * 0.2);
                    next.foot_r
                        .orientation
                        .rotate_x(move1 * -0.4 + move2_pre * 0.4);
                    next.foot_r.orientation.rotate_z(move1 * 1.4);

                    next.control.orientation.rotate_y(move2_pre * -1.6);
                    next.control.position += Vec3::new(0.0, 0.0, move2_pre * 4.0);
                    next.torso.orientation.rotate_z(move2 * -6.0);
                    next.chest.orientation.rotate_z(move2 * -2.0);
                    next.head.orientation.rotate_z(move2 * 1.3);
                    next.belt.orientation.rotate_z(move2 * 0.6);
                    next.shorts.orientation.rotate_z(move2 * 1.5);
                    next.foot_r.orientation.rotate_z(move2_pre * -1.7);
                    next.control.orientation.rotate_z(move2 * -1.8);
                    next.control.position += Vec3::new(move2 * 14.0, 0.0, 0.0);
                },
                Some("common.abilities.sword.defensive_combo") => {
                    let (move1, move2, move3) = if strike == current_strike {
                        match stage_section {
                            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                            Some(StageSection::Action) => (1.0, anim_time.powf(0.5), 0.0),
                            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                            _ => (0.0, 0.0, 0.0),
                        }
                    } else {
                        (1.0, 1.0, 0.0)
                    };

                    match strike {
                        0 => {
                            let s1_move1_hack = if current_strike == 1 { move1 } else { 0.0 };
                            next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                            next.hand_l.orientation = Quaternion::rotation_x(s_a.shl.3)
                                * Quaternion::rotation_y(s_a.shl.4);
                            next.hand_r.position = Vec3::new(
                                -s_a.sc.0 + 6.0 + move1 * -12.0,
                                -4.0 + move1 * 3.0,
                                -2.0,
                            );
                            next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                            next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                            next.control.orientation = Quaternion::rotation_x(s_a.sc.3)
                                * Quaternion::rotation_z(s1_move1_hack * PI);

                            next.chest.orientation = Quaternion::rotation_z(move1 * 0.8);
                            next.head.orientation = Quaternion::rotation_z(move1 * -0.3);
                            next.belt.orientation = Quaternion::rotation_z(move1 * -0.2);
                            next.shorts.orientation = Quaternion::rotation_z(move1 * -0.6);
                            next.control.position += Vec3::new(0.0, 0.0, move1 * 5.0);

                            next.chest.orientation.rotate_z(move2 * -1.9);
                            next.head.orientation.rotate_z(move2 * 1.3);
                            next.belt.orientation.rotate_z(move2 * 0.7);
                            next.shorts.orientation.rotate_z(move2 * 1.5);
                            next.control.orientation.rotate_y(move2 * -1.6);
                            next.control.orientation.rotate_z(move2 * -1.1);
                            next.control.position +=
                                Vec3::new(move2 * 12.0, move2 * 5.0, move2 * -1.0);
                        },
                        1 => {
                            next.belt.orientation.rotate_z(move1 * 0.2);
                            next.shorts.orientation.rotate_z(move1 * 0.3);
                            next.control.position += Vec3::new(0.0, move1 * -5.0, move1 * 1.0);

                            next.chest.orientation.rotate_z(move2 * 2.1);
                            next.head.orientation.rotate_z(move2 * -1.2);
                            next.belt.orientation.rotate_z(move2 * -1.0);
                            next.shorts.orientation.rotate_z(move2 * -1.8);
                            next.control.orientation.rotate_z(move2 * 0.9);
                            next.control.position +=
                                Vec3::new(move2 * -12.0, move2 * 2.0, move2 * -1.0);
                        },
                        _ => {},
                    }
                },
                Some("common.abilities.sword.defensive_retreat") => {
                    let (move1, move2, move3) = if strike == current_strike {
                        match stage_section {
                            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                            Some(StageSection::Action) => (1.0, anim_time.powi(2), 0.0),
                            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(2)),
                            _ => (0.0, 0.0, 0.0),
                        }
                    } else {
                        (1.0, 1.0, 0.0)
                    };

                    let move2_pre = move2.min(0.3) * 10.0 / 3.0;

                    next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                    next.hand_l.orientation =
                        Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                    next.hand_r.position =
                        Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                    next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                    next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                    next.control.orientation = Quaternion::rotation_x(s_a.sc.3);

                    next.chest.orientation = Quaternion::rotation_z(move1 * 0.7);
                    next.shorts.orientation = Quaternion::rotation_z(move1 * -0.3);
                    next.belt.orientation = Quaternion::rotation_z(move1 * -0.1);
                    next.head.orientation = Quaternion::rotation_z(move1 * -0.4);
                    next.foot_l.position += Vec3::new(0.0, move1 * -4.0, 0.0);

                    next.chest.orientation.rotate_z(move2 * -1.5);
                    next.head.orientation.rotate_z(move2 * 0.9);
                    next.belt.orientation.rotate_z(move2 * 0.4);
                    next.shorts.orientation.rotate_z(move2 * 1.0);
                    next.control
                        .orientation
                        .rotate_y(move2 * -1.6 + move3 * 1.6);
                    next.control
                        .orientation
                        .rotate_z(move1 * 0.3 + move2 * -1.5 + move3 * -0.4);
                    next.control.position += Vec3::new(move2 * 12.0 + move3 * -12.0, 0.0, 0.0);
                },
                Some("common.abilities.sword.parrying_combo") => {
                    let (move1, move2, move3) = if strike == current_strike {
                        match stage_section {
                            Some(StageSection::Buildup) => (anim_time.powf(0.5), 0.0, 0.0),
                            Some(StageSection::Action) => {
                                (1.0, (anim_time.min(2.0 / 3.0) * 1.5).powi(2), 0.0)
                            },
                            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                            _ => (0.0, 0.0, 0.0),
                        }
                    } else {
                        (1.0, 1.0, 0.0)
                    };

                    match strike {
                        0 => {
                            next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                            next.hand_l.orientation = Quaternion::rotation_x(s_a.shl.3)
                                * Quaternion::rotation_y(s_a.shl.4);
                            next.hand_r.position = Vec3::new(
                                -s_a.sc.0 + 6.0 + move1 * -12.0,
                                -4.0 + move1 * 3.0,
                                -2.0,
                            );
                            next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                            next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                            next.control.orientation = Quaternion::rotation_x(s_a.sc.3)
                                * Quaternion::rotation_z(move1 * PI / 4.0 + move2 * -PI / 2.0);

                            next.chest.orientation = Quaternion::rotation_z(move1 * 0.8);
                            next.head.orientation = Quaternion::rotation_z(move1 * -0.4);
                            next.belt.orientation = Quaternion::rotation_z(move1 * -0.3);
                            next.shorts.orientation = Quaternion::rotation_z(move1 * -0.6);
                            next.control.orientation.rotate_z(move1 * -1.5);
                            next.control.orientation.rotate_x(move1 * 0.5);
                            next.control.orientation.rotate_y(move1 * 1.5);
                            next.control.position += Vec3::new(0.0, move1 * 5.0, move1 * 12.0);

                            next.chest.orientation.rotate_z(move2 * -1.5);
                            next.head.orientation.rotate_z(move2 * 0.9);
                            next.belt.orientation.rotate_z(move2 * 0.5);
                            next.shorts.orientation.rotate_z(move2 * 1.1);
                            next.control.orientation.rotate_y(move2 * -4.0);
                            next.control.orientation.rotate_z(move2 * -1.0);
                            next.control.position +=
                                Vec3::new(move2 * 12.0, move2 * -6.0, move2 * -9.0);
                        },
                        1 => {
                            next.control.position +=
                                Vec3::new(move1 * -10.0, move1 * 3.0, move1 * 5.0);
                            next.chest.orientation.rotate_z(move1 * 1.8);
                            next.head.orientation.rotate_z(move1 * -1.3);
                            next.belt.orientation.rotate_z(move1 * -0.7);
                            next.shorts.orientation.rotate_z(move1 * -1.4);
                            next.control.orientation.rotate_z(move1.powi(2) * -2.6);
                            next.control.orientation.rotate_x(move1 * 0.3);
                            next.control.orientation.rotate_y(move1 * -0.2);

                            next.control.position +=
                                Vec3::new(move2 * 12.0, move2 * 2.0, move2 * -1.0);
                        },
                        _ => {},
                    }
                },
                Some("common.abilities.sword.parrying_counter") => {
                    let (move1, move2, move3) = match stage_section {
                        Some(StageSection::Buildup) => (anim_time.powf(0.5), 0.0, 0.0),
                        Some(StageSection::Action) => {
                            (1.0, (anim_time.min(2.0 / 3.0) * 1.5).powi(2), 0.0)
                        },
                        Some(StageSection::Recover) => {
                            (1.0, 1.0, (anim_time.min(0.5) * 2.0).powf(0.5))
                        },
                        _ => (0.0, 0.0, 0.0),
                    };

                    next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                    next.hand_l.orientation =
                        Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                    next.hand_r.position =
                        Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                    next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                    next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                    next.control.orientation = Quaternion::rotation_x(s_a.sc.3)
                        * Quaternion::rotation_z(move2 * -PI / 4.0);

                    next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2);
                    next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2);
                    next.foot_l.orientation = Quaternion::identity();
                    next.foot_r.orientation = Quaternion::identity();

                    next.foot_r.position += Vec3::new(
                        0.0,
                        move1 * 4.0 - move3 * 4.0,
                        (1.0 - (move1 - 0.5) * 2.0) * 2.0,
                    );
                    next.torso.position += Vec3::new(0.0, move1 * -2.0 + move3 * 2.0, 0.0);
                    next.chest.position +=
                        Vec3::new(0.0, move1 * 2.0 - move3 * 2.0, move1 * -3.0 + move3 * 3.0);
                    next.shorts.orientation = Quaternion::rotation_x(move1 * 0.5 - move3 * 0.5);
                    next.shorts.position += Vec3::new(0.0, move1 * 1.5 - move3 * 1.5, 0.0);
                    next.control.orientation.rotate_y(move1 * -1.5);
                    next.control.orientation.rotate_z(move1 * 0.8);

                    next.chest.orientation = Quaternion::rotation_z(move2 * -0.7);
                    next.head.orientation = Quaternion::rotation_z(move2 * 0.4);
                    next.shorts.orientation.rotate_z(move2 * 0.5);
                    next.belt.orientation = Quaternion::rotation_z(move2 * 0.1);
                    next.control
                        .orientation
                        .rotate_z(move2 * -1.4 + move3 * -0.5);
                    next.control.orientation.rotate_x(move2 * 0.5 + move3 * 0.2);
                    next.control.position += Vec3::new(move2 * 7.0, 0.0, move2 * 6.0);
                },
                Some("common.abilities.sword.heavy_combo") => {
                    let (move1, move2, move3) = if strike == current_strike {
                        match stage_section {
                            Some(StageSection::Buildup) => {
                                (((anim_time.max(0.4) - 0.4) * 1.5).powf(0.5), 0.0, 0.0)
                            },
                            Some(StageSection::Action) => {
                                (1.0, (anim_time.min(0.4) * 2.5).powi(2), 0.0)
                            },
                            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                            _ => (0.0, 0.0, 0.0),
                        }
                    } else {
                        (1.0, 1.0, 0.0)
                    };

                    match strike {
                        0 => {
                            let fast1 = move1.min(0.2) * 5.0;
                            next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                            next.hand_l.orientation = Quaternion::rotation_x(s_a.shl.3)
                                * Quaternion::rotation_y(s_a.shl.4);
                            next.hand_r.position = Vec3::new(
                                -s_a.sc.0 + 6.0 + fast1 * -12.0,
                                -4.0 + fast1 * 3.0,
                                -2.0,
                            );
                            next.hand_r.orientation = Quaternion::rotation_x(0.9 + fast1 * 0.5);
                            next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                            next.control.orientation = Quaternion::rotation_x(s_a.sc.3);

                            next.control.position +=
                                Vec3::new(move1 * 3.0, move1 * 4.0, move1 * 8.0);
                            next.control.orientation.rotate_x(move1 * 1.0);
                            next.control.orientation.rotate_z(move1 * -0.4);
                            next.chest.orientation = Quaternion::rotation_z(move1 * 0.3);
                            next.head.orientation = Quaternion::rotation_z(move1 * -0.25);
                            next.shorts.orientation = Quaternion::rotation_z(move1 * -0.2);
                            next.belt.orientation = Quaternion::rotation_z(move1 * -0.1);

                            next.chest.orientation.rotate_z(move2 * -0.8);
                            next.head.orientation.rotate_z(move2 * 0.5);
                            next.shorts.orientation.rotate_z(move2 * 0.4);
                            next.belt.orientation.rotate_z(move2 * 0.2);
                            next.control.orientation.rotate_x(move2 * -1.9);
                            next.control.orientation.rotate_z(move2 * 0.1);
                            next.control.position +=
                                Vec3::new(move2 * 4.0, move2 * 2.0, move2 * -9.0);
                        },
                        1 => {
                            next.control.position +=
                                Vec3::new(move1 * 3.0, move1 * -2.0, move1 * 9.0);
                            next.control.orientation.rotate_x(move1 * 1.6);
                            next.control.orientation.rotate_z(move1 * 0.9);
                            next.control.orientation.rotate_y(move1 * 0.6);

                            next.chest.orientation.rotate_z(move2 * 1.1);
                            next.head.orientation.rotate_z(move2 * -0.6);
                            next.shorts.orientation.rotate_z(move2 * -0.8);
                            next.belt.orientation.rotate_z(move2 * -0.2);
                            next.control.position += Vec3::new(move2 * -6.0, 0.0, move2 * -7.0);
                            next.control.orientation.rotate_x(move2 * -2.1);
                            next.control.orientation.rotate_z(move2 * 0.4);
                        },
                        _ => {},
                    }
                },
                _ => {},
            }
        }
        next
    }
}
