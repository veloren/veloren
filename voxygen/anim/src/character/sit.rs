use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::ToolKind;
use std::{f32::consts::PI, ops::Mul};

pub struct SitAnimation;

impl Animation for SitAnimation {
    type Dependency<'a> = (Option<ToolKind>, Option<ToolKind>, f32);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_sit\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_sit")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, _second_tool_kind, global_time): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let slow = (anim_time * 1.0).sin();
        let slowa = (anim_time * 1.0 + PI / 2.0).sin();
        let stop = (anim_time * 3.0).min(PI / 2.0).sin();

        let head_look = Vec2::new(
            (global_time * 0.05 + anim_time / 15.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.25,
            (global_time * 0.05 + anim_time / 15.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.125,
        );
        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1 + slow * 0.1 + stop * -0.8);
        next.head.orientation = Quaternion::rotation_z(head_look.x + slow * 0.2 - slow * 0.1)
            * Quaternion::rotation_x((slowa * -0.1 + slow * 0.1 + head_look.y).abs());

        next.chest.position = Vec3::new(
            0.0,
            s_a.chest.0 + stop * -0.4,
            s_a.chest.1 + slow * 0.1 + stop * -0.8,
        );
        next.chest.orientation = Quaternion::rotation_x(stop * 0.15);

        next.belt.position = Vec3::new(0.0, s_a.belt.0 + stop * 1.2, s_a.belt.1);
        next.belt.orientation = Quaternion::rotation_x(stop * 0.3);

        next.back.position = Vec3::new(0.0, s_a.back.0, s_a.back.1);

        next.shorts.position = Vec3::new(0.0, s_a.shorts.0 + stop * 2.5, s_a.shorts.1 + stop * 0.6);
        next.shorts.orientation = Quaternion::rotation_x(stop * 0.6);

        next.hand_l.position = Vec3::new(
            -s_a.hand.0,
            s_a.hand.1 + slowa * 0.15,
            s_a.hand.2 + slow * 0.7 + stop * -2.0,
        );
        next.hand_l.orientation = Quaternion::rotation_x(slowa * -0.1 + slow * 0.1);

        next.hand_r.position = Vec3::new(
            s_a.hand.0,
            s_a.hand.1 + slowa * 0.15,
            s_a.hand.2 + slow * 0.7 + stop * -2.0,
        );
        next.hand_r.orientation = Quaternion::rotation_x(slow * -0.1 + slowa * 0.1);

        next.foot_l.position = Vec3::new(-s_a.foot.0, 4.0 + s_a.foot.1, 3.0 + s_a.foot.2);
        next.foot_l.orientation = Quaternion::rotation_x(slow * 0.1 + stop * 1.2 + slow * 0.1);

        next.foot_r.position = Vec3::new(s_a.foot.0, 4.0 + s_a.foot.1, 3.0 + s_a.foot.2);
        next.foot_r.orientation = Quaternion::rotation_x(slowa * 0.1 + stop * 1.2 + slowa * 0.1);

        next.shoulder_l.position = Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_l.orientation = Quaternion::rotation_x(0.0);

        next.shoulder_r.position = Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_r.orientation = Quaternion::rotation_x(0.0);

        next.torso.position = Vec3::new(0.0, -2.2, stop * -1.76);

        if skeleton.holding_lantern {
            next.hand_r.position = Vec3::new(
                s_a.hand.0 + 1.0 - head_look.x * 8.0,
                s_a.hand.1 + 5.0 + head_look.x * 6.0,
                s_a.hand.2 + 9.0 + head_look.y * 6.0,
            );
            next.hand_r.orientation = Quaternion::rotation_x(2.25)
                * Quaternion::rotation_z(0.9)
                * Quaternion::rotation_y(head_look.x * 3.0)
                * Quaternion::rotation_x(head_look.y * 3.0);

            let fast = (anim_time * 5.0).sin();
            let fast2 = (anim_time * 4.5 + 8.0).sin();

            next.lantern.position = Vec3::new(-0.5, -0.5, -2.5);
            next.lantern.orientation = next.hand_r.orientation.inverse()
                * Quaternion::rotation_x(fast * 0.1)
                * Quaternion::rotation_y(fast2 * 0.1);
        }

        next
    }
}
