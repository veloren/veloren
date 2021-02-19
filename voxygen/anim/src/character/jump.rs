use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use std::f32::consts::PI;

pub struct JumpAnimation;
impl Animation for JumpAnimation {
    #[allow(clippy::type_complexity)]
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        (Option<Hands>, Option<Hands>),
        Vec3<f32>,
        Vec3<f32>,
        f64,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_jump\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_jump")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, hands, orientation, last_ori, global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let slow = (anim_time as f32 * 7.0).sin();

        let random = ((((2.0
            * ((global_time as f32 - anim_time as f32)
                - ((global_time as f32 - anim_time as f32).round())))
        .abs())
            * 10.0)
            .round())
            / 10.0;

        let switch = if random > 0.5 { 1.0 } else { -1.0 };

        let ori: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if ::vek::Vec2::new(ori, last_ori)
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
        next.head.scale = Vec3::one() * s_a.head_scale;
        next.shoulder_l.scale = Vec3::one() * 1.1;
        next.shoulder_r.scale = Vec3::one() * 1.1;
        next.back.scale = Vec3::one() * 1.02;

        next.head.position = Vec3::new(0.0, -1.0 + s_a.head.0, -1.0 + s_a.head.1);
        next.head.orientation =
            Quaternion::rotation_x(0.25 + slow * 0.04) * Quaternion::rotation_z(tilt * -2.5);

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + 1.0);
        next.chest.orientation = Quaternion::rotation_z(tilt * -2.0);

        next.belt.position = Vec3::new(0.0, s_a.belt.0, s_a.belt.1);
        next.belt.orientation = Quaternion::rotation_z(tilt * 2.0);

        next.back.position = Vec3::new(0.0, s_a.back.0, s_a.back.1);
        next.back.orientation = Quaternion::rotation_z(0.0);

        next.shorts.position = Vec3::new(0.0, s_a.shorts.0, s_a.shorts.1);
        next.shorts.orientation = Quaternion::rotation_z(tilt * 3.0);

        if random > 0.5 {
            next.hand_l.position = Vec3::new(
                -s_a.hand.0,
                1.0 + s_a.hand.1 + 4.0,
                2.0 + s_a.hand.2 + slow * 1.5,
            );
            next.hand_l.orientation =
                Quaternion::rotation_x(1.9 + slow * 0.4) * Quaternion::rotation_y(0.2);

            next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1 - 3.0, s_a.hand.2 + slow * 1.5);
            next.hand_r.orientation =
                Quaternion::rotation_x(-0.5 + slow * -0.4) * Quaternion::rotation_y(-0.2);
        } else {
            next.hand_l.position =
                Vec3::new(-s_a.hand.0, s_a.hand.1 - 3.0, s_a.hand.2 + slow * 1.5);
            next.hand_l.orientation =
                Quaternion::rotation_x(-0.5 + slow * -0.4) * Quaternion::rotation_y(0.2);

            next.hand_r.position = Vec3::new(
                s_a.hand.0,
                1.0 + s_a.hand.1 + 4.0,
                2.0 + s_a.hand.2 + slow * 1.5,
            );
            next.hand_r.orientation =
                Quaternion::rotation_x(1.9 + slow * 0.4) * Quaternion::rotation_y(-0.2);
        };

        next.foot_l.position = Vec3::new(
            -s_a.foot.0,
            s_a.foot.1 - 6.0 * switch,
            1.0 + s_a.foot.2 + slow * 1.5,
        );
        next.foot_l.orientation = Quaternion::rotation_x(-1.2 * switch + slow * -0.2 * switch);

        next.foot_r.position = Vec3::new(
            s_a.foot.0,
            s_a.foot.1 + 6.0 * switch,
            1.0 + s_a.foot.2 + slow * 1.5,
        );
        next.foot_r.orientation = Quaternion::rotation_x(1.2 * switch + slow * 0.2 * switch);

        next.shoulder_l.position = Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_l.orientation = Quaternion::rotation_x(0.4 * switch);

        next.shoulder_r.position = Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_r.orientation = Quaternion::rotation_x(-0.4 * switch);

        next.glider.position = Vec3::new(0.0, 0.0, 10.0);
        next.glider.scale = Vec3::one() * 0.0;

        match active_tool_kind {
            Some(ToolKind::Dagger) => {
                next.main.position = Vec3::new(-4.0, -5.0, 7.0);
                next.main.orientation =
                    Quaternion::rotation_y(0.25 * PI) * Quaternion::rotation_z(1.5 * PI);
            },
            Some(ToolKind::Shield) => {
                next.main.position = Vec3::new(-0.0, -5.0, 3.0);
                next.main.orientation =
                    Quaternion::rotation_y(0.25 * PI) * Quaternion::rotation_z(-1.5 * PI);
            },
            Some(ToolKind::Staff) | Some(ToolKind::Sceptre) => {
                next.main.position = Vec3::new(2.0, -5.0, -1.0);
                next.main.orientation = Quaternion::rotation_y(-0.5) * Quaternion::rotation_z(1.57);
            },
            Some(ToolKind::Bow) => {
                next.main.position = Vec3::new(0.0, -5.0, 6.0);
                next.main.orientation = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
            },
            _ => {
                next.main.position = Vec3::new(-7.0, -5.0, 15.0);
                next.main.orientation = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
            },
        }

        match second_tool_kind {
            Some(ToolKind::Dagger) => {
                next.second.position = Vec3::new(4.0, -6.0, 7.0);
                next.second.orientation =
                    Quaternion::rotation_y(-0.25 * PI) * Quaternion::rotation_z(-1.5 * PI);
            },
            Some(ToolKind::Shield) => {
                next.second.position = Vec3::new(0.0, -4.0, 3.0);
                next.second.orientation =
                    Quaternion::rotation_y(-0.25 * PI) * Quaternion::rotation_z(1.5 * PI);
            },

            _ => {
                next.second.position = Vec3::new(-7.0, -5.0, 15.0);
                next.second.orientation =
                    Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
            },
        }

        next.lantern.position = Vec3::new(s_a.lantern.0, s_a.lantern.1, s_a.lantern.2);
        next.lantern.orientation = Quaternion::rotation_x(1.0 * switch + slow * 0.3 * switch)
            * Quaternion::rotation_y(0.6 * switch + slow * 0.3 * switch);
        next.lantern.scale = Vec3::one() * 0.65;
        next.hold.scale = Vec3::one() * 0.0;

        next.torso.position = Vec3::new(0.0, 0.0, 0.0) * s_a.scaler;
        next.torso.orientation = Quaternion::rotation_x(-0.2);
        next.torso.scale = Vec3::one() / 11.0 * s_a.scaler;

        next.second.scale = match hands {
            (Some(Hands::One), Some(Hands::One)) => Vec3::one(),
            (_, _) => Vec3::zero(),
        };

        next
    }
}
