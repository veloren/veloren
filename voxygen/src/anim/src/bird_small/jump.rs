use super::{super::Animation, BirdSmallSkeleton, SkeletonAttr};
//use std::f32::consts::PI;
use super::super::vek::*;

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Dependency = (f32, f64);
    type Skeleton = BirdSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"bird_small_jump\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "bird_small_jump")]
    fn update_skeleton_inner(
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
