use super::{
    super::{Animation, vek::*},
    GolemSkeleton, SkeletonAttr,
};
use common::states::utils::{AbilityInfo, StageSection};

pub struct ComboAnimation;
impl Animation for ComboAnimation {
    type Dependency<'a> = (
        Option<&'a str>,
        Option<StageSection>,
        Option<AbilityInfo>,
        usize,
        Vec2<f32>,
    );
    type Skeleton = GolemSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"golem_combo\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "golem_combo"))]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_id, stage_section, _ability_info, current_strike, _move_dir): Self::Dependency<'_>,
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
            let (move1, move2) = if strike == current_strike {
                match stage_section {
                    Some(StageSection::Buildup) => {
                        (((anim_time.max(0.4) - 0.4) * 1.5).powf(0.5), 0.0)
                    },
                    Some(StageSection::Action) => (1.0, (anim_time.min(0.4) * 2.5).powi(2)),
                    Some(StageSection::Recover) => (1.0, 1.0),
                    _ => (0.0, 0.0),
                }
            } else {
                (1.0, 1.0)
            };
            let move1 = move1 * multi_strike_pullback;
            let move2 = move2 * multi_strike_pullback;

            match ability_id {
                Some("common.abilities.custom.claygolem.dashstrike") => match strike {
                    0..=2 => {
                        next.head.orientation = Quaternion::rotation_x(-0.2)
                            * Quaternion::rotation_z(move1 * -1.2 + move2 * 2.0);

                        next.upper_torso.orientation = Quaternion::rotation_x(move1 * -0.6)
                            * Quaternion::rotation_z(move1 * 1.2 + move2 * -3.2);

                        next.lower_torso.orientation =
                            Quaternion::rotation_z(move1 * -1.2 + move2 * 3.2)
                                * Quaternion::rotation_x(move1 * 0.6);

                        next.shoulder_l.orientation = Quaternion::rotation_y(move1 * 0.8)
                            * Quaternion::rotation_x(move1 * -1.0 + move2 * 1.6);

                        next.shoulder_r.orientation = Quaternion::rotation_x(move1 * 0.4);

                        next.hand_l.orientation = Quaternion::rotation_z(0.0)
                            * Quaternion::rotation_x(move1 * -1.0 + move2 * 1.8);

                        next.hand_r.orientation = Quaternion::rotation_y(move1 * 0.5)
                            * Quaternion::rotation_x(move1 * 0.4);
                        next.torso.position = Vec3::new(0.0, move1 * 3.7, -8.0 + move2 * 8.0);
                    },
                    _ => {},
                },
                _ => match strike {
                    0..=2 => {
                        next.head.orientation = Quaternion::rotation_x(-0.2)
                            * Quaternion::rotation_z(move1 * -1.2 + move2 * 2.0);

                        next.upper_torso.orientation = Quaternion::rotation_x(move1 * -0.6)
                            * Quaternion::rotation_z(move1 * 1.2 + move2 * -3.2);

                        next.lower_torso.orientation =
                            Quaternion::rotation_z(move1 * -1.2 + move2 * 3.2)
                                * Quaternion::rotation_x(move1 * 0.6);

                        next.shoulder_l.orientation = Quaternion::rotation_y(move1 * 0.8)
                            * Quaternion::rotation_x(move1 * -1.0 + move2 * 1.6);

                        next.shoulder_r.orientation = Quaternion::rotation_x(move1 * 0.4);

                        next.hand_l.orientation = Quaternion::rotation_z(0.0)
                            * Quaternion::rotation_x(move1 * -1.0 + move2 * 1.8);

                        next.hand_r.orientation = Quaternion::rotation_y(move1 * 0.5)
                            * Quaternion::rotation_x(move1 * 0.4);
                        next.torso.position = Vec3::new(0.0, move1 * 3.7, move1 * -1.6);
                    },
                    _ => {},
                },
            }
        }
        next
    }
}
