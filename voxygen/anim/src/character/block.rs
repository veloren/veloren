use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::ToolKind;

pub struct BlockAnimation;

impl Animation for BlockAnimation {
    type Dependency = (Option<ToolKind>, Option<ToolKind>, f32);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_block\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_block")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, _second_tool_kind, _global_time): Self::Dependency,
        _anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        next.head.position = Vec3::new(0.0, -1.0 + s_a.head.0, s_a.head.1 + 19.5);
        next.head.orientation = Quaternion::rotation_x(-0.25);

        next.hand_l.position = Vec3::new(s_a.hand.0 - 6.0, s_a.hand.1 + 3.5, s_a.hand.2 + 0.0);
        next.hand_l.orientation = Quaternion::rotation_x(-0.3);
        next.hand_r.position = Vec3::new(s_a.hand.0 - 6.0, s_a.hand.1 + 3.0, s_a.hand.2 - 2.0);
        next.hand_r.orientation = Quaternion::rotation_x(-0.3);
        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_x(-0.3);

        next.torso.position = Vec3::new(0.0, -0.2, 0.1) * s_a.scaler;

        next
    }
}
