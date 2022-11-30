use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, util::Dir};
use core::f32::consts::PI;

pub struct TalkAnimation;

impl Animation for TalkAnimation {
    type Dependency<'a> = (Option<ToolKind>, Option<ToolKind>, f32, f32, Dir);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_talk\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_talk")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, _second_tool_kind, _velocity, _global_time, look_dir): Self::Dependency<
            '_,
        >,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let slowa = (anim_time * 6.0).sin();
        let slowb = (anim_time * 4.0 + PI / 2.0).sin();
        let slowc = (anim_time * 12.0 + PI / 2.0).sin();

        next.head.orientation = Quaternion::rotation_x(slowc * 0.035 + look_dir.z * 0.7);
        next.hand_l.position = Vec3::new(
            -s_a.hand.0 + 0.5 + slowb * 0.5,
            s_a.hand.1 + 5.0 + slowc * 1.0,
            s_a.hand.2 + 2.0 + slowa * 1.0,
        );

        next.hand_l.orientation = Quaternion::rotation_x(0.0);

        next.hand_r.position = Vec3::new(
            s_a.hand.0 - 0.5 + slowb * 0.5,
            s_a.hand.1 + 4.0 + slowc * -1.0,
            s_a.hand.2 + 2.0 + slowa * 1.0,
        );
        next.hand_l.orientation = Quaternion::rotation_y(-0.2 + slowb * 0.2 + slowa * 0.07)
            * Quaternion::rotation_x(1.3 + slowa * 0.15);
        next.hand_r.orientation = Quaternion::rotation_y(0.2 + slowa * -0.1 + slowb * 0.07)
            * Quaternion::rotation_x(1.3 + slowb * -0.15 + slowc * 0.05);

        next
    }
}
