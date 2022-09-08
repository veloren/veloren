use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::Hands,
    states::utils::{AbilityInfo, StageSection},
};

pub struct DashAnimation;

type DashAnimationDependency<'a> = (
    (Option<Hands>, Option<Hands>),
    Option<&'a str>,
    f32,
    Option<StageSection>,
    Option<AbilityInfo>,
);
impl Animation for DashAnimation {
    type Dependency<'a> = DashAnimationDependency<'a>;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_dash\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_dash")]
    #[allow(clippy::single_match)] // TODO: Pending review in #587
    fn update_skeleton_inner<'a>(
        skeleton: &Self::Skeleton,
        (_hands, ability_id, _global_time, stage_section, _ability_info): Self::Dependency<'a>,
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
            Some("common.abilities.sword.reaching_charge") => {
                let (move1, move2, move3, move4) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0, 0.0),
                    Some(StageSection::Charge) => (1.0, anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, 1.0, anim_time.powi(2), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time.powi(4)),
                    _ => (0.0, 0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move4;
                let move1 = move1 * pullback;
                let _move2 = move2 * pullback;
                let move3 = move3 * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3);

                next.chest.orientation.rotate_z(move1 * 0.4);
                next.head.orientation.rotate_z(move1 * -0.2);
                next.shorts.orientation.rotate_z(move1 * -0.3);
                next.belt.orientation.rotate_z(move1 * -0.1);
                next.control.orientation.rotate_z(move1 * -0.7);

                next.control.orientation.rotate_x(move3 * -1.1);
                next.chest.orientation.rotate_z(move3 * -1.1);
                next.head.orientation.rotate_z(move3 * 0.4);
                next.shorts.orientation.rotate_z(move3 * 0.5);
                next.belt.orientation.rotate_z(move3 * 0.2);
                next.control.orientation.rotate_z(move3 * 0.9);
                next.control.position += Vec3::new(0.0, move3 * 6.0, 0.0);
            },
            _ => {},
        }

        next
    }
}
