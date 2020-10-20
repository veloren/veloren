use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use std::ops::Mul;

pub struct BlockAnimation;

impl Animation for BlockAnimation {
    type Dependency = (Option<ToolKind>, Option<ToolKind>, f64);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_block\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_block")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let _head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 1.5)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.3,
            ((global_time + anim_time) as f32 / 1.5)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.15,
        );
        next.head.position = Vec3::new(
            0.0,
            -1.0 + skeleton_attr.head.0,
            skeleton_attr.head.1 + 19.5,
        );
        next.head.orientation = Quaternion::rotation_x(-0.25);
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.position = Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1);

        next.belt.position = Vec3::new(0.0, skeleton_attr.belt.0, skeleton_attr.belt.1);

        next.shorts.position = Vec3::new(0.0, skeleton_attr.shorts.0, skeleton_attr.shorts.1);

        match active_tool_kind {
            Some(ToolKind::Shield(_)) => {
                next.hand_l.position = Vec3::new(
                    skeleton_attr.hand.0 - 6.0,
                    skeleton_attr.hand.1 + 3.5,
                    skeleton_attr.hand.2 + 0.0,
                );
                next.hand_l.orientation = Quaternion::rotation_x(-0.3);
                next.hand_r.position = Vec3::new(
                    skeleton_attr.hand.0 - 6.0,
                    skeleton_attr.hand.1 + 3.0,
                    skeleton_attr.hand.2 - 2.0,
                );
                next.hand_r.orientation = Quaternion::rotation_x(-0.3);
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(-0.3);
            },
            _ => {},
        }

        next.glider.position = Vec3::new(0.0, 0.0, 10.0);
        next.glider.scale = Vec3::one() * 0.0;

        next.lantern.position = Vec3::new(
            skeleton_attr.lantern.0,
            skeleton_attr.lantern.1,
            skeleton_attr.lantern.2,
        );
        next.lantern.scale = Vec3::one() * 0.65;
        next.hold.scale = Vec3::one() * 0.0;

        next.torso.position = Vec3::new(0.0, -0.2, 0.1) * skeleton_attr.scaler;

        next.second.scale = match (
            active_tool_kind.map(|tk| tk.hands()),
            second_tool_kind.map(|tk| tk.hands()),
        ) {
            (Some(Hands::OneHand), Some(Hands::OneHand)) => Vec3::one(),
            (_, _) => Vec3::zero(),
        };

        next
    }
}
