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
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_hands, _ability_id, _global_time, _stage_section, _ability_info): Self::Dependency<'_>,
        _anim_time: f32,
        rate: &mut f32,
        _s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_z(0.0);
        next.main_weapon_trail = true;
        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation = Quaternion::rotation_z(0.0);
        next.off_weapon_trail = true;

        next
    }
}
