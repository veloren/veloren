// Standard
use std::f32::consts::PI;

// Library
use vek::*;

// Local
use super::{
    CharacterSkeleton,
    super::Animation,
};

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Skeleton = CharacterSkeleton;
    type Dependency = f64;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        time: f64,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (time as f32 * 14.0).sin();
        let fuzzwave = (time as f32 * 12.0).sin();
        let wavecos = (time as f32 * 14.0).cos();
        let wave_slow = (time as f32 * 8.0 + PI).sin();
        let wavecos_slow = (time as f32 * 8.0 + PI).cos();
        let wave_dip = (wave_slow.abs() - 0.5).abs();

        next.head.offset = Vec3::unit_z() * (12.0 + fuzzwave *1.0)/ 11.0;

        next.chest.offset = Vec3::unit_z() * (8.0 + fuzzwave * 0.8) / 11.0;
        next.chest.ori = Quaternion::rotation_z(wave * 0.3);

        next.belt.offset = Vec3::unit_z() * (6.0 + fuzzwave * 0.8)/ 11.0;
        next.belt.ori = Quaternion::rotation_z(wave * 0.3);

        next.shorts.offset = Vec3::unit_z() * (3.0 + fuzzwave * 0.8) / 11.0;
        next.shorts.ori = Quaternion::rotation_z(wave * 0.2);

        next.l_hand.offset = Vec3::new(0.0 - wavecos * 1.0, 7.5, 11.0 - wave * 1.0) / 11.0;
        next.l_hand.ori = Quaternion::rotation_y(wave * -1.8);
        next.r_hand.offset = Vec3::new(0.0 + wavecos * 1.0, -7.5, 11.0 + wave * 1.0) / 11.0;
        next.r_hand.ori = Quaternion::rotation_y(wave * 1.8);

        next.l_foot.offset = Vec3::new(2.5 - wavecos * 4.0, 3.4, 6.0 + wave * 2.9) / 11.0;
        next.l_foot.ori = Quaternion::rotation_y(wave * -1.0);
        next.r_foot.offset = Vec3::new(2.5 + wavecos * 4.0, -3.4, 6.0 - wave * 2.9) / 11.0;
        next.r_foot.ori = Quaternion::rotation_y(wave * 1.0);

        next.back.offset = Vec3::new(-6.0, 16.0, 15.0);
        next.back.ori = Quaternion::rotation_x(2.5);
        next.back.scale = Vec3::one();

        next
    }
}
