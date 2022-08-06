use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
use core::f32::consts::PI;

pub struct FinisherMeleeAnimation;
impl Animation for FinisherMeleeAnimation {
    type Dependency<'a> = (Option<&'a str>, Option<StageSection>);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_finisher_melee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_finisher_melee")]
    fn update_skeleton_inner<'a>(
        skeleton: &Self::Skeleton,
        (ability_id, stage_section): Self::Dependency<'a>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();
        next.main_weapon_trail = true;
        next.off_weapon_trail = true;

        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_z(0.0);
        match ability_id {
            Some("common.abilities.sword.balanced_finisher") => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.25).min(1.0), 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time.powf(0.1), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                    _ => (0.0, 0.0, 0.0),
                };

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.control.position = Vec3::new(
                    s_a.sc.0 + move2 * 12.0,
                    s_a.sc.1,
                    s_a.sc.2 + move1 * 6.0 - move2 * 8.0,
                );
                next.control.orientation =
                    Quaternion::rotation_x(s_a.sc.3 + move1 * 1.6 + move2 * -2.6)
                        * Quaternion::rotation_y(move1 * -0.4 + move2 * 0.6)
                        * Quaternion::rotation_z(move1 * -0.2 + move2 * -0.2);

                next.chest.orientation = Quaternion::rotation_z(move1 * 1.0 + move2 * -1.2);
                next.head.orientation = Quaternion::rotation_z(move1 * -0.4 + move2 * 0.3);
                next.belt.orientation = Quaternion::rotation_z(move1 * -0.25 + move2 * 0.2);
                next.shorts.orientation = Quaternion::rotation_z(move1 * -0.5 + move2 * 0.4);
            },
            _ => {},
        }

        next
    }
}
