// Standard
use std::f32::consts::PI;

// Library
use vek::*;

// Local
use super::{super::Animation, QuadrupedMediumSkeleton, SCALE};

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Skeleton = QuadrupedMediumSkeleton;
    type Dependency = (f32, f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency,
        anim_time: f64,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 14.0).sin();
        let wave_ultra_slow = (anim_time as f32 * 1.0 + PI).sin();
        let wave_ultra_slow_cos = (anim_time as f32 * 1.0 + PI).cos();
        let wave_cos = (anim_time as f32 * 14.0).cos();
        let wave_slow = (anim_time as f32 * 3.5 + PI).sin();
        let wave_slow_cos = (anim_time as f32 * 3.5 + PI).cos();
        let wave_stop = (anim_time as f32 * 5.0).min(PI / 2.0).sin();

        next.wolf_upperhead.offset = Vec3::new(0.0, 7.5, 15.0 + wave_stop * 4.8) / 11.0;
        next.wolf_upperhead.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.25);
        next.wolf_upperhead.scale = Vec3::one() / 10.88;

        next.wolf_jaw.offset = Vec3::new(0.0, 4.5, 2.0);
        next.wolf_jaw.ori = Quaternion::rotation_x(0.0);
        next.wolf_jaw.scale = Vec3::one() * 1.01;

        next.wolf_lowerhead.offset = Vec3::new(0.0, 3.1, -4.5);
        next.wolf_lowerhead.ori = Quaternion::rotation_x(wave_stop * -0.1);
        next.wolf_lowerhead.scale = Vec3::one() * 0.98;

        next.wolf_tail.offset = Vec3::new(0.0, -12.0, 8.0) / 11.0;
        next.wolf_tail.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.25);
        next.wolf_tail.scale = Vec3::one() / 11.0;

        next.wolf_torsoback.offset =
            Vec3::new(0.0, -9.5 + wave_stop * 1.0, 11.0 + wave_stop * 2.2) / 11.0;
        next.wolf_torsoback.ori = Quaternion::rotation_x(wave_slow * -0.25);
        next.wolf_torsoback.scale = Vec3::one() / 11.0;

        next.wolf_torsomid.offset = Vec3::new(0.0, 0.0, 12.0 + wave_stop * 3.6) / 11.0;
        next.wolf_torsomid.ori = Quaternion::rotation_x(wave_slow * -0.25);
        next.wolf_torsomid.scale = Vec3::one() / 10.9;

        next.wolf_ears.offset = Vec3::new(0.0, 0.75, 6.25);
        next.wolf_ears.ori = Quaternion::rotation_x(0.0);
        next.wolf_ears.scale = Vec3::one() * 1.05;

        next.wolf_LFFoot.offset =
            Vec3::new(-5.0, 5.0 + wave_stop * 3.0, 5.0 + wave_stop * 7.0) / 11.0;
        next.wolf_LFFoot.ori = Quaternion::rotation_x(wave_stop * 1.0 + wave * 0.15);
        next.wolf_LFFoot.scale = Vec3::one() / 11.0;

        next.wolf_RFFoot.offset =
            Vec3::new(5.0, 5.0 - wave_stop * 3.0, 5.0 + wave_stop * 5.0) / 11.0;
        next.wolf_RFFoot.ori = Quaternion::rotation_x(wave_stop * -1.0 + wave * 0.15);
        next.wolf_RFFoot.scale = Vec3::one() / 11.0;

        next.wolf_LBFoot.offset =
            Vec3::new(-5.0, -10.0 - wave_stop * 2.0, 5.0 + wave_stop * 0.0) / 11.0;
        next.wolf_LBFoot.ori = Quaternion::rotation_x(wave_stop * -1.0 + wave * 0.15);
        next.wolf_LBFoot.scale = Vec3::one() / 11.0;

        next.wolf_RBFoot.offset =
            Vec3::new(5.0, -10.0 + wave_stop * 2.0, 5.0 + wave_stop * 2.0) / 11.0;
        next.wolf_RBFoot.ori = Quaternion::rotation_x(wave_stop * 1.0 + wave * 0.15);
        next.wolf_RBFoot.scale = Vec3::one() / 11.0;

        next
    }
}
