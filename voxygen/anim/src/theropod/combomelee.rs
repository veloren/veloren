use super::{
    super::{vek::*, Animation},
    SkeletonAttr, TheropodSkeleton,
};
use common::states::utils::StageSection;

pub struct ComboAnimation;

impl Animation for ComboAnimation {
    type Dependency<'a> = (Option<&'a str>, StageSection, usize, f32, f32);
    type Skeleton = TheropodSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"theropod_combo\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "theropod_combo")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_ability_id, stage_section, current_strike, global_time, timer): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        _s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let multi_strike_pullback = 1.0
            - if matches!(stage_section, StageSection::Recover) {
                anim_time.powi(4)
            } else {
                0.0
            };

        for strike in 0..=current_strike {
            match strike {
                0 => {
                    let (movement1base, movement2base) = match stage_section {
                        StageSection::Buildup => (anim_time.powi(2), 0.0),
                        StageSection::Action => (1.0, anim_time.powi(4)),
                        _ => (0.0, 0.0),
                    };
                    let subtract = global_time - timer;
                    let check = subtract - subtract.trunc();
                    let mirror = (check - 0.5).signum();
                    let movement1 = mirror * movement1base * multi_strike_pullback;
                    let movement2 = mirror * movement2base * multi_strike_pullback;
                    let movement1abs = movement1base * multi_strike_pullback;
                    let movement2abs = movement2base * multi_strike_pullback;

                    next.head.orientation = Quaternion::rotation_x(movement1abs * 0.2)
                        * Quaternion::rotation_y(movement1 * 0.1 + movement2 * 0.2);
                    next.neck.orientation = Quaternion::rotation_x(movement1abs * -0.3)
                        * Quaternion::rotation_y(movement1 * 0.1 + movement2 * 0.1);

                    next.jaw.orientation =
                        Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * 0.5);

                    next.chest_front.orientation = Quaternion::rotation_x(movement1abs * -0.2);
                    next.chest_back.orientation = Quaternion::rotation_x(movement1abs * 0.2);

                    next.leg_l.orientation = Quaternion::rotation_x(movement1abs * -0.1);

                    next.leg_r.orientation = Quaternion::rotation_x(movement1abs * -0.1);
                    next.foot_l.orientation = Quaternion::rotation_x(movement1abs * -0.3);
                    next.foot_r.orientation = Quaternion::rotation_x(movement1abs * -0.3);

                    next.tail_front.orientation =
                        Quaternion::rotation_x(0.1 + movement1abs * -0.1 + movement2abs * -0.3)
                            * Quaternion::rotation_z(movement1 * -0.1 + movement2 * -0.2);

                    next.tail_back.orientation =
                        Quaternion::rotation_x(0.1 + movement1abs * -0.1 + movement2abs * -0.3)
                            * Quaternion::rotation_z(movement1 * -0.1 + movement2 * -0.2);
                },
                1 | 2 => {
                    let (movement1base, movement2base) = match stage_section {
                        StageSection::Buildup => (anim_time.powi(2), 0.0),
                        StageSection::Action => (1.0, anim_time.powi(4)),
                        _ => (0.0, 0.0),
                    };
                    let subtract = global_time - timer;
                    let check = subtract - subtract.trunc();
                    let mirror = (check - 0.5).signum();
                    let movement1 = mirror * movement1base * multi_strike_pullback;
                    let movement2 = mirror * movement2base * multi_strike_pullback;
                    let movement1abs = movement1base * multi_strike_pullback;
                    let movement2abs = movement2base * multi_strike_pullback;

                    next.head.orientation =
                        Quaternion::rotation_x(movement1abs * -0.4 + movement2abs * 1.2)
                            * Quaternion::rotation_y(movement1 * 0.1 + movement2 * -0.1);
                    next.neck.orientation =
                        Quaternion::rotation_x(movement1abs * 0.4 + movement2abs * -1.2)
                            * Quaternion::rotation_y(movement1 * 0.1 + movement2 * -0.1);

                    next.chest_front.orientation =
                        Quaternion::rotation_x(movement1abs * 0.6 + movement2abs * -1.5);
                    next.chest_back.orientation =
                        Quaternion::rotation_x(movement1abs * -0.6 + movement2abs * 1.5);

                    next.leg_l.orientation = Quaternion::rotation_x(movement1abs * -0.5);

                    next.leg_r.orientation = Quaternion::rotation_x(movement1abs * -0.5);
                    next.foot_l.orientation = Quaternion::rotation_x(movement1abs * 0.4);
                    next.foot_r.orientation = Quaternion::rotation_x(movement1abs * 0.4);

                    next.tail_front.orientation =
                        Quaternion::rotation_x(0.1 + movement1abs * -0.1 + movement2abs * -0.3);

                    next.tail_back.orientation =
                        Quaternion::rotation_x(0.1 + movement1abs * -0.1 + movement2abs * -0.3);
                },
                _ => {},
            }
        }

        next
    }
}
