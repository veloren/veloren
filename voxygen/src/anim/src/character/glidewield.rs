use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::ToolKind;

pub struct GlideWieldAnimation;

type GlideWieldAnimationDependency = (
    Option<ToolKind>,
    Option<ToolKind>,
    Vec3<f32>,
    Vec3<f32>,
    Vec3<f32>,
    f64,
);

impl Animation for GlideWieldAnimation {
    type Dependency = GlideWieldAnimationDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_glidewield\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_glidewield")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, _second_tool_kind, velocity, _orientation, _last_ori, _global_time): Self::Dependency,
        _anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();
        *rate = 1.0;

        next.hand_l.position = Vec3::new(-2.0 - s_a.hand.0, s_a.hand.1, s_a.hand.2 + 15.0);
        next.hand_l.orientation = Quaternion::rotation_x(3.35);

        next.hand_r.position = Vec3::new(2.0 + s_a.hand.0, s_a.hand.1, s_a.hand.2 + 15.0);
        next.hand_r.orientation = Quaternion::rotation_x(3.35);
        next.glider.scale = Vec3::one() * 1.0;

        if speed > 0.5 {
            next.glider.orientation = Quaternion::rotation_x(0.8);
            next.glider.position = Vec3::new(0.0, -10.0, 15.0);
        } else {
            next.glider.orientation = Quaternion::rotation_x(0.35);
            next.glider.position = Vec3::new(0.0, -9.0, 17.0);
        }

        next
    }
}
