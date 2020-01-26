use super::{super::Animation, QuadrupedMediumSkeleton, SkeletonAttr};
use std::f32::consts::PI;
use vek::*;

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Skeleton = QuadrupedMediumSkeleton;
    type Dependency = (f32, f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        _skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 14.0).sin();
        let wave_slow = (anim_time as f32 * 3.5 + PI).sin();
        let wave_stop = (anim_time as f32 * 5.0).min(PI / 2.0).sin();

        next.head_upper.offset = Vec3::new(0.0, 7.5, 15.0 + wave_stop * 4.8) / 11.0;
        next.head_upper.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.25);
        next.head_upper.scale = Vec3::one() / 10.88;

        next.head_lower.offset = Vec3::new(0.0, 3.1, -4.5);
        next.head_lower.ori = Quaternion::rotation_x(wave_stop * -0.1);
        next.head_lower.scale = Vec3::one() * 0.98;

        next.jaw.offset = Vec3::new(0.0, 4.5, 2.0);
        next.jaw.ori = Quaternion::rotation_x(0.0);
        next.jaw.scale = Vec3::one() * 1.01;

        next.tail.offset = Vec3::new(0.0, -12.0, 8.0) / 11.0;
        next.tail.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.25);
        next.tail.scale = Vec3::one() / 11.0;

        next.torso_back.offset =
            Vec3::new(0.0, -9.5 + wave_stop * 1.0, 11.0 + wave_stop * 2.2) / 11.0;
        next.torso_back.ori = Quaternion::rotation_x(wave_slow * -0.25);
        next.torso_back.scale = Vec3::one() / 11.0;

        next.torso_mid.offset = Vec3::new(0.0, 0.0, 12.0 + wave_stop * 3.6) / 11.0;
        next.torso_mid.ori = Quaternion::rotation_x(wave_slow * -0.25);
        next.torso_mid.scale = Vec3::one() / 10.5;

        next.ears.offset = Vec3::new(0.0, 0.75, 6.25);
        next.ears.ori = Quaternion::rotation_x(0.0);
        next.ears.scale = Vec3::one() * 1.05;

        next.foot_lf.offset = Vec3::new(-5.0, 5.0 + wave_stop * 3.0, 5.0 + wave_stop * 7.0) / 11.0;
        next.foot_lf.ori = Quaternion::rotation_x(wave_stop * 1.0 + wave * 0.15);
        next.foot_lf.scale = Vec3::one() / 11.0;

        next.foot_rf.offset = Vec3::new(5.0, 5.0 - wave_stop * 3.0, 5.0 + wave_stop * 5.0) / 11.0;
        next.foot_rf.ori = Quaternion::rotation_x(wave_stop * -1.0 + wave * 0.15);
        next.foot_rf.scale = Vec3::one() / 11.0;

        next.foot_lb.offset =
            Vec3::new(-5.0, -10.0 - wave_stop * 2.0, 5.0 + wave_stop * 0.0) / 11.0;
        next.foot_lb.ori = Quaternion::rotation_x(wave_stop * -1.0 + wave * 0.15);
        next.foot_lb.scale = Vec3::one() / 11.0;

        next.foot_rb.offset = Vec3::new(5.0, -10.0 + wave_stop * 2.0, 5.0 + wave_stop * 2.0) / 11.0;
        next.foot_rb.ori = Quaternion::rotation_x(wave_stop * 1.0 + wave * 0.15);
        next.foot_rb.scale = Vec3::one() / 11.0;

        next
    }
}
