// Standard
use std::f32::consts::PI;

// Library
use vek::*;

// Local
use super::{super::Animation, CharacterSkeleton, SCALE};
//

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Skeleton = CharacterSkeleton;
    type Dependency = f64;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        global_time: f64,
        anim_time: f64,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let wave = (anim_time as f32 * 14.0).sin();
        let waveslow = (anim_time as f32 * 7.0).sin();
        let arcwave = (1.0f32.ln_1p() - 1.5).abs();
        let wavetest = (wave.cbrt());
        let fuzzwave = (anim_time as f32 * 12.0).sin();
        let wavecos = (anim_time as f32 * 14.0).cos();
        let wave_stop = (anim_time as f32 * 4.5).min(PI / 2.0).sin();
        let wave_stopalt = (anim_time as f32 * 5.0).min(PI / 2.0).sin();

        let wave_slowtest = (anim_time as f32).min(PI / 2.0).sin();
        let wavecos_slow = (anim_time as f32 * 8.0 + PI).cos();

        next.head.offset = Vec3::new(5.5, 2.0, 12.0);
        next.head.ori = Quaternion::rotation_x(0.25);
        next.head.scale = Vec3::one();

        next.chest.offset = Vec3::new(5.5, 0.0, 8.0);
        next.chest.ori = Quaternion::rotation_z(0.0);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(5.5, 0.0, 6.0);
        next.belt.ori = Quaternion::rotation_z(0.0);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(5.5, 0.0, 3.0);
        next.shorts.ori = Quaternion::rotation_z(0.0);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(-6.0, 0.0, 12.0);
        next.l_hand.ori = Quaternion::rotation_x(wave_stopalt * 1.2 - waveslow * 0.15);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(9.0, 0.0, 12.0);
        next.r_hand.ori = Quaternion::rotation_x(wave_stopalt * -1.2 + waveslow * 0.15);
        next.r_hand.scale = Vec3::one();

        next.l_foot.offset = Vec3::new(-3.4, 1.0, 6.0);
        next.l_foot.ori = Quaternion::rotation_x(wave_stop * -1.2 - waveslow * 0.2);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(3.4, -1.0, 6.0);
        next.r_foot.ori = Quaternion::rotation_x(wave_stop * 1.2 + waveslow * 0.2);
        next.r_foot.scale = Vec3::one();

        next.weapon.offset = Vec3::new(-5.0, -6.0, 19.0);
        next.weapon.ori = Quaternion::rotation_y(2.5);
        next.weapon.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(-10.0, -3.0, 2.5);
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = Vec3::one();

        next.r_shoulder.offset = Vec3::new(0.0, -3.0, 2.5);
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = Vec3::one();

        next.torso.offset = Vec3::new(-0.5, -0.2, 0.0);
        next.torso.ori = Quaternion::rotation_x(-0.2);
        next.torso.scale = Vec3::one() / 11.0;

        next.draw.offset = Vec3::new(13.5, 0.0, 0.0);
        next.draw.ori = Quaternion::rotation_y(0.0);
        next.draw.scale = Vec3::one() * 0.0;
        next
    }
}
