use super::{
    super::{Animation, vek::*},
    CrustaceanSkeleton, SkeletonAttr,
};
use common::states::utils::{AbilityInfo, StageSection};

pub struct ComboAnimation;
impl Animation for ComboAnimation {
    type Dependency<'a> = (
        Option<&'a str>,
        Option<StageSection>,
        Option<AbilityInfo>,
        usize,
        f32,
        Vec3<f32>,
        f32,
    );
    type Skeleton = CrustaceanSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"crustacean_combo\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "crustacean_combo"))]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_id, stage_section, _ability_info, current_strike, global_time, velocity, timer): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        _s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let multi_strike_pullback = 1.0
            - if matches!(stage_section, Some(StageSection::Recover)) {
                anim_time.powi(4)
            } else {
                0.0
            };
        for strike in 0..=current_strike {
            match ability_id {
                Some("common.abilities.custom.crab.triplestrike") => {
                    let (movement1base, movement2base, movement3) = match stage_section {
                        Some(StageSection::Buildup) => (anim_time.sqrt(), 0.0, 0.0),
                        Some(StageSection::Action) => (1.0, anim_time.powi(4), 0.0),
                        Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                        _ => (0.0, 0.0, 0.0),
                    };
                    let pullback = multi_strike_pullback;
                    let subtract = global_time - timer;
                    let check = subtract - subtract.trunc();
                    let mirror = (check - 0.5).signum();
                    let twitch3 = (mirror * movement3 * 9.0).sin();
                    let _movement1 = mirror * movement1base * pullback;
                    let _movement2 = mirror * movement2base * pullback;
                    let movement1abs = movement1base * pullback;
                    let movement2abs = movement2base * pullback;
                    let mirror_var = if strike == 1 { -mirror } else { mirror };
                    if velocity.xy().magnitude() > 0.1 {
                        next.chest.orientation = Quaternion::rotation_z(0.0);
                    }

                    next.chest.orientation = Quaternion::rotation_x(
                        movement1abs * 0.3 + movement2abs * -0.2 + (twitch3 / 5.0),
                    );
                    next.arm_r.orientation =
                        Quaternion::rotation_z(movement1abs * -0.8 + movement2abs * 1.0)
                            * Quaternion::rotation_x(movement1abs * 0.4 + movement2abs * 0.3);
                    next.arm_r.position = Vec3::new(
                        0.0,
                        7.0 * movement1abs - 3.0 * movement2abs,
                        -0.8 * mirror_var,
                    );
                    next.pincer_r1.position =
                        Vec3::new(0.0, -3.0 * movement1abs + 4.0 * movement2abs, 0.0);

                    next.arm_l.orientation =
                        Quaternion::rotation_z(movement1abs * 0.8 + movement2abs * -1.0)
                            * Quaternion::rotation_x(movement1abs * 0.4 + movement2abs * 0.3);
                    next.arm_l.position = Vec3::new(
                        0.0,
                        7.0 * movement1abs - 3.0 * movement2abs,
                        0.8 * mirror_var,
                    );
                    next.pincer_l1.position =
                        Vec3::new(0.0, -3.0 * movement1abs + 4.0 * movement2abs, 0.0);
                },
                Some("common.abilities.custom.karkatha.triplestrike") => {
                    let (movement1base, movement2base, movement3) = match stage_section {
                        Some(StageSection::Buildup) => (anim_time.sqrt(), 0.0, 0.0),
                        Some(StageSection::Action) => (1.0, anim_time.powi(4), 0.0),
                        Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                        _ => (0.0, 0.0, 0.0),
                    };
                    let pullback = multi_strike_pullback;
                    let subtract = global_time - timer;
                    let check = subtract - subtract.trunc();
                    let mirror = (check - 0.5).signum();
                    let twitch3 = (mirror * movement3 * 9.0).sin();
                    let _movement1 = mirror * movement1base * pullback;
                    let _movement2 = mirror * movement2base * pullback;
                    let movement1abs = movement1base * pullback;
                    let movement2abs = movement2base * pullback;
                    if velocity.xy().magnitude() > 0.1 {
                        next.chest.orientation = Quaternion::rotation_z(0.0);
                    }

                    next.chest.orientation = Quaternion::rotation_x(
                        movement1abs * 0.3 + movement2abs * -0.2 + (twitch3 / 5.0),
                    );
                    if mirror < 0.0 {
                        next.arm_r.orientation =
                            Quaternion::rotation_z(movement1abs * 0.6 + movement2abs * -1.8);

                        next.pincer_r1.position =
                            Vec3::new(0.0, -3.0 * movement1abs + 4.0 * movement2abs, 4.0);
                        next.pincer_r1.orientation =
                            Quaternion::rotation_x(movement1abs * -0.4 + movement2abs * 0.3);
                    } else {
                        next.arm_l.orientation =
                            Quaternion::rotation_z(movement1abs * -0.6 + movement2abs * 1.6);
                        next.pincer_l1.position =
                            Vec3::new(0.0, -3.0 * movement1abs + 4.0 * movement2abs, 4.0);
                        next.pincer_l1.orientation =
                            Quaternion::rotation_x(movement1abs * -0.4 + movement2abs * 0.3);
                        next.pincer_l0.position = Vec3::new(0.0, -4.0, 0.0);
                        next.pincer_l0.orientation =
                            Quaternion::rotation_x(movement1abs * 0.4 + movement2abs * -0.6);
                    }
                },
                _ => {},
            }
        }
        next
    }
}
