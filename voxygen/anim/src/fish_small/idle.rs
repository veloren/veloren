use super::{super::Animation, FishSmallSkeleton, SkeletonAttr};
//use std::{f32::consts::PI, ops::Mul};
use super::super::vek::*;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency = f64;
    type Skeleton = FishSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"fish_small_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "fish_small_idle")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        _anim_time: f64,
        _rate: &mut f32,
        _skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        next.torso.position = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.torso.scale = Vec3::one() / 10.88;

        next.tail.position = Vec3::new(0.0, 4.5, 2.0);
        next.tail.orientation = Quaternion::rotation_x(0.0);
        next.tail.scale = Vec3::one() * 1.01;

        next
    }
}
