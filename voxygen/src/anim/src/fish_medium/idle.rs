use super::{super::Animation, FishMediumSkeleton, SkeletonAttr};
//use std::{f32::consts::PI, ops::Mul};
use super::super::vek::*;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency = f64;
    type Skeleton = FishMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"fish_medium_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "fish_medium_idle")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        _anim_time: f64,
        _rate: &mut f32,
        _skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        next.head.scale = Vec3::one() / 10.88;
        next.torso.scale = Vec3::one() * 1.01;
        next.rear.scale = Vec3::one() * 0.98;
        next.tail.scale = Vec3::one() / 11.0;
        next.fin_l.scale = Vec3::one() / 11.0;
        next.fin_r.scale = Vec3::one() / 10.5;

        next.head.position = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.head.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);

        next.torso.position = Vec3::new(0.0, 4.5, 2.0);
        next.torso.orientation = Quaternion::rotation_x(0.0);

        next.rear.position = Vec3::new(0.0, 3.1, -4.5);
        next.rear.orientation = Quaternion::rotation_z(0.0);

        next.tail.position = Vec3::new(0.0, -13.0, 8.0) / 11.0;
        next.tail.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);

        next.fin_l.position = Vec3::new(0.0, -11.7, 11.0) / 11.0;
        next.fin_l.orientation = Quaternion::rotation_y(0.0);

        next.fin_r.position = Vec3::new(0.0, 0.0, 12.0) / 11.0;
        next.fin_r.orientation = Quaternion::rotation_y(0.0);
        next
    }
}
