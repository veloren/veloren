use super::{super::Animation, FishMediumSkeleton, SkeletonAttr};
//use std::f32::consts::PI;
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (f32, f64);
    type Skeleton = FishMediumSkeleton;

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

        next.torso.offset = Vec3::new(0.0, 4.5, 2.0);
        next.torso.ori = Quaternion::rotation_x(0.0);
        next.torso.scale = Vec3::one() * 1.01;

        next.rear.offset = Vec3::new(0.0, 3.1, -4.5);
        next.rear.ori = Quaternion::rotation_z(0.0);
        next.rear.scale = Vec3::one() * 0.98;

        next.tail.offset = Vec3::new(0.0, -13.0, 8.0) / 11.0;
        next.tail.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.tail.scale = Vec3::one() / 11.0;

        next.fin_l.offset = Vec3::new(0.0, -11.7, 11.0) / 11.0;
        next.fin_l.ori = Quaternion::rotation_y(0.0);
        next.fin_l.scale = Vec3::one() / 11.0;

        next.fin_r.offset = Vec3::new(0.0, 0.0, 12.0) / 11.0;
        next.fin_r.ori = Quaternion::rotation_y(0.0);
        next.fin_r.scale = Vec3::one() / 10.5;
        next
    }
}
