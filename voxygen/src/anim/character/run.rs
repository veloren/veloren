// Standard
use std::f32::consts::PI;

// Library
use vek::*;

// Local
use super::{super::Animation, CharacterSkeleton, SCALE};

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Skeleton = CharacterSkeleton;
    type Dependency = f64;

    fn update_skeleton(skeleton: &Self::Skeleton, time: f64) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (time as f32 * 14.0).sin();
        let wavetest = (wave.cbrt());
        let fuzzwave = (time as f32 * 12.0).sin();
        let wavecos = (time as f32 * 14.0).cos();
        let wave_slow = (time as f32 * 7.0 + PI).sin();
        let wavecos_slow = (time as f32 * 8.0 + PI).cos();
        let wave_dip = (wave_slow.abs() - 0.5).abs();

        next.head.offset = Vec3::new(6.0, 0.0, 12.0 + wavecos * 1.3);
        next.head.ori = Quaternion::rotation_y(-0.15);
        next.head.scale = Vec3::one();

        next.chest.offset = Vec3::new(2.5, 0.0, 8.0 + wavecos * 1.1);
        next.chest.ori = Quaternion::rotation_z(wave * 0.1);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(2.5, 0.0, 6.0 + wavecos * 1.1);
        next.belt.ori = Quaternion::rotation_z(wave * 0.25);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(2.5, 0.0, 3.0 + wavecos * 1.1);
        next.shorts.ori = Quaternion::rotation_z(wave * 0.6);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(2.0 - wavecos * 2.5, 7.5, 12.0 + wave * 1.5);
        next.l_hand.ori = Quaternion::rotation_y(wavecos * 0.9);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(2.0 + wavecos * 2.5, -7.5, 12.0 - wave * 1.5);
        next.r_hand.ori = Quaternion::rotation_y(wavecos * -0.9);
        next.r_hand.scale = Vec3::one();

        next.l_foot.offset = Vec3::new(3.5 + wave * 1.0, 3.4, 6.0);
        next.l_foot.ori = Quaternion::rotation_y(-0.0 - wave * 1.5);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(3.5 - wave * 1.0, -3.4, 6.0);
        next.r_foot.ori = Quaternion::rotation_y(-0.0 + wave * 1.5);
        next.r_foot.scale = Vec3::one();

        next.weapon.offset = Vec3::new(-5.0, 14.0, 13.0);
        next.weapon.ori = Quaternion::rotation_x(2.5);
        next.weapon.scale = Vec3::one();

        next.torso.offset = Vec3::new(-0.5, 0.0, 0.0);
        next.torso.ori = Quaternion::rotation_y(0.25 + wavecos * 0.1);
        next.torso.scale = Vec3::one() / 11.0;

        next.l_shoulder.offset = Vec3::new(3.0, 6.0, 18.0);
        next.l_shoulder.ori = Quaternion::rotation_y(0.0);
        next.l_shoulder.scale = Vec3::one();

        next.r_shoulder.offset = Vec3::new(3.0, -6.0, 18.0);
        next.r_shoulder.ori = Quaternion::rotation_y(0.0);
        next.r_shoulder.scale = Vec3::one();

        next
    }
}
