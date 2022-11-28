use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use core::f32::consts::PI;
use std::ops::Mul;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency<'a> = (
        Option<ToolKind>,
        Option<ToolKind>,
        (Option<Hands>, Option<Hands>),
        f32,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_idle")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, hands, global_time): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let slow = (anim_time * 1.0).sin();
        let head_look = Vec2::new(
            (global_time + anim_time / 12.0).floor().mul(7331.0).sin() * 0.1,
            (global_time + anim_time / 12.0).floor().mul(1337.0).sin() * 0.05,
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
        next.head.orientation =
            Quaternion::rotation_z(head_look.x) * Quaternion::rotation_x(head_look.y.abs());

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + slow * 0.3);
        next.chest.orientation = Quaternion::rotation_z(head_look.x * 0.6);

        next.belt.position = Vec3::new(0.0, s_a.belt.0, s_a.belt.1);
        next.belt.orientation = Quaternion::rotation_z(head_look.x * -0.1);

        next.back.position = Vec3::new(0.0, s_a.back.0, s_a.back.1);

        next.shorts.position = Vec3::new(0.0, s_a.shorts.0, s_a.shorts.1);
        next.shorts.orientation = Quaternion::rotation_z(head_look.x * -0.2);

        next.hand_l.position = Vec3::new(
            -s_a.hand.0,
            s_a.hand.1 + slow * 0.15,
            s_a.hand.2 + slow * 0.5,
        );

        next.hand_l.orientation = Quaternion::rotation_x(slow * -0.06);

        next.hand_r.position = Vec3::new(
            s_a.hand.0,
            s_a.hand.1 + slow * 0.15,
            s_a.hand.2 + slow * 0.5,
        );
        next.hand_r.orientation = Quaternion::rotation_x(slow * -0.06);

        next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2);

        next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2);

        next.shoulder_l.position = Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);

        next.shoulder_r.position = Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);

        next.glider.position = Vec3::new(0.0, 0.0, 10.0);
        next.glider.scale = Vec3::one() * 0.0;
        next.hold.position = Vec3::new(0.4, -0.3, -5.8);
        match hands {
            (Some(Hands::Two), _) | (None, Some(Hands::Two)) => match active_tool_kind {
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
            (_, _) => {},
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
                        Quaternion::rotation_y(2.35) * Quaternion::rotation_z(PI / 2.0);
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
                _ => {},
            },
            (_, _) => {},
        };

        next.lantern.position = Vec3::new(s_a.lantern.0, s_a.lantern.1, s_a.lantern.2);
        next.lantern.orientation = Quaternion::rotation_x(0.1) * Quaternion::rotation_y(0.1);

        next.torso.position = Vec3::new(0.0, 0.0, 0.0);

        next.second.scale = match hands {
            (Some(Hands::One), Some(Hands::One)) => Vec3::one(),
            (_, _) => Vec3::zero(),
        };

        if let (None, Some(Hands::Two)) = hands {
            next.second = next.main;
        }

        next
    }
}
