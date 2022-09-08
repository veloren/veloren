use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
use std::ops::{Mul, Sub};

pub struct RapidMeleeAnimation;
impl Animation for RapidMeleeAnimation {
    type Dependency<'a> = (Option<&'a str>, Option<StageSection>, (u32, u32));
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_rapid_melee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_rapid_melee")]
    fn update_skeleton_inner<'a>(
        skeleton: &Self::Skeleton,
        (ability_id, stage_section, (current_strike, max_strikes)): Self::Dependency<'a>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_z(0.0);
        next.main_weapon_trail = true;

        match ability_id {
            Some("common.abilities.sword.reaching_flurry") => {
                let (move1, move2, move3, move2alt) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0, 0.0),
                    Some(StageSection::Action) => (
                        1.0,
                        anim_time.min(0.5).mul(2.0).powi(2) - anim_time.max(0.5).sub(0.5).mul(2.0),
                        0.0,
                        anim_time.powi(2),
                    ),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4), 1.0),
                    _ => (0.0, 0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3);

                next.chest.orientation = Quaternion::rotation_z(move1 * 0.7);
                next.head.orientation = Quaternion::rotation_z(move1 * -0.4);
                next.shorts.orientation = Quaternion::rotation_z(move1 * -0.5);
                next.belt.orientation = Quaternion::rotation_z(move1 * -0.2);
                next.control.orientation.rotate_x(move1 * -1.1);
                next.control.orientation.rotate_z(move1 * -0.7);
                next.control.position += Vec3::new(move1 * 1.0, move1 * -1.0, move1 * 4.0);

                next.chest.orientation.rotate_z(move2 * -1.2);
                next.head.orientation.rotate_z(move2 * 0.6);
                next.belt.orientation.rotate_z(move2 * 0.3);
                next.shorts.orientation.rotate_z(move2 * 0.7);
                next.control.orientation.rotate_z(move2 * 1.2);
                next.control.position += Vec3::new(0.0, move2 * 12.0, 0.0);

                if current_strike == max_strikes {
                    next.control.position += Vec3::new(move2alt * -6.0, move2alt * -6.0, 0.0);
                }
            },
            _ => {},
        }

        next
    }
}
