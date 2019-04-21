// Standard
use std::f32::consts::PI;

// Library
use vek::*;

// Local
use super::{
    CharacterSkeleton,
    super::Animation,
    SCALE,
};

pub struct IdleAnimation;

//TODO: Make it actually good, possibly add the head rotating slightly, add breathing, etc.
impl Animation for IdleAnimation {
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
        let waveultra_slow = (time as f32 * 1.0 + PI).sin();
        let waveultracos_slow = (time as f32 * 1.0 + PI).cos();
        let wave_dip = (wave_slow.abs() - 0.5).abs();

        next.head.offset = Vec3::new(-1.0, 0.0, 12.0 + waveultra_slow * 0.4) / SCALE;
        next.head.ori = Quaternion::rotation_y(waveultra_slow * 0.05);
        next.head.scale = Vec3::one() / SCALE;

        next.chest.offset = Vec3::new(2.5, 0.0, 8.0 + waveultra_slow * 0.4) / SCALE;
        next.chest.ori = Quaternion::rotation_y(0.0);
        next.chest.scale = Vec3::one() / SCALE;

        next.belt.offset = Vec3::new(2.5, 0.0, 6.0 + waveultra_slow * 0.4) / SCALE;
        next.belt.ori = Quaternion::rotation_y(0.0);
        next.belt.scale = Vec3::one() / SCALE;

        next.shorts.offset = Vec3::new(2.5, 0.0, 3.0 + waveultra_slow * 0.4) / SCALE;
        next.shorts.ori = Quaternion::rotation_y(0.0);
        next.shorts.scale = Vec3::one() / SCALE;

        next.l_hand.offset = Vec3::new(2.0 + waveultracos_slow * 0.3, 7.5, 13.5 + waveultra_slow * 1.1) / SCALE;
        next.l_hand.ori = Quaternion::rotation_y(0.0 + waveultra_slow * 0.06);
        next.r_hand.offset = Vec3::new(2.0 + waveultracos_slow * 0.3 , - 7.5, 13.5 + waveultra_slow * 1.1) / SCALE;
        next.r_hand.ori = Quaternion::rotation_y(0.0 + waveultra_slow * 0.06);

        next.l_foot.offset = Vec3::new(5.0, 3.4, 8.0) / SCALE;
        next.l_foot.ori = Quaternion::rotation_y(0.04 + waveultra_slow * 0.04);
        next.r_foot.offset = Vec3::new(5.0, -3.4, 8.0) / SCALE;
        next.r_foot.ori = Quaternion::rotation_y(0.04 + waveultra_slow * 0.04);

        next.back.offset = Vec3::new(-4.5, 12.0, 11.0);
        next.back.ori = Quaternion::rotation_x(2.5);
        next.back.scale = Vec3::one();


        next.torso.offset = Vec3::new(0.0, 0.0, 0.0);
        next.torso.ori = Quaternion::rotation_y(0.0);
        next.torso.scale = Vec3::one();

        next
    }
}
