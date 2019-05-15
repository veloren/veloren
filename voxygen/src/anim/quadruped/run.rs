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
        let wavequick = (anim_time as f32 * 20.0).sin();
        let wavequickcos = (anim_time as f32 * 20.0).cos();
        let wavetest = (wave.cbrt());
        let fuzzwave = (anim_time as f32 * 12.0).sin();
        let wavecos = (anim_time as f32 * 14.0).cos();
        let wave_slow = (anim_time as f32 * 7.0 + PI).sin();
        let wavecos_slow = (anim_time as f32 * 8.0 + PI).cos();
        let wave_dip = (wave_slow.abs() - 0.5).abs();

        next.pighead.offset = Vec3::new(0.0, 9.0, -1.5 + wave * 1.5) / 11.0;
        next.pighead.ori = Quaternion::rotation_x(0.2 + wave * 0.05) * Quaternion::rotation_y(wavecos * 0.03);
        next.pighead.scale = Vec3::one() / 10.5;

        next.pigchest.offset = Vec3::new(0.0, 0.0, 1.5 + wavecos * 1.2) / 11.0;
        next.pigchest.ori = Quaternion::rotation_x(wave * 0.1);
        next.pigchest.scale = Vec3::one() / 11.0;

        next.piglf_leg.offset = Vec3::new(-4.5, 11.0 + wavequick * 0.8, 2.5 + wavequickcos * 1.5) / 11.0;
        next.piglf_leg.ori = Quaternion::rotation_x(wavequick *  0.3);
        next.piglf_leg.scale = Vec3::one() / 11.0;

        next.pigrf_leg.offset = Vec3::new(2.5, 11.0 - wavequickcos * 0.8, 2.5 + wavequick * 1.5) / 11.0;
        next.pigrf_leg.ori = Quaternion::rotation_x(wavequickcos * -0.3);
        next.pigrf_leg.scale = Vec3::one() / 11.0;

        next.piglb_leg.offset = Vec3::new(-4.5, 6.0 - wavequickcos * 0.8, 2.5 + wavequick * 1.5) / 11.0;
        next.piglb_leg.ori = Quaternion::rotation_x(wavequickcos * -0.3);
        next.piglb_leg.scale = Vec3::one() / 11.0;

        next.pigrb_leg.offset = Vec3::new(2.5, 6.0 + wavequick * 0.8, 2.5 + wavequickcos * 1.5) / 11.0;
        next.pigrb_leg.ori = Quaternion::rotation_x(wavequick *  0.3);
        next.pigrb_leg.scale = Vec3::one() / 11.0;

        next
    }
}
