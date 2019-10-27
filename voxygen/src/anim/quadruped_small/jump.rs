use super::{
    super::{Animation, SkeletonAttr},
    QuadrupedSmallSkeleton,
};
use std::f32::consts::PI;
use vek::*;

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Skeleton = QuadrupedSmallSkeleton;
    type Dependency = (f32, f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (_velocity, _global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        _skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave_slow = (anim_time as f32 * 7.0 + PI).sin();
        let wave_stop = (anim_time as f32 * 4.5).min(PI / 2.0).sin();

        next.head.offset = Vec3::new(0.0, 0.0, -1.5) / 11.0;
        next.head.ori = Quaternion::rotation_x(wave_stop * 0.4);
        next.head.scale = Vec3::one() / 10.5;

        next.chest.offset = Vec3::new(0.0, -9.0, 1.5) / 11.0;
        next.chest.ori = Quaternion::rotation_x(0.0);
        next.chest.scale = Vec3::one() / 11.0;

        next.leg_lf.offset = Vec3::new(-4.5, 3.0, 1.5) / 11.0;
        next.leg_lf.ori = Quaternion::rotation_x(wave_stop * 0.6 - wave_slow * 0.3);
        next.leg_lf.scale = Vec3::one() / 11.0;

        next.leg_rf.offset = Vec3::new(2.5, 3.0, 1.5) / 11.0;
        next.leg_rf.ori = Quaternion::rotation_x(wave_stop * 0.6 - wave_slow * 0.3);
        next.leg_rf.scale = Vec3::one() / 11.0;

        next.leg_lb.offset = Vec3::new(-4.5, -4.0, 2.0) / 11.0;
        next.leg_lb.ori = Quaternion::rotation_x(wave_stop * -0.6 + wave_slow * 0.3);
        next.leg_lb.scale = Vec3::one() / 11.0;

        next.leg_rb.offset = Vec3::new(2.5, -4.0, 2.0) / 11.0;
        next.leg_rb.ori = Quaternion::rotation_x(wave_stop * -0.6 + wave_slow * 0.3);
        next.leg_rb.scale = Vec3::one() / 11.0;

        next
    }
}
