use super::{
    super::{vek::*, Animation},
    ObjectSkeleton, SkeletonAttr,
};
use common::{
    comp::{item::ToolKind, object::Body},
    states::utils::StageSection,
};
pub struct ShootAnimation;

type ShootAnimationDependency = (
    Option<ToolKind>,
    Option<ToolKind>,
    Option<StageSection>,
    Body,
);
impl Animation for ShootAnimation {
    type Dependency<'a> = ShootAnimationDependency;
    type Skeleton = ObjectSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"object_shoot\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "object_shoot")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, _second_tool_kind, stage_section, body): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;

        let mut next = (*skeleton).clone();

        let (movement1, movement2, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time.powf(0.25), 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0),
        };

        next.bone0.position = Vec3::new(s_a.bone0.0, s_a.bone0.1, s_a.bone0.2);
        next.bone1.position = Vec3::new(s_a.bone1.0, s_a.bone1.1, s_a.bone1.2);

        #[allow(clippy::single_match)]
        match body {
            Body::Crossbow => {
                next.bone0.position = Vec3::new(s_a.bone0.0, s_a.bone0.1, s_a.bone0.2);
                next.bone0.orientation =
                    Quaternion::rotation_x(movement1 * 0.05 + movement2 * 0.1) * (1.0 - movement3);

                next.bone1.position = Vec3::new(s_a.bone1.0, s_a.bone1.1, s_a.bone1.2);
                next.bone1.orientation = Quaternion::rotation_z(0.0);
            },
            _ => {},
        }

        next
    }
}
