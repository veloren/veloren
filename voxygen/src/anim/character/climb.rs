use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::ToolKind;
use std::f32::consts::PI;
use vek::*;

pub struct ClimbAnimation;

impl Animation for ClimbAnimation {
    type Dependency = (Option<ToolKind>, Vec3<f32>, Vec3<f32>, f64);
    type Skeleton = CharacterSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, velocity, _orientation, _global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let speed = velocity.magnitude();
        *rate = speed;

        let constant = 1.0;
        let smooth = (anim_time as f32 * constant as f32 * 1.5).sin();
        let smootha = (anim_time as f32 * constant as f32 * 1.5 + PI / 2.0).sin();

        let quick = (((5.0)
            / (0.6 + 4.0 * ((anim_time as f32 * constant as f32 * 1.5).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * constant as f32 * 1.5).sin());
        let quicka = (((5.0)
            / (0.6
                + 4.0
                    * ((anim_time as f32 * constant as f32 * 1.5 + PI / 2.0).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * constant as f32 * 1.5 + PI / 2.0).sin());

        next.head.offset = Vec3::new(
            0.0,
            -4.0 + skeleton_attr.head.0,
            skeleton_attr.head.1 + smootha * 0.2,
        );
        next.head.ori = Quaternion::rotation_z(smooth * 0.1)
            * Quaternion::rotation_x(0.6)
            * Quaternion::rotation_y(quick * 0.1);
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(
            0.0,
            skeleton_attr.chest.0 + 1.0,
            skeleton_attr.chest.1 + smootha * 1.1,
        );
        next.chest.ori = Quaternion::rotation_z(quick * 0.25)
            * Quaternion::rotation_x(-0.15)
            * Quaternion::rotation_y(quick * -0.12);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0, skeleton_attr.belt.0 + 1.0, skeleton_attr.belt.1);
        next.belt.ori = Quaternion::rotation_z(quick * 0.0) * Quaternion::rotation_x(0.0);
        next.belt.scale = Vec3::one();

        next.back.offset = Vec3::new(0.0, skeleton_attr.back.0, skeleton_attr.back.1);
        next.back.ori = Quaternion::rotation_x(-0.2);
        next.back.scale = Vec3::one() * 1.02;

        next.shorts.offset = Vec3::new(0.0, skeleton_attr.shorts.0 + 1.0, skeleton_attr.shorts.1);
        next.shorts.ori = Quaternion::rotation_z(quick * 0.0)
            * Quaternion::rotation_x(0.1)
            * Quaternion::rotation_y(quick * 0.10);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(
            -skeleton_attr.hand.0,
            skeleton_attr.hand.1 + quicka * 1.5,
            skeleton_attr.hand.2 - quick * 4.0,
        );
        next.l_hand.ori = Quaternion::rotation_x(2.2 + quicka * 0.5);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(
            skeleton_attr.hand.0,
            skeleton_attr.hand.1 - quicka * 1.5,
            skeleton_attr.hand.2 + quick * 4.0,
        );
        next.r_hand.ori = Quaternion::rotation_x(2.2 - quicka * 0.5);
        next.r_hand.scale = Vec3::one();

        next.l_foot.offset = Vec3::new(
            -skeleton_attr.foot.0,
            1.0 + skeleton_attr.foot.1,
            skeleton_attr.foot.2 + quick * 2.5,
        );
        next.l_foot.ori = Quaternion::rotation_x(0.2 - quicka * 0.5);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(
            skeleton_attr.foot.0,
            1.0 + skeleton_attr.foot.1,
            skeleton_attr.foot.2 - quick * 2.5,
        );
        next.r_foot.ori = Quaternion::rotation_x(0.2 + quicka * 0.5);
        next.r_foot.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(
            -skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.l_shoulder.ori = Quaternion::rotation_x(smootha * 0.15);
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.r_shoulder.ori = Quaternion::rotation_x(smooth * 0.15);
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.glider.offset = Vec3::new(0.0, 0.0, 10.0);
        next.glider.scale = Vec3::one() * 0.0;

        next.main.offset = Vec3::new(-7.0, -5.0, 18.0);
        next.main.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57 + smootha * 0.25);
        next.main.scale = Vec3::one();

        next.second.offset = Vec3::new(0.0, 0.0, 0.0);
        next.second.ori = Quaternion::rotation_y(0.0);
        next.second.scale = Vec3::one() * 0.0;

        next.lantern.offset = Vec3::new(
            skeleton_attr.lantern.0,
            skeleton_attr.lantern.1,
            skeleton_attr.lantern.2,
        );
        next.lantern.ori =
            Quaternion::rotation_x(smooth * -0.3) * Quaternion::rotation_y(smooth * -0.3);
        next.lantern.scale = Vec3::one() * 0.65;

        next.torso.offset = Vec3::new(0.0, -0.2 + smooth * -0.08, 0.4) * skeleton_attr.scaler;
        next.torso.ori = Quaternion::rotation_x(0.0) * Quaternion::rotation_y(0.0);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next.control.scale = Vec3::one();

        next.l_control.scale = Vec3::one();

        next.r_control.scale = Vec3::one();
        next
    }
}
