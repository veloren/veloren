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
                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time.powf(0.1), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                    _ => (0.0, 0.0, 0.0),
                };

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
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
            Some("common.abilities.sword.offensive_finisher") => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time.powi(2), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                    _ => (0.0, 0.0, 0.0),
                };

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3);

                next.chest.orientation = Quaternion::rotation_z(move1 * 0.9);
                next.head.orientation = Quaternion::rotation_z(move1 * -0.4);
                next.belt.orientation = Quaternion::rotation_z(move1 * -0.25);
                next.shorts.orientation = Quaternion::rotation_z(move1 * -0.6);
                next.foot_l.orientation = Quaternion::rotation_z(move1 * 0.8);
                next.foot_l.position += Vec3::new(0.0, move1 * -5.0, 0.0);
                next.control.orientation.rotate_x(move1 * 1.2);
                next.control.position += Vec3::new(0.0, 0.0, move1 * 6.0);

                next.chest.orientation.rotate_z(move2 * -1.7);
                next.head.orientation.rotate_z(move2 * 0.8);
                next.belt.orientation.rotate_z(move2 * 0.5);
                next.shorts.orientation.rotate_z(move2 * 1.1);
                next.foot_l.orientation.rotate_z(move2 * -0.8);
                next.foot_l.position += Vec3::new(0.0, move2 * 5.0, 0.0);
                next.control.orientation.rotate_x(move2 * -2.3);
                next.control.orientation.rotate_z(move2 * 0.6);
                next.control.position += Vec3::new(0.0, 0.0, move2 * -10.0);

                next.chest.orientation.rotate_z(move3 * 0.6);
                next.head.orientation.rotate_z(move3 * -0.2);
                next.shorts.orientation.rotate_z(move3 * -0.2);
                next.control.position += Vec3::new(0.0, 0.0, move3 * 4.0);
                next.control.orientation.rotate_x(move3 * 0.6);
            },
            Some("common.abilities.sword.crippling_finisher") => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.5), 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time.powi(2), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                    _ => (0.0, 0.0, 0.0),
                };

                let move1alt1 = move1.min(0.5) * 2.0;
                let move1alt2 = (move1.max(0.5) - 0.5) * 2.0;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation = Quaternion::rotation_x(s_a.shl.3 + move1alt2 * PI)
                    * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position = Vec3::new(
                    -s_a.sc.0 + 6.0 + move1alt1 * -12.0,
                    -4.0 + move1alt1 * 3.0,
                    -2.0,
                );
                next.hand_r.orientation =
                    Quaternion::rotation_x(0.9 + move1 * 0.5 + move1alt1 * PI);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3);

                next.control.position += Vec3::new(
                    move1 * 6.0,
                    (1.0 - (move1 - 0.5).abs() * 2.0) * 3.0,
                    move1 * 22.0,
                );
                next.control.orientation.rotate_x(move1 * -1.5);

                next.chest.orientation = Quaternion::rotation_x(move2 * -0.4);
                next.head.orientation = Quaternion::rotation_x(move2 * 0.2);
                next.belt.orientation = Quaternion::rotation_x(move2 * 0.4);
                next.shorts.orientation = Quaternion::rotation_x(move2 * 0.8);
                next.control.orientation.rotate_x(move2 * -0.4);
                next.control.position += Vec3::new(0.0, 0.0, move2 * -10.0);
                next.belt.position += Vec3::new(0.0, move2 * 2.0, move2 * 0.0);
                next.shorts.position += Vec3::new(0.0, move2 * 4.0, move2 * 1.0);
                next.chest.position += Vec3::new(0.0, move2 * -2.5, 0.0);
            },
            Some("common.abilities.sword.cleaving_finisher") => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.5), 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time.powi(2), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                    _ => (0.0, 0.0, 0.0),
                };

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation =
                    Quaternion::rotation_x(s_a.sc.3) * Quaternion::rotation_z(move1 * -0.7);

                next.chest.orientation = Quaternion::rotation_z(move1 * 0.4);
                next.head.orientation = Quaternion::rotation_z(move1 * -0.1);
                next.belt.orientation = Quaternion::rotation_z(move1 * -0.1);
                next.shorts.orientation = Quaternion::rotation_z(move1 * -0.3);
                next.control.orientation.rotate_x(move1 * 1.4);
                next.control.position += Vec3::new(move1 * -2.0, 0.0, move1 * 8.0);

                next.chest.orientation.rotate_z(move2 * -0.9);
                next.head.orientation.rotate_z(move2 * 0.5);
                next.belt.orientation.rotate_z(move2 * 0.3);
                next.shorts.orientation.rotate_z(move2 * 0.7);
                next.control.orientation.rotate_x(move2.powf(0.25) * -2.8);
                next.control.position += Vec3::new(move2 * 12.0, 0.0, move2 * -10.0);
            },
            _ => {},
        }

        next
    }
}
