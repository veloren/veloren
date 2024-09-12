use super::{
    super::{vek::*, Animation},
    quadruped_low_alpha, quadruped_low_beta, QuadrupedLowSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct ComboAnimation;
impl Animation for ComboAnimation {
    type Dependency<'a> = (Option<&'a str>, StageSection, usize, f32, f32);
    type Skeleton = QuadrupedLowSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_low_combo\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_low_combo")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_id, stage_section, current_strike, global_time, timer): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let _multi_strike_pullback = 1.0
            - if matches!(stage_section, StageSection::Recover) {
                anim_time.powi(4)
            } else {
                0.0
            };

        for strike in 0..=current_strike {
            match ability_id {
                Some("common.abilities.custom.hydra.multi_bite") => {
                    let (movement1base, movement2base, movement3) = match stage_section {
                        StageSection::Buildup => (anim_time.sqrt(), 0.0, 0.0),
                        StageSection::Action => (1.0, anim_time.powi(4), 0.0),
                        StageSection::Recover => (1.0, 1.0, anim_time),
                        _ => (0.0, 0.0, 0.0),
                    };
                    let pullback = 1.0 - movement3;
                    let subtract = global_time - timer;
                    let check = subtract - subtract.trunc();
                    let mirror = (check - 0.5).signum();
                    let twitch3 = (mirror * movement3 * 9.0).sin();
                    let movement1 = mirror * movement1base * pullback;
                    let movement2 = mirror * movement2base * pullback;
                    let movement1abs = movement1base * pullback;
                    let movement2abs = movement2base * pullback;

                    match strike {
                        2 => {
                            next.head_l_upper.orientation = Quaternion::rotation_z(twitch3 * -0.7);

                            next.head_l_lower.orientation =
                                Quaternion::rotation_z(movement1abs * 0.5 + movement2abs * 0.2)
                                    * Quaternion::rotation_y(
                                        movement1abs * -0.4 + movement2abs * -0.2,
                                    )
                                    * Quaternion::rotation_x(
                                        movement1abs * 0.35 + movement2abs * -0.9,
                                    );

                            next.jaw_l.orientation =
                                Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * 0.5);

                            next.head_c_upper.orientation = Quaternion::rotation_z(twitch3 * -0.7);

                            next.head_c_lower.orientation =
                                Quaternion::rotation_x(movement1abs * 0.35 + movement2abs * -0.9);

                            next.jaw_c.orientation =
                                Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * 0.5);

                            next.head_r_upper.orientation = Quaternion::rotation_z(twitch3 * -0.7);

                            next.head_r_lower.orientation =
                                Quaternion::rotation_z(movement1abs * -0.5 + movement2abs * -0.2)
                                    * Quaternion::rotation_y(
                                        movement1abs * 0.4 + movement2abs * 0.2,
                                    )
                                    * Quaternion::rotation_x(
                                        movement1abs * 0.35 + movement2abs * -0.9,
                                    );

                            next.jaw_r.orientation =
                                Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * 0.5);

                            next.chest.orientation =
                                Quaternion::rotation_z(movement1 * -0.2 + movement2 * 0.6);

                            next.tail_front.orientation = Quaternion::rotation_x(0.25)
                                * Quaternion::rotation_z(movement1 * -1.0 + movement2 * 2.2);

                            next.tail_rear.orientation = Quaternion::rotation_x(-0.12)
                                * Quaternion::rotation_z(movement1 * -0.6 + movement2 * 0.6);
                        },
                        0 | 1 => {
                            let dir = match strike {
                                0 => 1.0,
                                _ => -1.0,
                            };
                            next.head_l_upper.orientation = Quaternion::rotation_z(twitch3 * 0.2);

                            next.head_l_lower.orientation =
                                Quaternion::rotation_x(movement1abs * 0.4 + movement2abs * -1.2);

                            next.jaw_l.orientation =
                                Quaternion::rotation_x(movement1abs * -0.9 + movement2abs * 0.9);

                            next.head_c_upper.orientation = Quaternion::rotation_z(twitch3 * 0.2);

                            next.head_c_lower.orientation =
                                Quaternion::rotation_x(movement1abs * 0.5 + movement2abs * -1.5);

                            next.jaw_c.orientation =
                                Quaternion::rotation_x(movement1abs * -0.9 + movement2abs * 0.9);

                            next.head_r_upper.orientation = Quaternion::rotation_z(twitch3 * 0.2);

                            next.head_r_lower.orientation =
                                Quaternion::rotation_x(movement1abs * 0.4 + movement2abs * -1.2);

                            next.jaw_r.orientation =
                                Quaternion::rotation_x(movement1abs * -0.9 + movement2abs * 0.9);

                            next.chest.orientation = Quaternion::rotation_z(
                                movement1 * 0.2 * dir + movement2 * -0.3 * dir,
                            );

                            next.tail_front.orientation = Quaternion::rotation_x(0.15)
                                * Quaternion::rotation_z(movement1 * 0.4 + movement2 * 0.2);

                            next.tail_rear.orientation = Quaternion::rotation_x(-0.12)
                                * Quaternion::rotation_z(movement1 * 0.4 + movement2 * 0.2);
                        },
                        _ => {},
                    }
                },
                Some(
                    "common.abilities.custom.icedrake.multi_bite"
                    | "common.abilities.custom.icedrake.icy_bite"
                    | "common.abilities.custom.driggle.bite",
                ) => {
                    let (movement1base, movement2base, movement3) = match stage_section {
                        StageSection::Buildup => (anim_time.sqrt(), 0.0, 0.0),
                        StageSection::Action => (1.0, anim_time.powi(4), 0.0),
                        StageSection::Recover => (1.0, 1.0, anim_time),
                        _ => (0.0, 0.0, 0.0),
                    };
                    let pullback = 1.0 - movement3;
                    let subtract = global_time - timer;
                    let check = subtract - subtract.trunc();
                    let mirror = (check - 0.5).signum();
                    let twitch3 = (mirror * movement3 * 9.0).sin();
                    let movement1 = mirror * movement1base * pullback;
                    let movement2 = mirror * movement2base * pullback;
                    let movement1abs = movement1base * pullback;
                    let movement2abs = movement2base * pullback;

                    match strike {
                        0 | 2 => {
                            next.head_c_upper.orientation = Quaternion::rotation_z(twitch3 * -0.7);

                            next.head_c_lower.orientation =
                                Quaternion::rotation_x(movement1abs * 0.35 + movement2abs * -0.9)
                                    * Quaternion::rotation_y(movement1 * 0.7 + movement2 * -1.0);

                            next.jaw_c.orientation =
                                Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * 0.5);
                            next.chest.orientation =
                                Quaternion::rotation_y(movement1 * -0.08 + movement2 * 0.15)
                                    * Quaternion::rotation_z(movement1 * -0.2 + movement2 * 0.6);

                            next.tail_front.orientation = Quaternion::rotation_x(0.15)
                                * Quaternion::rotation_z(movement1 * -0.4 + movement2 * -0.2);

                            next.tail_rear.orientation = Quaternion::rotation_x(-0.12)
                                * Quaternion::rotation_z(movement1 * -0.4 + movement2 * -0.2);
                        },
                        1 => {
                            next.head_c_upper.orientation = Quaternion::rotation_z(twitch3 * 0.2);

                            next.head_c_lower.orientation =
                                Quaternion::rotation_x(movement1abs * 0.15 + movement2abs * -0.6)
                                    * Quaternion::rotation_y(movement1 * -0.1 + movement2 * 0.15);

                            next.jaw_c.orientation =
                                Quaternion::rotation_x(movement1abs * -0.9 + movement2abs * 0.9);
                            next.chest.orientation =
                                Quaternion::rotation_y(movement1 * 0.08 + movement2 * -0.15)
                                    * Quaternion::rotation_z(movement1 * 0.2 + movement2 * -0.3);

                            next.tail_front.orientation = Quaternion::rotation_x(0.15)
                                * Quaternion::rotation_z(movement1 * 0.4 + movement2 * 0.2);

                            next.tail_rear.orientation = Quaternion::rotation_x(-0.12)
                                * Quaternion::rotation_z(movement1 * 0.4 + movement2 * 0.2);
                        },
                        _ => {},
                    }
                },
                Some(
                    "common.abilities.custom.asp.singlestrike"
                    | "common.abilities.custom.maneater.singlestrike"
                    | "common.abilities.custom.quadlowbasic.singlestrike",
                ) => {
                    quadruped_low_alpha(
                        &mut next,
                        s_a,
                        stage_section,
                        anim_time,
                        global_time,
                        timer,
                    );
                },
                Some(
                    "common.abilities.custom.basilisk.triplestrike"
                    | "common.abilities.custom.quadlowbasic.triplestrike"
                    | "common.abilities.custom.quadlowbreathe.triplestrike"
                    | "common.abilities.custom.quadlowtail.triplestrike"
                    | "common.abilities.custom.rocksnapper.triplestrike",
                ) => match strike {
                    0 | 2 => {
                        quadruped_low_alpha(
                            &mut next,
                            s_a,
                            stage_section,
                            anim_time,
                            global_time,
                            timer,
                        );
                    },
                    1 => {
                        quadruped_low_beta(
                            &mut next,
                            s_a,
                            stage_section,
                            anim_time,
                            global_time,
                            timer,
                        );
                    },
                    _ => {},
                },
                Some("common.abilities.custom.quadlowquick.quadstrike") => match strike {
                    0 | 2 | 3 => {
                        quadruped_low_alpha(
                            &mut next,
                            s_a,
                            stage_section,
                            anim_time,
                            global_time,
                            timer,
                        );
                    },
                    1 => {
                        quadruped_low_beta(
                            &mut next,
                            s_a,
                            stage_section,
                            anim_time,
                            global_time,
                            timer,
                        );
                    },
                    _ => {},
                },
                Some("common.abilities.custom.dwarves.snaretongue.tongue") => {
                    let (movement1base, movement2base, movement3) = match stage_section {
                        StageSection::Buildup => (anim_time.sqrt(), 0.0, 0.0),
                        StageSection::Action => (1.0, anim_time.powi(4), 0.0),
                        StageSection::Recover => (1.0, 1.0, anim_time),
                        _ => (0.0, 0.0, 0.0),
                    };
                    let pullback = 1.0 - movement3;
                    let subtract = global_time - timer;
                    let check = subtract - subtract.trunc();
                    let mirror = (check - 0.5).signum();
                    let twitch3 = (mirror * movement3 * 9.0).sin();
                    let movement1 = mirror * movement1base * pullback;
                    let movement2 = mirror * movement2base * pullback;
                    let movement1abs = movement1base * pullback;
                    let movement2abs = movement2base * pullback;

                    next.head_c_upper.orientation = Quaternion::rotation_z(twitch3 * -0.7);
                    next.head_c_lower.orientation =
                        Quaternion::rotation_x(movement1abs * 0.35 + movement2abs * -0.4)
                            * Quaternion::rotation_y(movement1 * 0.7 + movement2 * -0.7);

                    next.jaw_c.orientation =
                        Quaternion::rotation_x(movement2abs * -0.8 + movement3 * -0.6);
                    next.chest.orientation =
                        Quaternion::rotation_y(movement1 * -0.08 + movement2 * 0.15)
                            * Quaternion::rotation_z(movement1 * -0.2 + movement2 * 0.6);

                    next.tail_front.position = Vec3::new(
                        0.0,
                        s_a.tail_front.0 + (4.0 * s_a.tail_front.0 * movement2abs),
                        s_a.tail_front.1,
                    );
                    next.tail_rear.position =
                        Vec3::new(0.0, 3.0 * s_a.tail_rear.0 * movement2abs, s_a.tail_rear.1);
                    next.tail_front.orientation = Quaternion::rotation_x(movement3 * 0.15);
                },
                _ => {},
            }
        }
        next
    }
}
