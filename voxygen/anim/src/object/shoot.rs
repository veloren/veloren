use super::{
    super::{vek::*, Animation},
    ObjectSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};
pub struct ShootAnimation;

type ShootAnimationDependency = (
    Option<ToolKind>,
    Option<ToolKind>,
    f32,
    Vec3<f32>,
    Vec3<f32>,
    f64,
    Option<StageSection>,
);
impl Animation for ShootAnimation {
    type Dependency = ShootAnimationDependency;
    type Skeleton = ObjectSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"object_shoot\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "object_shoot")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            _active_tool_kind,
            _second_tool_kind,
            _velocity,
            orientation,
            last_ori,
            _global_time,
            _stage_section,
        ): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;

        let mut next = (*skeleton).clone();

        let ori: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let _tilt = if ::vek::Vec2::new(ori, last_ori)
            .map(|o| o.magnitude_squared())
            .map(|m| m > 0.001 && m.is_finite())
            .reduce_and()
            && ori.angle_between(last_ori).is_finite()
        {
            ori.angle_between(last_ori).min(0.2)
                * last_ori.determine_side(Vec2::zero(), ori).signum()
        } else {
            0.0
        } * 1.3;

        next.bone0.position = Vec3::new(s_a.bone0.0, s_a.bone0.1, s_a.bone0.2);
        next.bone0.orientation = Quaternion::rotation_z(anim_time as f32 * 1.0);

        next
    }
}
