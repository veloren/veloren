use super::{
    super::{vek::*, Animation},
    biped_small_alpha_axe, biped_small_alpha_dagger, biped_small_alpha_spear,
    biped_small_wield_spear, init_biped_small_alpha, BipedSmallSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct ComboAnimation;
impl Animation for ComboAnimation {
    type Dependency<'a> = (Option<&'a str>, StageSection, usize, Vec3<f32>, f32, f32);
    type Skeleton = BipedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_small_combo\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_small_combo")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_id, stage_section, current_strike, velocity, _global_time, _timer): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_z(0.0);
        let multi_strike_pullback = 1.0
            - if matches!(stage_section, StageSection::Recover) {
                anim_time.powi(4)
            } else {
                0.0
            };

        for strike in 0..=current_strike {
            match ability_id {
                Some(
                    "common.abilities.custom.bushly.singlestrike"
                    | "common.abilities.custom.irrwurz.singlestrike"
                    | "common.abilities.custom.husk.singlestrike"
                    | "common.abilities.custom.husk.triplestrike"
                    | "common.abilities.custom.dwarves.clockwork.singlestrike"
                    | "common.abilities.custom.dwarves.clockwork.triplestrike",
                ) => {
                    let (move1, move2) = if strike == current_strike {
                        match stage_section {
                            StageSection::Buildup => {
                                (((anim_time.max(0.4) - 0.4) * 1.5).powf(0.5), 0.0)
                            },
                            StageSection::Action => (1.0, (anim_time.min(0.4) * 2.5).powi(2)),
                            StageSection::Recover => (1.0, 1.0),
                            _ => (0.0, 0.0),
                        }
                    } else {
                        (1.0, 1.0)
                    };
                    let move1 = move1 * multi_strike_pullback;
                    let move2 = move2 * multi_strike_pullback;
                    next.hand_l.position = Vec3::new(s_a.grip.0 * 4.0, 0.0, s_a.grip.2);
                    next.hand_r.position = Vec3::new(-s_a.grip.0 * 4.0, 0.0, s_a.grip.2);
                    next.main.position = Vec3::new(0.0, 0.0, 0.0);
                    next.main.orientation = Quaternion::rotation_x(0.0);
                    next.hand_l.orientation = Quaternion::rotation_x(0.0);
                    next.hand_r.orientation = Quaternion::rotation_x(0.0);

                    match strike {
                        0..=2 => {
                            next.chest.orientation = Quaternion::rotation_x(move2 * -1.0)
                                * Quaternion::rotation_z(move1 * 1.2 + move2 * -1.8);
                            next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2);
                            next.hand_l.orientation = Quaternion::rotation_x(1.2);
                            next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2);
                            next.hand_r.orientation = Quaternion::rotation_x(1.2);
                        },
                        _ => {},
                    }
                },
                Some(
                    "common.abilities.axesimple.doublestrike"
                    | "common.abilities.custom.boreal_warrior.hammer.singlestrike",
                ) => {
                    let anim_time = anim_time.min(1.0);
                    let (move1base, move2base, move3) = match stage_section {
                        StageSection::Buildup => (anim_time.sqrt(), 0.0, 0.0),
                        StageSection::Action => (1.0, anim_time.powi(4), 0.0),
                        StageSection::Recover => (1.0, 1.0, anim_time),
                        _ => (0.0, 0.0, 0.0),
                    };
                    let pullback = 1.0 - move3;
                    let move1abs = move1base * pullback;
                    let move2abs = move2base * pullback;

                    init_biped_small_alpha(&mut next, s_a);
                    biped_small_alpha_axe(&mut next, s_a, move1abs, move2abs);
                },
                Some("common.abilities.daggersimple.singlestrike") => {
                    let anim_time = anim_time.min(1.0);
                    let (move1base, move2base, move3) = match stage_section {
                        StageSection::Buildup => (anim_time.sqrt(), 0.0, 0.0),
                        StageSection::Action => (1.0, anim_time.powi(4), 0.0),
                        StageSection::Recover => (1.0, 1.0, anim_time),
                        _ => (0.0, 0.0, 0.0),
                    };
                    let pullback = 1.0 - move3;
                    let move1abs = move1base * pullback;
                    let move2abs = move2base * pullback;

                    init_biped_small_alpha(&mut next, s_a);
                    biped_small_alpha_dagger(&mut next, s_a, move1abs, move2abs);
                },
                Some("common.abilities.spear.doublestrike") => {
                    let anim_time = anim_time.min(1.0);
                    let speed = Vec2::<f32>::from(velocity).magnitude();
                    let speednorm = speed / 9.4;
                    let speednormcancel = 1.0 - speednorm;

                    let (move1base, move2base, move3) = match stage_section {
                        StageSection::Buildup => (anim_time.sqrt(), 0.0, 0.0),
                        StageSection::Action => (1.0, anim_time.powi(4), 0.0),
                        StageSection::Recover => (1.0, 1.0, anim_time),
                        _ => (0.0, 0.0, 0.0),
                    };
                    let pullback = 1.0 - move3;
                    let move1abs = move1base * pullback;
                    let move2abs = move2base * pullback;

                    init_biped_small_alpha(&mut next, s_a);
                    biped_small_alpha_spear(
                        &mut next,
                        s_a,
                        move1abs,
                        move2abs,
                        anim_time,
                        speednormcancel,
                    );
                },
                Some("common.abilities.haniwa.guard.backpedal") => {
                    init_biped_small_alpha(&mut next, s_a);
                    biped_small_wield_spear(&mut next, s_a, anim_time, 0.0, 0.0);

                    let (move1, move2, move3) = match stage_section {
                        StageSection::Buildup => (anim_time.powf(0.25), 0.0, 0.0),
                        StageSection::Action => (1.0, anim_time, 0.0),
                        StageSection::Recover => (1.0, 1.0, anim_time.powf(0.25)),
                        _ => (0.0, 0.0, 0.0),
                    };
                    let pullback = 1.0 - move3;
                    let move1 = move1 * pullback;
                    let move2 = move2 * pullback;

                    biped_small_alpha_spear(&mut next, s_a, move1, move2, anim_time, 0.0);
                },
                _ => {},
            }
        }
        next
    }
}
