// Standard
use std::{f32::consts::PI, ops::Mul};

// Library
use vek::*;

// Local
use super::{super::Animation, CharacterSkeleton, SCALE};

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
        let wave_slow = (anim_time as f32 * 7.0).sin();
        let wave_slow_cos = (anim_time as f32 * 7.0).cos();
        let arc_wave = (1.0f32.ln_1p() - 1.5).abs();
        let wave_test = (wave.cbrt());
        let fuzz_wave = (anim_time as f32 * 12.0).sin();
        let wave_cos = (anim_time as f32 * 14.0).cos();
        let wave_stop = (anim_time as f32 * 1.5).min(PI / 2.0).sin();
        let wave_stop_alt = (anim_time as f32 * 5.0).min(PI / 2.0).sin();
        let wave_very_slow = (anim_time as f32 * 3.0).sin();
        let wave_very_slow_alt = (anim_time as f32 * 2.5).sin();
        let wave_very_slow_cos = (anim_time as f32 * 3.0).cos();

        let wave_slow_test = (anim_time as f32).min(PI / 2.0).sin();

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
        next.head.offset = Vec3::new(0.0, 2.0, 12.0);
        next.head.ori = Quaternion::rotation_x(0.35 - wave_very_slow * 0.10 + head_look.y)
            * Quaternion::rotation_z(head_look.x + wave_very_slow_cos * 0.15);
        next.head.scale = Vec3::one();

        next.chest.offset = Vec3::new(0.0, 0.0, 8.0);
        next.chest.ori = Quaternion::rotation_z(wave_very_slow_cos * 0.15);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0, 0.0, 6.0);
        next.belt.ori = Quaternion::rotation_z(wave_very_slow_cos * 0.20);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(0.0, 0.0, 3.0);
        next.shorts.ori = Quaternion::rotation_z(wave_very_slow_cos * 0.25);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(
            -10.0,
            6.0 - wave_very_slow * 1.50,
            15.0 + wave_very_slow * 0.50,
        ) / 11.0;
        next.l_hand.ori = Quaternion::rotation_x(0.2 + wave_very_slow_cos * 0.05);
        next.l_hand.scale = Vec3::one() / 11.0;

        next.r_hand.offset = Vec3::new(
            10.0,
            6.0 - wave_very_slow * 1.50,
            14.5 + wave_very_slow * 0.50,
        ) / 11.0;
        next.r_hand.ori = Quaternion::rotation_x(0.1 + wave_very_slow * 0.05);
        next.r_hand.scale = Vec3::one() / 11.0;

        next.l_foot.offset = Vec3::new(-3.4, 1.0, 8.0);
        next.l_foot.ori = Quaternion::rotation_x(
            wave_stop * -0.7 - wave_slow_cos * -0.21 + wave_very_slow * 0.19,
        );
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(3.4, 1.0, 8.0);
        next.r_foot.ori = Quaternion::rotation_x(
            wave_stop * -0.8 + wave_slow * -0.25 + wave_very_slow_alt * 0.13,
        );
        next.r_foot.scale = Vec3::one();

        next.weapon.offset = Vec3::new(-7.0, -5.0, 15.0);
        next.weapon.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
        next.weapon.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(-10.0, -3.2, 2.5);
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = Vec3::one() * 1.04;

        next.r_shoulder.offset = Vec3::new(0.0, -3.2, 2.5);
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = Vec3::one() * 1.04;

        next.draw.offset = Vec3::new(0.0, -9.0 + wave_very_slow * 0.10, 18.0);
        next.draw.ori = Quaternion::rotation_x(0.95 - wave_very_slow * 0.15)
            * Quaternion::rotation_y(wave_very_slow_cos * 0.04);
        next.draw.scale = Vec3::one();

        next.left_equip.offset = Vec3::new(0.0, 0.0, 5.0) / 11.0;
        next.left_equip.ori = Quaternion::rotation_x(0.0);;
        next.left_equip.scale = Vec3::one() * 0.0;

        next.right_equip.offset = Vec3::new(0.0, 0.0, 5.0) / 11.0;
        next.right_equip.ori = Quaternion::rotation_x(0.0);;
        next.right_equip.scale = Vec3::one() * 0.0;

        next.torso.offset = Vec3::new(0.0, -0.2, 0.0);
        next.torso.ori = Quaternion::rotation_x(-0.8 + wave_very_slow * 0.10);
        next.torso.scale = Vec3::one() / 11.0;

        next
    }
}
