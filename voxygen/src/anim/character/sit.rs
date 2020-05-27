use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::ToolKind;
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct SitAnimation;

impl Animation for SitAnimation {
    type Dependency = (Option<ToolKind>, f64);
    type Skeleton = CharacterSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let slow = (anim_time as f32 * 1.0).sin();
        let slowa = (anim_time as f32 * 1.0 + PI / 2.0).sin();
        let stop = (anim_time as f32 * 3.0).min(PI / 2.0).sin();
        let slow_abs = ((anim_time as f32 * 0.3).sin()) + 1.0;

        let head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 18.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.25,
            ((global_time + anim_time) as f32 / 18.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.125,
        );
        next.head.offset = Vec3::new(
            0.0,
            -3.0 + skeleton_attr.head.0,
            skeleton_attr.head.1 + slow * 0.1 + stop * -0.8,
        );
        next.head.ori = Quaternion::rotation_z(head_look.x + slow * 0.2 - slow * 0.1)
            * Quaternion::rotation_x((slowa * -0.1 + slow * 0.1 + head_look.y).abs());
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(
            0.0,
            skeleton_attr.chest.0 + stop * -0.4,
            skeleton_attr.chest.1 + slow * 0.1 + stop * -0.8,
        );
        next.chest.ori = Quaternion::rotation_x(stop * 0.15);
        next.chest.scale = Vec3::one() + slow_abs * 0.05;

        next.belt.offset = Vec3::new(0.0, skeleton_attr.belt.0 + stop * 1.2, skeleton_attr.belt.1);
        next.belt.ori = Quaternion::rotation_x(stop * 0.3);
        next.belt.scale = (Vec3::one() + slow_abs * 0.05) * 1.02;

        next.back.offset = Vec3::new(0.0, skeleton_attr.back.0, skeleton_attr.back.1);
        next.back.scale = Vec3::one() * 1.02;

        next.shorts.offset = Vec3::new(
            0.0,
            skeleton_attr.shorts.0 + stop * 2.5,
            skeleton_attr.shorts.1 + stop * 0.6,
        );
        next.shorts.ori = Quaternion::rotation_x(stop * 0.6);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(
            -skeleton_attr.hand.0,
            skeleton_attr.hand.1 + slowa * 0.15,
            skeleton_attr.hand.2 + slow * 0.7 + stop * -2.0,
        );
        next.l_hand.ori = Quaternion::rotation_x(slowa * -0.1 + slow * 0.1);
        next.l_hand.scale = Vec3::one() + slow_abs * -0.05;

        next.r_hand.offset = Vec3::new(
            skeleton_attr.hand.0,
            skeleton_attr.hand.1 + slowa * 0.15,
            skeleton_attr.hand.2 + slow * 0.7 + stop * -2.0,
        );
        next.r_hand.ori = Quaternion::rotation_x(slow * -0.1 + slowa * 0.1);
        next.r_hand.scale = Vec3::one() + slow_abs * -0.05;

        next.l_foot.offset = Vec3::new(
            -skeleton_attr.foot.0,
            4.0 + skeleton_attr.foot.1,
            3.0 + skeleton_attr.foot.2,
        );
        next.l_foot.ori = Quaternion::rotation_x(slow * 0.1 + stop * 1.2 + slow * 0.1);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(
            skeleton_attr.foot.0,
            4.0 + skeleton_attr.foot.1,
            3.0 + skeleton_attr.foot.2,
        );
        next.r_foot.ori = Quaternion::rotation_x(slowa * 0.1 + stop * 1.2 + slowa * 0.1);
        next.r_foot.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(
            -skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = (Vec3::one() + slow_abs * -0.05) * 1.15;

        next.r_shoulder.offset = Vec3::new(
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = (Vec3::one() + slow_abs * -0.05) * 1.15;

        next.glider.offset = Vec3::new(0.0, 0.0, 10.0);
        next.glider.scale = Vec3::one() * 0.0;

        next.main.offset = Vec3::new(-7.0, -5.0, 15.0);
        next.main.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
        next.main.scale = Vec3::one() + slow_abs * -0.05;

        next.second.offset = Vec3::new(0.0, 0.0, 0.0);
        next.second.ori = Quaternion::rotation_y(0.0);
        next.second.scale = Vec3::one() * 0.0;

        next.lantern.offset = Vec3::new(
            skeleton_attr.lantern.0,
            skeleton_attr.lantern.1,
            skeleton_attr.lantern.2,
        );
        next.lantern.scale = Vec3::one() * 0.65;

        next.torso.offset = Vec3::new(0.0, -0.2, stop * -0.16) * skeleton_attr.scaler;
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next.control.scale = Vec3::one();

        next.l_control.scale = Vec3::one();

        next.r_control.scale = Vec3::one();
        next
    }
}
