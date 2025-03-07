use super::{
    super::{Animation, vek::*},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::ToolKind;
use std::{f32::consts::PI, ops::Mul};

pub struct SteerAnimation;

impl Animation for SteerAnimation {
    type Dependency<'a> = (Option<ToolKind>, Option<ToolKind>, f32, f32);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_steer\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "character_steer"))]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, _second_tool_kind, steer_dir, global_time): Self::Dependency<'_>,
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
        next.chest.orientation = Quaternion::rotation_z(head_look.x * 0.06);

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

        let helm_center = Vec3::new(0.0, 0.6, 0.75) / s_a.scaler * 11.0;

        let rot = steer_dir * 0.5;

        let hand_rotation = Quaternion::rotation_y(rot) * Quaternion::rotation_x(PI / 2.0);

        let hand_offset = Vec3::new(rot.cos(), 0.0, -rot.sin()) * 0.4 / s_a.scaler * 11.0;

        next.hand_l.position = helm_center - hand_offset;
        next.hand_r.position = helm_center + hand_offset;

        let ori_l = Quaternion::rotation_x(
            PI / 2.0 + (next.hand_l.position.z / next.hand_l.position.x).atan(),
        );
        let ori_r = Quaternion::rotation_x(
            PI / 2.0 - (next.hand_r.position.z / next.hand_r.position.x).atan(),
        );

        next.hand_l.orientation = hand_rotation * ori_l;
        next.hand_r.orientation = -hand_rotation * ori_r;

        next.shoulder_l.position = Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_l.orientation = ori_r;
        next.shoulder_r.position = Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_r.orientation = ori_l;

        next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2);
        next.foot_l.orientation = Quaternion::identity();

        next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2);
        next.foot_r.orientation = Quaternion::identity();

        next.glider.position = Vec3::new(0.0, 0.0, 10.0);
        next.glider.scale = Vec3::one() * 0.0;
        next.hold.position = Vec3::new(0.4, -0.3, -5.8);

        if skeleton.holding_lantern {
            next.hand_r.position = Vec3::new(
                s_a.hand.0 - head_look.x * 6.0,
                s_a.hand.1 + 5.0 - head_look.y * 10.0 + slow * 0.15,
                s_a.hand.2 + 12.0 + head_look.y * 6.0 + slow * 0.5,
            );
            next.hand_r.orientation = Quaternion::rotation_x(2.25 + slow * -0.06)
                * Quaternion::rotation_z(0.9)
                * Quaternion::rotation_y(head_look.x * 1.5)
                * Quaternion::rotation_x(head_look.y * 1.5);

            next.lantern.position = Vec3::new(-0.5, -0.5, -2.5);
            next.lantern.orientation = next.hand_r.orientation.inverse();
        }

        next.torso.position = Vec3::new(0.0, 0.0, 0.0);

        next
    }
}
