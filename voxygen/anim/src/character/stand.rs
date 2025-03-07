use super::{
    super::{Animation, vek::*},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use std::ops::Mul;

pub struct StandAnimation;

impl Animation for StandAnimation {
    type Dependency<'a> = (
        Option<ToolKind>,
        Option<ToolKind>,
        (Option<Hands>, Option<Hands>),
        Vec3<f32>,
        Vec3<f32>,
        f32,
        Vec3<f32>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_stand\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "character_stand"))]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, hands, orientation, last_ori, global_time, avg_vel): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let slow = (anim_time * 1.0).sin();
        let impact = (avg_vel.z).max(-15.0);
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
        let head_look = Vec2::new(
            ((global_time + anim_time) / 10.0).floor().mul(7331.0).sin() * 0.15,
            ((global_time + anim_time) / 10.0).floor().mul(1337.0).sin() * 0.07,
        );
        next.head.scale = Vec3::one() * s_a.head_scale;
        next.chest.scale = Vec3::one() * 1.01;
        next.hand_l.scale = Vec3::one() * 1.04;
        next.hand_r.scale = Vec3::one() * 1.04;
        next.back.scale = Vec3::one() * 1.02;
        next.hold.scale = Vec3::one() * 0.0;
        next.lantern.scale = Vec3::one() * 0.65;
        next.shoulder_l.scale = Vec3::one() * 1.1;
        next.shoulder_r.scale = Vec3::one() * 1.1;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1 + slow * 0.3);
        next.head.orientation = Quaternion::rotation_z(head_look.x)
            * Quaternion::rotation_x(impact * -0.02 + head_look.y.abs());

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + slow * 0.3 + impact * 0.2);
        next.chest.orientation =
            Quaternion::rotation_z(head_look.x * 0.6) * Quaternion::rotation_x(impact * 0.04);

        next.belt.position = Vec3::new(0.0, s_a.belt.0 + impact * 0.005, s_a.belt.1);
        next.belt.orientation =
            Quaternion::rotation_z(head_look.x * -0.1) * Quaternion::rotation_x(impact * -0.03);

        next.back.position = Vec3::new(0.0, s_a.back.0, s_a.back.1);

        next.shorts.position = Vec3::new(0.0, s_a.shorts.0 + impact * -0.2, s_a.shorts.1);
        next.shorts.orientation =
            Quaternion::rotation_z(head_look.x * -0.2) * Quaternion::rotation_x(impact * -0.04);

        next.hand_l.position = Vec3::new(
            -s_a.hand.0,
            s_a.hand.1 + slow * 0.15 - impact * 0.2,
            s_a.hand.2 + slow * 0.5 + impact * -0.1,
        );

        next.hand_l.orientation = Quaternion::rotation_x(slow * -0.06 + impact * -0.1);

        next.hand_r.position = Vec3::new(
            s_a.hand.0,
            s_a.hand.1 + slow * 0.15 - impact * 0.2,
            s_a.hand.2 + slow * 0.5 + impact * -0.1,
        );
        next.hand_r.orientation = Quaternion::rotation_x(slow * -0.06 + impact * -0.1);

        next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1 - impact * 0.15, s_a.foot.2);
        next.foot_l.orientation = Quaternion::rotation_x(impact * 0.02);

        next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1 + impact * 0.15, s_a.foot.2);
        next.foot_r.orientation = Quaternion::rotation_x(impact * -0.02);

        next.shoulder_l.position = Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);

        next.shoulder_r.position = Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);

        next.glider.position = Vec3::new(0.0, 0.0, 10.0);
        next.glider.scale = Vec3::one() * 0.0;
        next.hold.position = Vec3::new(0.4, -0.3, -5.8);

        next.do_tools_on_back(hands, active_tool_kind, second_tool_kind);

        next.do_hold_lantern(s_a, anim_time, 0.0, 0.0, impact, tilt);

        next.torso.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.scale = Vec3::one();
        next.second.scale = match hands {
            (Some(Hands::One) | None, Some(Hands::One)) => Vec3::one(),
            (_, _) => Vec3::zero(),
        };

        if let (None, Some(Hands::Two)) = hands {
            next.second = next.main;
        }

        next
    }
}
