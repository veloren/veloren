use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use core::f32::consts::PI;
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

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_stand")]
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
        match (hands, active_tool_kind, second_tool_kind) {
            ((Some(Hands::Two), _), tool, _) | ((None, Some(Hands::Two)), _, tool) => match tool {
                Some(ToolKind::Bow) => {
                    next.main.position = Vec3::new(0.0, -5.0, 6.0);
                    next.main.orientation =
                        Quaternion::rotation_y(2.5) * Quaternion::rotation_z(PI / 2.0);
                },
                Some(ToolKind::Staff) | Some(ToolKind::Sceptre) => {
                    next.main.position = Vec3::new(2.0, -5.0, -1.0);
                    next.main.orientation =
                        Quaternion::rotation_y(-0.5) * Quaternion::rotation_z(PI / 2.0);
                },
                _ => {
                    next.main.position = Vec3::new(-7.0, -5.0, 15.0);
                    next.main.orientation =
                        Quaternion::rotation_y(2.5) * Quaternion::rotation_z(PI / 2.0);
                },
            },
            ((_, _), _, _) => {},
        };

        match hands {
            (Some(Hands::One), _) => match active_tool_kind {
                Some(ToolKind::Dagger) => {
                    next.main.position = Vec3::new(5.0, 1.0, 2.0);
                    next.main.orientation =
                        Quaternion::rotation_x(-1.35 * PI) * Quaternion::rotation_z(2.0 * PI);
                },
                Some(ToolKind::Axe) | Some(ToolKind::Hammer) | Some(ToolKind::Sword) => {
                    next.main.position = Vec3::new(-4.0, -5.0, 10.0);
                    next.main.orientation =
                        Quaternion::rotation_y(2.5) * Quaternion::rotation_z(PI / 2.0);
                },
                Some(ToolKind::Shield) => {
                    next.main.position = Vec3::new(-0.0, -4.0, 3.0);
                    next.main.orientation =
                        Quaternion::rotation_y(0.25 * PI) * Quaternion::rotation_z(-1.5 * PI);
                },
                _ => {},
            },
            (_, _) => {},
        };
        match hands {
            (None | Some(Hands::One), Some(Hands::One)) => match second_tool_kind {
                Some(ToolKind::Dagger) => {
                    next.second.position = Vec3::new(-5.0, 1.0, 2.0);
                    next.second.orientation =
                        Quaternion::rotation_x(-1.35 * PI) * Quaternion::rotation_z(-2.0 * PI);
                },
                Some(ToolKind::Axe) | Some(ToolKind::Hammer) | Some(ToolKind::Sword) => {
                    next.second.position = Vec3::new(4.0, -6.0, 10.0);
                    next.second.orientation =
                        Quaternion::rotation_y(-2.5) * Quaternion::rotation_z(-PI / 2.0);
                },
                Some(ToolKind::Shield) => {
                    next.second.position = Vec3::new(0.0, -4.0, 3.0);
                    next.second.orientation =
                        Quaternion::rotation_y(-0.25 * PI) * Quaternion::rotation_z(1.5 * PI);
                },
                _ => {},
            },
            (_, _) => {},
        };

        next.lantern.position = Vec3::new(s_a.lantern.0, s_a.lantern.1, s_a.lantern.2);
        next.lantern.orientation = Quaternion::rotation_x(0.1) * Quaternion::rotation_y(0.1);

        if skeleton.holding_lantern {
            next.hand_r.position = Vec3::new(
                s_a.hand.0 - head_look.x * 10.0,
                s_a.hand.1 + 5.0 - head_look.y * 8.0 + slow * 0.15 - impact * 0.2,
                s_a.hand.2 + 12.0 + slow * 0.5 + impact * -0.1,
            );
            next.hand_r.orientation = Quaternion::rotation_x(2.5 + slow * -0.06 + impact * -0.1)
                * Quaternion::rotation_z(0.9)
                * Quaternion::rotation_y(head_look.x * 1.5)
                * Quaternion::rotation_x(head_look.y * 1.5);

            let fast = (anim_time * 5.0).sin();
            let fast2 = (anim_time * 4.5 + 8.0).sin();

            next.lantern.position = Vec3::new(-0.5, -0.5, -2.5);
            next.lantern.orientation = next.hand_r.orientation.inverse()
                * Quaternion::rotation_x(fast * 0.1)
                * Quaternion::rotation_y(fast2 * 0.1 + tilt * 3.0);
        }

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
