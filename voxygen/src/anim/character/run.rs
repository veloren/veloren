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

        let wave = (time as f32 * 12.0).sin();
	    let wavecos = (time as f32 * 12.0).cos();
        let wave_slow = (time as f32 * 6.0 + PI).sin();
        let wavecos_slow = (time as f32 * 6.0 + PI).cos();
        let wave_dip = (wave_slow.abs() - 0.5).abs();

        next.head.offset = Vec3::unit_z() * 13.0 / SCALE;
        next.head.ori = Quaternion::rotation_z(wave * 0.3);

        next.chest.offset = Vec3::unit_z() * 9.0 / SCALE;
        next.chest.ori = Quaternion::rotation_z(wave * 0.3);

        next.belt.offset = Vec3::unit_z() * 7.0 / SCALE;
        next.belt.ori = Quaternion::rotation_z(wave * 0.2);

        next.shorts.offset = Vec3::unit_z() * 4.0 / SCALE;
        next.shorts.ori = Quaternion::rotation_z(wave * 0.1);

        next.l_hand.offset = Vec3::new(-6.0 - wave_dip * 6.0, wave * 5.0, 11.0 - wave_dip * 6.0) / SCALE;
        next.r_hand.offset = Vec3::new(6.0 + wave_dip * 6.0, -wave * 5.0, 11.0 - wave_dip * 6.0) / SCALE;

        next.l_foot.offset = Vec3::new(-3.5, 1.0 - wave * 8.0, 3.5 - wave_dip * 4.0) / SCALE;
        next.l_foot.ori = Quaternion::rotation_x(-wave + 1.0);
        next.r_foot.offset = Vec3::new(3.5, 1.0 + wave * 8.0, 3.5 - wave_dip * 4.0) / SCALE;
        next.r_foot.ori = Quaternion::rotation_x(wave + 1.0);

        next.back.offset = Vec3::new(-9.0, 5.0, 18.0);
        next.back.ori = Quaternion::rotation_y(2.5);
        next.back.scale = Vec3::one();

        next

    }
}
