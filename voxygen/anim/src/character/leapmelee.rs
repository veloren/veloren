use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::states::utils::StageSection;

pub struct LeapAnimation;

type LeapAnimationDependency = (StageSection,);
impl Animation for LeapAnimation {
    type Dependency<'a> = LeapAnimationDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_leapmelee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_leapmelee")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        _stage_section: Self::Dependency<'_>,
        _anim_time: f32,
        rate: &mut f32,
        _s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        /* let mut next = */
        (*skeleton).clone()
        // next
    }
}
