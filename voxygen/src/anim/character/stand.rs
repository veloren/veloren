use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::Tool;
use std::ops::Mul;
use vek::*;

pub struct StandAnimation;

impl Animation for StandAnimation {
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
        let breathe = ((anim_time as f32 * 0.5).sin()).abs();

        let head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 12.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.3,
            ((global_time + anim_time) as f32 / 12.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.15,
        );
        next.head.offset = Vec3::new(
            0.0 + skeleton_attr.neck_right,
            -3.0 + skeleton_attr.neck_forward,
            skeleton_attr.neck_height + 21.0 + slow * 0.3,
        );
        next.head.ori =
            Quaternion::rotation_z(head_look.x) * Quaternion::rotation_x(head_look.y.abs());
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(0.0, 0.0, 7.0 + slow * 0.3);
        next.chest.ori = Quaternion::rotation_z(head_look.x * 0.6);
        next.chest.scale = Vec3::one() * 1.01;

        next.belt.offset = Vec3::new(0.0, 0.0, 5.0 + slow * 0.3);
        next.belt.ori = Quaternion::rotation_z(head_look.x * 0.4);
        next.belt.scale = Vec3::one() + breathe * 0.05;

        next.shorts.offset = Vec3::new(0.0, 0.0, 2.0 + slow * 0.3);
        next.shorts.ori = Quaternion::rotation_x(0.0);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(-7.0, -0.25 + slow * 0.15, 5.0 + slow * 0.5);

        next.l_hand.ori = Quaternion::rotation_x(0.0 + slow * -0.06);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(7.0, -0.25 + slow * 0.15, 5.0 + slow * 0.5);
        next.r_hand.ori = Quaternion::rotation_x(0.0 + slow * -0.06);
        next.r_hand.scale = Vec3::one();

        next.l_foot.offset = Vec3::new(-3.4, -0.1, 8.0);
        next.l_foot.ori = Quaternion::identity();
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(3.4, -0.1, 8.0);
        next.r_foot.ori = Quaternion::identity();
        next.r_foot.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(-5.0, 0.0, 5.0);
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = (Vec3::one() + breathe * -0.05) * 1.15;

        next.r_shoulder.offset = Vec3::new(5.0, 0.0, 5.0);
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = (Vec3::one() + breathe * -0.05) * 1.15;

        next.glider.offset = Vec3::new(0.0, 5.0, 0.0);
        next.glider.ori = Quaternion::rotation_y(0.0);
        next.glider.scale = Vec3::one() * 0.0;

        next.main.offset = Vec3::new(
            -7.0 + skeleton_attr.weapon_x,
            -5.0 + skeleton_attr.weapon_y,
            18.0,
        );
        next.main.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
        next.main.scale = Vec3::one();

        next.second.offset = Vec3::new(
            0.0 + skeleton_attr.weapon_x,
            0.0 + skeleton_attr.weapon_y,
            0.0,
        );
        next.second.ori = Quaternion::rotation_y(0.0);
        next.second.scale = Vec3::one() * 0.0;

        next.lantern.offset = Vec3::new(0.0, 0.0, 0.0);
        next.lantern.ori = Quaternion::rotation_x(0.0);
        next.lantern.scale = Vec3::one() * 0.0;

        next.torso.offset = Vec3::new(0.0, -0.1, 0.1) * skeleton_attr.scaler;
        next.torso.ori = Quaternion::rotation_x(0.0);
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
        next
    }
}
