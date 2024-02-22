use super::{
    super::{vek::*, Animation},
    dual_wield_start, hammer_start, twist_back, twist_forward, CharacterSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct DashAnimation;

type DashAnimationDependency<'a> = (Option<&'a str>, StageSection);
impl Animation for DashAnimation {
    type Dependency<'a> = DashAnimationDependency<'a>;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_dash\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_dash")]
    #[allow(clippy::single_match)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_id, stage_section): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        if matches!(stage_section, StageSection::Action | StageSection::Charge) {
            next.main_weapon_trail = true;
            next.off_weapon_trail = true;
        }

        match ability_id {
            Some("common.abilities.hammer.intercept") => {
                hammer_start(&mut next, s_a);
                let (move1, _move2, move3, move4) = match stage_section {
                    StageSection::Buildup => (anim_time, 0.0, 0.0, 0.0),
                    StageSection::Charge => (1.0, anim_time, 0.0, 0.0),
                    StageSection::Action => (1.0, 0.0, anim_time, 0.0),
                    StageSection::Recover => (1.0, 0.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move4;
                let move1 = move1 * pullback;
                let move3 = move3 * pullback;

                twist_back(&mut next, move1, 1.6, 0.7, 0.3, 1.1);
                next.control.orientation.rotate_x(move1 * 1.8);

                twist_forward(&mut next, move3, 2.4, 0.9, 0.5, 1.4);
                next.control.orientation.rotate_z(move3 * -2.7);
                next.control.orientation.rotate_x(move3 * 2.0);
                next.control.position += Vec3::new(5.0, 0.0, 11.0) * move3;
            },
            Some("common.abilities.hammer.dual_intercept") => {
                dual_wield_start(&mut next);
                let (move1, _move2, move3, move4) = match stage_section {
                    StageSection::Buildup => (anim_time, 0.0, 0.0, 0.0),
                    StageSection::Charge => (1.0, anim_time, 0.0, 0.0),
                    StageSection::Action => (1.0, 0.0, anim_time, 0.0),
                    StageSection::Recover => (1.0, 0.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move4;
                let move1 = move1 * pullback;
                let move3 = move3 * pullback;

                next.control_l.orientation.rotate_x(move1 * -1.4);
                next.control_l.orientation.rotate_z(move1 * 0.8);
                next.control_r.orientation.rotate_x(move1 * -1.4);
                next.control_r.orientation.rotate_z(move1 * -0.8);
                next.control.position += Vec3::new(0.0, 0.0, -6.0) * move1;

                next.control_l.orientation.rotate_z(move3 * -2.6);
                next.control_l.orientation.rotate_x(move3 * 4.0);
                next.control_r.orientation.rotate_z(move3 * 2.6);
                next.control_r.orientation.rotate_x(move3 * 4.0);
                next.control.position += Vec3::new(0.0, 0.0, 20.0) * move3;
            },
            _ => {},
        }

        next
    }
}
