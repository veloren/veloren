// Standard
use std::f32::consts::PI;

// Library
use vek::*;

// Local
use super::{super::Animation, QuadrupedSkeleton, SCALE};

pub struct RunAnimation;

impl Animation for RunAnimation {
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

        next.pighead.offset = Vec3::new(5.5, 2.0, 11.0 + wavecos * 1.3);
        next.pighead.ori = Quaternion::rotation_x(0.15);
        next.pighead.scale = Vec3::one();

        next.pigchest.offset = Vec3::new(5.5, 0.0, 7.0 + wavecos * 1.1);
        next.pigchest.ori = Quaternion::rotation_z(wave * 0.1);
        next.pigchest.scale = Vec3::one();

        next.piglf_leg.offset = Vec3::new(5.5, 0.0, 5.0 + wavecos * 1.1);
        next.piglf_leg.ori = Quaternion::rotation_z(wave * 0.25);
        next.piglf_leg.scale = Vec3::one();

        next.pigrf_leg.offset = Vec3::new(5.5, 0.0, 2.0 + wavecos * 1.1);
        next.pigrf_leg.ori = Quaternion::rotation_z(wave * 0.6);
        next.pigrf_leg.scale = Vec3::one();

        next.piglb_leg.offset = Vec3::new(-6.0, 0.0 + wavecos * 2.5, 11.0 - wave * 1.5);
        next.piglb_leg.ori = Quaternion::rotation_x(wavecos * 0.9);
        next.piglb_leg.scale = Vec3::one();

        next.pigrb_leg.offset = Vec3::new(9.0, 0.0 - wavecos * 2.5, 11.0 + wave * 1.5);
        next.pigrb_leg.ori = Quaternion::rotation_x(wavecos * -0.9);
        next.pigrb_leg.scale = Vec3::one();

        next
    }
}
