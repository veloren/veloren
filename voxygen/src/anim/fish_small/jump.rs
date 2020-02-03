use super::{super::Animation, FishSmallSkeleton, SkeletonAttr};
//use std::f32::consts::PI;
use vek::*;

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Dependency = (f32, f64);
    type Skeleton = FishSmallSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        _anim_time: f64,
        _rate: &mut f32,
        _skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        next.torso.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.torso.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.torso.scale = Vec3::one() / 10.88;

        next.tail.offset = Vec3::new(0.0, 4.5, 2.0);
        next.tail.ori = Quaternion::rotation_x(0.0);
        next.tail.scale = Vec3::one() * 1.01;

        next
    }
}
