use super::{
    super::{Animation, SkeletonAttr},
    BipedLargeSkeleton,
};
//use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Skeleton = BipedLargeSkeleton;
    type Dependency = (f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        _anim_time: f64,
        _rate: &mut f32,
        _skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        next.head.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.head.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.head.scale = Vec3::one() / 10.88;

        next.upper_torso.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.upper_torso.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.upper_torso.scale = Vec3::one() / 10.88;

        next.lower_torso.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.lower_torso.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.lower_torso.scale = Vec3::one() / 10.88;

        next.shoulder_l.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.shoulder_l.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.shoulder_l.scale = Vec3::one() / 10.88;

        next.shoulder_r.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.shoulder_r.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.shoulder_r.scale = Vec3::one() / 10.88;

        next.hand_l.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.hand_l.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.hand_l.scale = Vec3::one() / 10.88;

        next.hand_r.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.hand_r.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.hand_r.scale = Vec3::one() / 10.88;

        next.leg_l.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.leg_l.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.leg_l.scale = Vec3::one() / 10.88;

        next.leg_r.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.leg_r.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.leg_r.scale = Vec3::one() / 10.88;

        next.foot_l.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.foot_l.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.foot_l.scale = Vec3::one() / 10.88;

        next.foot_r.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.foot_r.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.foot_r.scale = Vec3::one() / 10.88;
        next
    }
}
