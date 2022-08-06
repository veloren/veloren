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
        next.main_weapon_trail = true;
        next.off_weapon_trail = true;
        for strike in 0..=current_strike {
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

            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_z(0.0);
            match ability_id {
                Some("common.abilities.sword.balanced_combo") => match strike {
                    0 => {
                        next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                        next.hand_l.orientation =
                            Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                        next.chest.orientation =
                            Quaternion::rotation_z(move1 * 0.3 + move2alt * -1.0);
                        next.head.orientation =
                            Quaternion::rotation_z(move1 * -0.15 + move2alt * 0.5);
                        next.belt.orientation =
                            Quaternion::rotation_z(move1 * -0.2 + move2alt * 0.5);
                        next.shorts.orientation =
                            Quaternion::rotation_z(move1 * -0.25 + move2alt * 0.7);
                        next.hand_r.position =
                            Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
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
                },
                _ => {},
            }
        }
        next
    }
}
