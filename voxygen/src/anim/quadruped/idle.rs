// Standard
use std::{f32::consts::PI, ops::Mul};

// Library
use vek::*;

// Local
use super::{super::Animation, QuadrupedSkeleton, SCALE};

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Skeleton = QuadrupedSkeleton;
    type Dependency = (f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency,
        anim_time: f64,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 14.0).sin();
        let wavetest = (wave.cbrt());
        let waveultra_slow = (anim_time as f32 * 1.0 + PI).sin();
        let waveultracos_slow = (anim_time as f32 * 1.0 + PI).cos();
        let fuzzwave = (anim_time as f32 * 12.0).sin();
        let wavecos = (anim_time as f32 * 14.0).cos();
        let wave_slow = (anim_time as f32 * 3.5 + PI).sin();
        let wavecos_slow = (anim_time as f32 * 3.5 + PI).cos();
        let wave_dip = (wave_slow.abs() - 0.5).abs();

        let pighead_look = Vec2::new(
            ((global_time + anim_time) as f32 / 8.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.5,
            ((global_time + anim_time) as f32 / 8.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.25,
        );

        next.pighead.offset = Vec3::new(0.0, -2.0, -1.5 + wave * 0.2) / 11.0;
        next.pighead.ori = Quaternion::rotation_z(pighead_look.x)
            * Quaternion::rotation_x(pighead_look.y + wavecos_slow * 0.03);
        next.pighead.scale = Vec3::one() / 10.5;

        next.pigchest.offset = Vec3::new(wave_slow * 0.05, -9.0, 1.5 + wavecos_slow * 0.4) / 11.0;
        next.pigchest.ori = Quaternion::rotation_y(wave_slow * 0.05);
        next.pigchest.scale = Vec3::one() / 11.0;

        next.piglf_leg.offset = Vec3::new(-4.5, 2.0, 1.5) / 11.0;
        next.piglf_leg.ori = Quaternion::rotation_x(wave_slow * 0.08);
        next.piglf_leg.scale = Vec3::one() / 11.0;

        next.pigrf_leg.offset = Vec3::new(2.5, 2.0, 1.5) / 11.0;
        next.pigrf_leg.ori = Quaternion::rotation_x(wavecos_slow * 0.08);
        next.pigrf_leg.scale = Vec3::one() / 11.0;

        next.piglb_leg.offset = Vec3::new(-4.5, -3.0, 1.5) / 11.0;
        next.piglb_leg.ori = Quaternion::rotation_x(wavecos_slow * 0.08);
        next.piglb_leg.scale = Vec3::one() / 11.0;

        next.pigrb_leg.offset = Vec3::new(2.5, -3.0, 1.5) / 11.0;
        next.pigrb_leg.ori = Quaternion::rotation_x(wave_slow * 0.08);
        next.pigrb_leg.scale = Vec3::one() / 11.0;

        next
    }
}
