use super::{
    super::{vek::*, Animation},
    ObjectSkeleton, SkeletonAttr,
};
use common::{
    comp::{item::ToolKind, object::Body},
    states::utils::StageSection,
};
pub struct BeamAnimation;

type BeamAnimationDependency = (
    Option<ToolKind>,
    Option<ToolKind>,
    Option<StageSection>,
    Body,
);
impl Animation for BeamAnimation {
    type Dependency<'a> = BeamAnimationDependency;
    type Skeleton = ObjectSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"object_beam\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "object_beam")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, _second_tool_kind, _stage_section, _body): Self::Dependency<'_>,
        _anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;

        let mut next = (*skeleton).clone();

        next.bone0.position = Vec3::new(s_a.bone0.0, s_a.bone0.1, s_a.bone0.2);
        next.bone0.orientation = Quaternion::rotation_z(0.0);

        next.bone1.position = Vec3::new(s_a.bone1.0, s_a.bone1.1, s_a.bone1.2);
        next.bone1.orientation = Quaternion::rotation_z(0.0);

        next
    }
}
