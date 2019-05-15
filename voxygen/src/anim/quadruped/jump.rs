// Standard
use std::f32::consts::PI;

// Library
use vek::*;

// Local
use super::{super::Animation, QuadrupedSkeleton, SCALE};

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Skeleton = QuadrupedSkeleton;
    type Dependency = (f32, f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (velocity, global_time): Self::Dependency,
        anim_time: f64,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 14.0).sin();
        let wavetest = (wave.cbrt());
        let fuzzwave = (anim_time as f32 * 12.0).sin();
        let wavecos = (anim_time as f32 * 14.0).cos();
        let wave_slow = (anim_time as f32 * 7.0 + PI).sin();
        let wavecos_slow = (anim_time as f32 * 8.0 + PI).cos();
        let wave_dip = (wave_slow.abs() - 0.5).abs();
        let wave_stop = (anim_time as f32 * 4.5).min(PI / 2.0).sin();


        next.pighead.offset = Vec3::new(0.0, 0.0, -1.5 ) / 11.0;
        next.pighead.ori = Quaternion::rotation_x(wave_stop * 0.4);
        next.pighead.scale = Vec3::one() / 10.5;

        next.pigchest.offset = Vec3::new(0.0, -9.0, 1.5) / 11.0;
        next.pigchest.ori = Quaternion::rotation_x(0.0);
        next.pigchest.scale = Vec3::one() / 11.0;

        next.piglf_leg.offset = Vec3::new(-4.5, 3.0, 1.5) / 11.0;
        next.piglf_leg.ori = Quaternion::rotation_x(wave_stop * 0.6);
        next.piglf_leg.scale = Vec3::one() / 11.0;

        next.pigrf_leg.offset = Vec3::new(2.5, 3.0, 1.5) / 11.0;
        next.pigrf_leg.ori = Quaternion::rotation_x(wave_stop * 0.6 - wave_slow * 0.3);
        next.pigrf_leg.scale = Vec3::one() / 11.0;

        next.piglb_leg.offset = Vec3::new(-4.5, -4.0, 2.0) / 11.0;
        next.piglb_leg.ori = Quaternion::rotation_x(wave_stop * -0.6 + wave_slow * 0.3);
        next.piglb_leg.scale = Vec3::one() / 11.0;

        next.pigrb_leg.offset = Vec3::new(2.5, -4.0, 2.0) / 11.0;
        next.pigrb_leg.ori = Quaternion::rotation_x(wave_stop * -0.6 + wave_slow * 0.3);
        next.pigrb_leg.scale = Vec3::one() / 11.0;

        next
    }
}
