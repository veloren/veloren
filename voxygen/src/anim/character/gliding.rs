// Standard
use std::{f32::consts::PI, ops::Mul};

// Library
use vek::*;

// Local
use super::{super::Animation, CharacterSkeleton, SCALE};
//

pub struct GlidingAnimation;

impl Animation for GlidingAnimation {
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
        let wavecos_slow = (anim_time as f32 * 7.0).cos();
        let arcwave = (1.0f32.ln_1p() - 1.5).abs();
        let wavetest = (wave.cbrt());
        let fuzzwave = (anim_time as f32 * 12.0).sin();
        let wavecos = (anim_time as f32 * 14.0).cos();
        let wave_stop = (anim_time as f32 * 1.5).min(PI / 2.0).sin();
        let wave_stopalt = (anim_time as f32 * 5.0).min(PI / 2.0).sin();
        let waveveryslow = (anim_time as f32 * 3.0).sin();
        let waveveryslowalt = (anim_time as f32 * 2.5).sin();
        let waveveryslowcos = (anim_time as f32 * 3.0).cos();


        let wave_slowtest = (anim_time as f32).min(PI / 2.0).sin();

        let head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 4.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.5,
            ((global_time + anim_time) as f32 / 4.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.25,
        );
        next.head.offset = Vec3::new(5.5, 2.0, 12.0);
        next.head.ori = Quaternion::rotation_x(0.35 - waveveryslow * 0.10 + head_look.y) * Quaternion::rotation_z(head_look.x + waveveryslowcos * 0.15);
        next.head.scale = Vec3::one();

        next.chest.offset = Vec3::new(5.5, 0.0, 8.0);
        next.chest.ori = Quaternion::rotation_z(waveveryslowcos * 0.15);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(5.5, 0.0, 6.0);
        next.belt.ori = Quaternion::rotation_z(waveveryslowcos * 0.20);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(5.5, 0.0, 3.0);
        next.shorts.ori = Quaternion::rotation_z(waveveryslowcos * 0.25);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(-8.0, -10.0 + waveveryslow * 2.5, 18.5 + waveveryslow * 1.0);
        next.l_hand.ori = Quaternion::rotation_x(0.9 - waveveryslow * 0.10);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(11.0, -10.0 + waveveryslow * 2.5, 18.5 + waveveryslow * 1.0);
        next.r_hand.ori = Quaternion::rotation_x(0.9 - waveveryslow * 0.10);
        next.r_hand.scale = Vec3::one();

        next.l_foot.offset = Vec3::new(-3.4, 1.0, 8.0);
        next.l_foot.ori = Quaternion::rotation_x(wave_stop * -0.7 - wavecos_slow * -0.21 + waveveryslow * 0.19);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(3.4, 1.0, 8.0);
        next.r_foot.ori = Quaternion::rotation_x(wave_stop * -0.8  + waveslow * -0.25 + waveveryslowalt * 0.13);
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
        next.torso.ori = Quaternion::rotation_x(-0.8 + waveveryslow * 0.10);
        next.torso.scale = Vec3::one() / 11.0;

        next.draw.offset = Vec3::new(13.5, 3.0, -1.0);
        next.draw.ori = Quaternion::rotation_y(waveveryslowcos * 0.05);
        next.draw.scale = Vec3::one();

        next
    }
}
