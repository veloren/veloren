use super::{
    super::{vek::*, Animation},
    ShipSkeleton, SkeletonAttr,
};
use common::comp::item::ToolKind;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency<'a> = (
        Option<ToolKind>,
        Option<ToolKind>,
        f32,
        f32,
        Vec3<f32>,
        Vec3<f32>,
    );
    type Skeleton = ShipSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"ship_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "ship_idle")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, _second_tool_kind, _global_time, acc_vel, orientation, last_ori): Self::Dependency<'_>,
        _anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let ori: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if vek::Vec2::new(ori, last_ori)
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

        next.bone1.position = Vec3::new(s_a.bone1.0, s_a.bone1.1, s_a.bone1.2);
        next.bone1.orientation = Quaternion::rotation_y(acc_vel * 0.8);

        next.bone2.position = Vec3::new(s_a.bone2.0, s_a.bone2.1, s_a.bone2.2);
        next.bone2.orientation = Quaternion::rotation_y(-acc_vel * 0.8);

        next.bone3.position = Vec3::new(s_a.bone3.0, s_a.bone3.1, s_a.bone3.2);
        next.bone3.orientation = Quaternion::rotation_z(tilt * 25.0);
        next
    }
}
