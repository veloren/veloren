// Standard
use std::f32::consts::PI;

// Library
use vek::*;

// Local
use super::{
    CharacterSkeleton,
    super::Animation,
    SCALE
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

        let wave = (time as f32 * 16.0).sin();
        let wavetest = (wave.cbrt());
        let fuzzwave = (time as f32 * 12.0).sin();
        let wavecos = (time as f32 * 16.0).cos();
        let wave_slow = (time as f32 * 8.0 + PI).sin();
        let wavecos_slow = (time as f32 * 8.0 + PI).cos();
        let wave_dip = (wave_slow.abs() - 0.5).abs();


        next.head.offset = Vec3::unit_z() * (12.0 + wave *1.3)/ SCALE;
        next.chest.scale = Vec3::one() * 1.0;

        next.chest.offset = Vec3::new(2.5, 0.0, 8.0 + wave * 1.1)  / SCALE;
        next.chest.ori = Quaternion::rotation_z(wavecos * 0.2);
        next.chest.scale = Vec3::one() / SCALE;

        next.belt.offset = Vec3::new(2.5, 0.0, 6.0 + wave * 1.1) / SCALE;
        next.belt.ori = Quaternion::rotation_z(wavecos * 0.2);
        next.belt.scale = Vec3::one() /SCALE;

        next.shorts.offset = Vec3::new(2.5, 0.0, 3.0 + wave * 1.1) / SCALE;
        next.shorts.ori = Quaternion::rotation_z(wavecos * 0.6);
        next.shorts.scale = Vec3::one() /SCALE;

        next.l_hand.offset = Vec3::new(2.0 - wavecos * 1.0, 7.5, 11.0 - wave * 1.0) / SCALE;
        next.l_hand.ori = Quaternion::rotation_y(wavecos * -1.8);
        next.r_hand.offset = Vec3::new(2.0 + wavecos * 1.0, -7.5, 11.0 + wave * 1.0) / SCALE;
        next.r_hand.ori = Quaternion::rotation_y(wavecos * 1.8);

        next.l_foot.offset = Vec3::new(3.5 - wave * 1.0, 3.4, 6.0) / SCALE;
        next.l_foot.ori = Quaternion::rotation_y(-0.0 + wave * 1.5);
        next.l_foot.scale = Vec3::one() / SCALE;

        next.r_foot.offset = Vec3::new(3.5 + wave * 1.0, -3.4, 6.0) / SCALE;
        next.r_foot.ori = Quaternion::rotation_y(-0.0 - wave * 1.5);

        next.back.offset = Vec3::new(-4.5, 12.0, 11.0);
        next.back.ori = Quaternion::rotation_x(2.5);
        next.back.scale = Vec3::one();

        next.torso.offset = Vec3::new(0.0, 0.0, 0.0);
        next.torso.ori = Quaternion::rotation_y(0.2 + wave * 0.1);


        next.torso.scale = Vec3::one();



        next

    }
}
