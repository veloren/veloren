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
        skeleton: &mut Self::Skeleton,
        time: f64,
    ) {
        let wave = (time as f32 * 12.0).sin();
        let wave_fast = (time as f32 * 6.0).sin();
        let wave_dip = (wave_fast.abs() - 0.5).abs();

        skeleton.head.offset = Vec3::unit_z() * 13.0;
        skeleton.head.ori = Quaternion::rotation_z(wave * 0.3);

        skeleton.chest.offset = Vec3::unit_z() * 9.0;
        skeleton.chest.ori = Quaternion::rotation_z(wave * 0.3);

        skeleton.belt.offset = Vec3::unit_z() * 7.0;
        skeleton.belt.ori = Quaternion::rotation_z(wave * 0.2);

        skeleton.shorts.offset = Vec3::unit_z() * 4.0;
        skeleton.shorts.ori = Quaternion::rotation_z(wave * 0.1);

        skeleton.l_hand.offset = Vec3::new(-6.0 - wave_dip * 6.0, wave * 5.0, 11.0 - wave_dip * 6.0);
        skeleton.r_hand.offset = Vec3::new(6.0 + wave_dip * 6.0, -wave * 5.0, 11.0 - wave_dip * 6.0);

        skeleton.l_foot.offset = Vec3::new(-3.5, 1.0 - wave * 8.0, 3.5 - wave_dip * 4.0);
        skeleton.l_foot.ori = Quaternion::rotation_x(-wave + 1.0);
        skeleton.r_foot.offset = Vec3::new(3.5, 1.0 + wave * 8.0, 3.5 - wave_dip * 4.0);
        skeleton.r_foot.ori = Quaternion::rotation_x(wave + 1.0);

        skeleton.back.offset = Vec3::new(-9.0, 5.0, 18.0);
        skeleton.back.ori = Quaternion::rotation_y(2.5);
    }
}
