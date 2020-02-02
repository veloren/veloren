use super::{super::Animation, BirdMediumSkeleton, SkeletonAttr};
//use std::f32::consts::PI;
use vek::*;

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Dependency = (f32, f64);
    type Skeleton = BirdMediumSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        _anim_time: f64,
        _rate: &mut f32,
        _skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        next.head.offset = Vec3::new(0.0, 0.0, 0.0) / 11.0;
        next.head.ori = Quaternion::rotation_z(0.0);
        next.head.scale = Vec3::one();

        next.torso.offset = Vec3::new(0.0, 0.0, 0.0);
        next.torso.ori = Quaternion::rotation_x(0.0);
        next.torso.scale = Vec3::one() / 11.0;

        next.tail.offset = Vec3::new(0.0, 0.0, 0.0);
        next.tail.ori = Quaternion::rotation_z(0.0);
        next.tail.scale = Vec3::one();

        next.wing_l.offset = Vec3::new(0.0, 0.0, 0.0) / 11.0;
        next.wing_l.ori = Quaternion::rotation_z(0.0);
        next.wing_l.scale = Vec3::one();

        next.wing_r.offset = Vec3::new(0.0, 0.0, 0.0) / 11.0;
        next.wing_r.ori = Quaternion::rotation_y(0.0);
        next.wing_r.scale = Vec3::one();

        next.leg_l.offset = Vec3::new(0.0, 0.0, 0.0) / 11.0;
        next.leg_l.ori = Quaternion::rotation_y(0.0);
        next.leg_l.scale = Vec3::one() / 11.0;

        next.leg_r.offset = Vec3::new(0.0, 0.0, 0.0) / 11.0;
        next.leg_r.ori = Quaternion::rotation_x(0.0);
        next.leg_r.scale = Vec3::one() / 11.0;
        next
    }
}
