use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
use core::f32::consts::PI;

pub struct SelfBuffAnimation;
impl Animation for SelfBuffAnimation {
    type Dependency<'a> = (Option<&'a str>, Option<StageSection>);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_self_buff\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_self_buff")]
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
            Some("common.abilities.sword.defensive_bulwark") => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Movement) => (anim_time.powf(0.25), 0.0, 0.0),
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
                    Quaternion::rotation_x(s_a.sc.3) * Quaternion::rotation_z(move1 * -PI / 2.0);

                next.foot_r.position += Vec3::new(move1 * 2.0, move1 * -3.0, 0.0);
                next.foot_r.orientation.rotate_z(move1 * -1.5);
                next.chest.orientation = Quaternion::rotation_z(move1 * -0.6);
                next.head.orientation = Quaternion::rotation_z(move1 * 0.3);
                next.belt.orientation = Quaternion::rotation_z(move1 * -0.1);
                next.shorts.orientation = Quaternion::rotation_z(move1 * -0.3);
                next.control.orientation.rotate_x(move1 * 0.4);
                next.control.orientation.rotate_y(move1 * -0.8);
                next.control.position += Vec3::new(move1 * 12.0, 0.0, 0.0);
                next.hand_l.orientation.rotate_y(move1 * PI);
                next.hand_l.position += Vec3::new(0.0, 0.0, move1 * 7.0);

                next.hand_l.position += Vec3::new(0.0, 0.0, move2 * 20.0);
            },
            _ => {},
        }

        next
    }
}
