use super::{super::Animation, BirdSmallSkeleton, SkeletonAttr};
//use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Skeleton = BirdSmallSkeleton;
    type Dependency = f64;

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

        next.torso.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.torso.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.torso.scale = Vec3::one() / 10.88;

        next.wing_l.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.wing_l.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.wing_l.scale = Vec3::one() / 10.88;

        next.wing_r.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.wing_r.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.wing_r.scale = Vec3::one() / 10.88;

        next
    }
}
