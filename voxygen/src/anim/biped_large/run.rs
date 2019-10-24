use super::{
    super::{Animation, SkeletonAttr},
    BipedLargeSkeleton,
};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Skeleton = BipedLargeSkeleton;
    type Dependency = (f32, f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (_velocity, global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        _skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave_ultra_slow = (anim_time as f32 * 1.0 + PI).sin();
        let wave_ultra_slow_cos = (anim_time as f32 * 1.0 + PI).cos();
        let wave_slow = (anim_time as f32 * 3.5 + PI).sin();
        let wave_slow_cos = (anim_time as f32 * 3.5 + PI).cos();

        let duck_look = Vec2::new(
            ((global_time + anim_time) as f32 / 8.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.5,
            ((global_time + anim_time) as f32 / 8.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.25,
        );


        next.knight_head.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.knight_head.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.knight_head.scale = Vec3::one() / 10.88;

        next.knight_upper_torso.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.knight_upper_torso.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.knight_upper_torso.scale = Vec3::one() / 10.88;

        next.knight_lower_torso.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.knight_lower_torso.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.knight_lower_torso.scale = Vec3::one() / 10.88;

        next.knight_shoulder_l.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.knight_shoulder_l.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.knight_shoulder_l.scale = Vec3::one() / 10.88;

        next.knight_shoulder_r.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.knight_shoulder_r.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.knight_shoulder_r.scale = Vec3::one() / 10.88;

        next.knight_hand_l.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.knight_hand_l.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.knight_hand_l.scale = Vec3::one() / 10.88;

        next.knight_hand_r.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.knight_hand_r.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.knight_hand_r.scale = Vec3::one() / 10.88;

        next.knight_leg_l.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.knight_leg_l.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.knight_leg_l.scale = Vec3::one() / 10.88;

        next.knight_leg_r.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.knight_leg_r.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.knight_leg_r.scale = Vec3::one() / 10.88;

        next.knight_foot_l.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.knight_foot_l.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.knight_foot_l.scale = Vec3::one() / 10.88;

        next.knight_foot_r.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.knight_foot_r.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.knight_foot_r.scale = Vec3::one() / 10.88;
        next
    }
}
