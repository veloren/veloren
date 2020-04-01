use super::{super::Animation, QuadrupedSmallSkeleton, SkeletonAttr};
use std::f32::consts::PI;
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (f32, f64);
    type Skeleton = QuadrupedSmallSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (_velocity, _global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let slow = (anim_time as f32 * 14.0).sin();
        let fast = (anim_time as f32 * 20.0).sin();
        let fast_alt = (anim_time as f32 * 20.0 + PI / 2.0).sin();
        let slow_alt = (anim_time as f32 * 14.0 + PI / 2.0).sin();

        next.head.offset =
            Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1 + slow * 1.5) / 11.0;
        next.head.ori =
            Quaternion::rotation_x(0.2 + slow * 0.05) * Quaternion::rotation_y(slow_alt * 0.03);
        next.head.scale = Vec3::one() / 10.5;

        next.chest.offset = Vec3::new(
            0.0,
            skeleton_attr.chest.0,
            skeleton_attr.chest.1 + slow_alt * 1.2,
        ) / 11.0;
        next.chest.ori = Quaternion::rotation_x(slow * 0.1);
        next.chest.scale = Vec3::one() / 11.0;

        next.leg_lf.offset = Vec3::new(
            -skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1 + fast * 0.8,
            skeleton_attr.feet_f.2 + fast_alt * 1.5,
        ) / 11.0;
        next.leg_lf.ori = Quaternion::rotation_x(fast * 0.3);
        next.leg_lf.scale = Vec3::one() / 11.0;

        next.leg_rf.offset = Vec3::new(
            skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1 + fast_alt * -0.8,
            skeleton_attr.feet_f.2 + fast * 1.5,
        ) / 11.0;
        next.leg_rf.ori = Quaternion::rotation_x(fast_alt * -0.3);
        next.leg_rf.scale = Vec3::one() / 11.0;

        next.leg_lb.offset = Vec3::new(
            -skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1 + fast_alt * -0.8,
            skeleton_attr.feet_b.2 + fast * 1.5,
        ) / 11.0;
        next.leg_lb.ori = Quaternion::rotation_x(fast_alt * -0.3);
        next.leg_lb.scale = Vec3::one() / 11.0;

        next.leg_rb.offset = Vec3::new(
            skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1 + fast * 0.8,
            skeleton_attr.feet_b.2 + fast_alt * 1.5,
        ) / 11.0;
        next.leg_rb.ori = Quaternion::rotation_x(fast * 0.3);
        next.leg_rb.scale = Vec3::one() / 11.0;

        next
    }
}
