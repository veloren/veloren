// Standard
use std::{
    f32::consts::PI,
    ops::Mul,
};

// Library
use vek::*;

// Local
use super::{super::Animation, CharacterSkeleton, SCALE};

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Skeleton = CharacterSkeleton;
    type Dependency = f64;

    fn update_skeleton(skeleton: &Self::Skeleton, global_time: f64, anim_time: f64) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 12.0).sin();
        let wavecos = (anim_time as f32 * 12.0).cos();
        let wave_slow = (anim_time as f32 * 6.0 + PI).sin();
        let wavecos_slow = (anim_time as f32 * 6.0 + PI).cos();
        let waveultra_slow = (anim_time as f32 * 1.0 + PI).sin();
        let waveultracos_slow = (anim_time as f32 * 1.0 + PI).cos();
        let wave_dip = (wave_slow.abs() - 0.5).abs();

        let head_look = Vec2::new(
            (global_time as f32 / 5.0).floor().mul(7331.0).sin() * 0.5,
            (global_time as f32 / 5.0).floor().mul(1337.0).sin() * 0.25,
        );
        next.head.offset = Vec3::new(5.5, 2.0, 11.5 + waveultra_slow * 0.4);
        next.head.ori = Quaternion::rotation_z(head_look.x) * Quaternion::rotation_x(head_look.y);
        next.head.scale = Vec3::one();

        next.chest.offset = Vec3::new(5.5, 0.0, 7.5 + waveultra_slow * 0.4);
        next.chest.ori = Quaternion::rotation_x(0.0);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(5.5, 0.0, 5.5 + waveultra_slow * 0.4);
        next.belt.ori = Quaternion::rotation_x(0.0);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(5.5, 0.0, 2.5 + waveultra_slow * 0.4);
        next.shorts.ori = Quaternion::rotation_x(0.0);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(
            -7.5, -2.0 + waveultracos_slow * 0.3,

            12.0 + waveultra_slow * 1.1,
        );
        next.l_hand.ori = Quaternion::rotation_x(0.0 + waveultra_slow * 0.06);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(
            7.5, -2.0 + waveultracos_slow * 0.3,

            12.0 + waveultra_slow * 1.1,
        );
        next.r_hand.ori = Quaternion::rotation_x(0.0 + waveultra_slow * 0.06);
        next.r_hand.scale = Vec3::one();

        next.l_foot.offset = Vec3::new(-3.4, 0.0, 8.0);
        next.l_foot.ori = Quaternion::identity();
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(3.4, 0.0, 8.0);
        next.r_foot.ori = Quaternion::identity();
        next.r_foot.scale = Vec3::one();

        next.weapon.offset = Vec3::new(-5.0, -6.0, 18.5);
        next.weapon.ori = Quaternion::rotation_y(2.5);
        next.weapon.scale = Vec3::one();

        next.torso.offset = Vec3::new(-0.5, -0.2, 0.1);
        next.torso.ori = Quaternion::rotation_x(0.0);
        next.torso.scale = Vec3::one() / 11.0;

        next.l_shoulder.offset = Vec3::new(2.9, 6.0, 18.0);
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = Vec3::one();

        next.r_shoulder.offset = Vec3::new(2.9, -6.0, 18.0);
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = Vec3::one();

        next
    }
}
