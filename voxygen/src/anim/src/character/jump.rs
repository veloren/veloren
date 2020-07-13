use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::{Hands, ToolKind};
use std::f32::consts::PI;
use vek::*;

pub struct JumpAnimation;
impl Animation for JumpAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
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
        (active_tool_kind, second_tool_kind, orientation, last_ori, global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
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
        let tilt = if Vec2::new(ori, last_ori)
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

        next.head.offset = Vec3::new(
            0.0,
            -3.0 + skeleton_attr.head.0,
            -1.0 + skeleton_attr.head.1,
        );
        next.head.ori =
            Quaternion::rotation_x(0.25 + slow * 0.04) * Quaternion::rotation_z(tilt * -2.5);
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1 + 1.0);
        next.chest.ori = Quaternion::rotation_z(tilt * -2.0);
        next.chest.scale = Vec3::one() * 1.01;

        next.belt.offset = Vec3::new(0.0, skeleton_attr.belt.0, skeleton_attr.belt.1);
        next.belt.ori = Quaternion::rotation_z(tilt * 2.0);
        next.belt.scale = Vec3::one();

        next.back.offset = Vec3::new(0.0, skeleton_attr.back.0, skeleton_attr.back.1);
        next.back.ori = Quaternion::rotation_z(0.0);
        next.back.scale = Vec3::one() * 1.02;

        next.shorts.offset = Vec3::new(0.0, skeleton_attr.shorts.0, skeleton_attr.shorts.1);
        next.shorts.ori = Quaternion::rotation_z(tilt * 3.0);
        next.shorts.scale = Vec3::one();

        if random > 0.5 {
            next.l_hand.offset = Vec3::new(
                -skeleton_attr.hand.0,
                1.0 + skeleton_attr.hand.1 + 4.0,
                2.0 + skeleton_attr.hand.2 + slow * 1.5,
            );
            next.l_hand.ori =
                Quaternion::rotation_x(1.9 + slow * 0.4) * Quaternion::rotation_y(0.2);
            next.l_hand.scale = Vec3::one();

            next.r_hand.offset = Vec3::new(
                skeleton_attr.hand.0,
                skeleton_attr.hand.1 - 3.0,
                skeleton_attr.hand.2 + slow * 1.5,
            );
            next.r_hand.ori =
                Quaternion::rotation_x(-0.5 + slow * -0.4) * Quaternion::rotation_y(-0.2);
            next.r_hand.scale = Vec3::one();
        } else {
            next.l_hand.offset = Vec3::new(
                -skeleton_attr.hand.0,
                skeleton_attr.hand.1 - 3.0,
                skeleton_attr.hand.2 + slow * 1.5,
            );
            next.l_hand.ori =
                Quaternion::rotation_x(-0.5 + slow * -0.4) * Quaternion::rotation_y(0.2);
            next.l_hand.scale = Vec3::one();

            next.r_hand.offset = Vec3::new(
                skeleton_attr.hand.0,
                1.0 + skeleton_attr.hand.1 + 4.0,
                2.0 + skeleton_attr.hand.2 + slow * 1.5,
            );
            next.r_hand.ori =
                Quaternion::rotation_x(1.9 + slow * 0.4) * Quaternion::rotation_y(-0.2);
            next.r_hand.scale = Vec3::one();
        };

        next.l_foot.offset = Vec3::new(
            -skeleton_attr.foot.0,
            skeleton_attr.foot.1 - 6.0 * switch,
            1.0 + skeleton_attr.foot.2 + slow * 1.5,
        );
        next.l_foot.ori = Quaternion::rotation_x(-1.2 * switch + slow * -0.2 * switch);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(
            skeleton_attr.foot.0,
            skeleton_attr.foot.1 + 6.0 * switch,
            1.0 + skeleton_attr.foot.2 + slow * 1.5,
        );
        next.r_foot.ori = Quaternion::rotation_x(1.2 * switch + slow * 0.2 * switch);
        next.r_foot.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(
            -skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.l_shoulder.ori = Quaternion::rotation_x(0.4 * switch);
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.r_shoulder.ori = Quaternion::rotation_x(-0.4 * switch);
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.glider.offset = Vec3::new(0.0, 0.0, 10.0);
        next.glider.scale = Vec3::one() * 0.0;

        match active_tool_kind {
            Some(ToolKind::Dagger(_)) => {
                next.main.offset = Vec3::new(-4.0, -5.0, 7.0);
                next.main.ori =
                    Quaternion::rotation_y(0.25 * PI) * Quaternion::rotation_z(1.5 * PI);
            },
            Some(ToolKind::Shield(_)) => {
                next.main.offset = Vec3::new(-0.0, -5.0, 3.0);
                next.main.ori =
                    Quaternion::rotation_y(0.25 * PI) * Quaternion::rotation_z(-1.5 * PI);
            },
            _ => {
                next.main.offset = Vec3::new(-7.0, -5.0, 15.0);
                next.main.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
            },
        }
        next.main.scale = Vec3::one();

        match second_tool_kind {
            Some(ToolKind::Dagger(_)) => {
                next.second.offset = Vec3::new(4.0, -6.0, 7.0);
                next.second.ori =
                    Quaternion::rotation_y(-0.25 * PI) * Quaternion::rotation_z(-1.5 * PI);
            },
            Some(ToolKind::Shield(_)) => {
                next.second.offset = Vec3::new(0.0, -4.0, 3.0);
                next.second.ori =
                    Quaternion::rotation_y(-0.25 * PI) * Quaternion::rotation_z(1.5 * PI);
            },
            _ => {
                next.second.offset = Vec3::new(-7.0, -5.0, 15.0);
                next.second.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
            },
        }
        next.second.scale = Vec3::one();

        next.lantern.offset = Vec3::new(
            skeleton_attr.lantern.0,
            skeleton_attr.lantern.1,
            skeleton_attr.lantern.2,
        );
        next.lantern.ori = Quaternion::rotation_x(1.0 * switch + slow * 0.3 * switch)
            * Quaternion::rotation_y(0.6 * switch + slow * 0.3 * switch);
        next.lantern.scale = Vec3::one() * 0.65;

        next.torso.offset = Vec3::new(0.0, 0.0, 0.0) * skeleton_attr.scaler;
        next.torso.ori = Quaternion::rotation_x(-0.2);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next.control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.control.ori = Quaternion::rotation_x(0.0);
        next.control.scale = Vec3::one();

        next.l_control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.l_control.ori = Quaternion::rotation_x(0.0);
        next.l_control.scale = Vec3::one();

        next.r_control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.r_control.ori = Quaternion::rotation_x(0.0);
        next.r_control.scale = Vec3::one();

        next.second.scale = match (
            active_tool_kind.map(|tk| tk.into_hands()),
            second_tool_kind.map(|tk| tk.into_hands()),
        ) {
            (Some(Hands::OneHand), Some(Hands::OneHand)) => Vec3::one(),
            (_, _) => Vec3::zero(),
        };

        next
    }
}
