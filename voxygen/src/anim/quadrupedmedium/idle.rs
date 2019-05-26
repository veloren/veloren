// Standard
use std::{f32::consts::PI, ops::Mul};

// Library
use vek::*;

// Local
use super::{super::Animation, QuadrupedMediumSkeleton, SCALE};

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Skeleton = QuadrupedMediumSkeleton;
    type Dependency = (f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency,
        anim_time: f64,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 14.0).sin();
        let wave_ultra_slow = (anim_time as f32 * 1.0 + PI).sin();
        let wave_ultra_slow_cos = (anim_time as f32 * 1.0 + PI).cos();
        let wave_cos = (anim_time as f32 * 14.0).cos();
        let wave_slow = (anim_time as f32 * 3.5 + PI).sin();
        let wave_slow_cos = (anim_time as f32 * 3.5 + PI).cos();
        let wave_dip = (wave_slow.abs() - 0.5).abs();

        let wolf_look = Vec2::new(
            ((global_time + anim_time) as f32 / 8.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.5,
            ((global_time + anim_time) as f32 / 8.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.25,
        );
        let wolf_tail = Vec2::new(
            ((global_time + anim_time) as f32 / 2.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.25,
            ((global_time + anim_time) as f32 / 2.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.125,
        );

        next.wolf_upperhead.offset = Vec3::new(0.0, 7.5, 15.0 + wave_ultra_slow * 0.4) / 11.0;
        next.wolf_upperhead.ori =
            Quaternion::rotation_z(wolf_look.x) * Quaternion::rotation_x(wolf_look.y);
        next.wolf_upperhead.scale = Vec3::one() / 10.88;

        next.wolf_jaw.offset =
            Vec3::new(0.0, 4.5 - wave_ultra_slow_cos * 0.12, 2.0 + wave_slow * 0.2);
        next.wolf_jaw.ori = Quaternion::rotation_x(wave_slow * 0.05);
        next.wolf_jaw.scale = Vec3::one() * 1.01;

        next.wolf_lowerhead.offset = Vec3::new(0.0, 3.1, -4.5 + wave_ultra_slow * 0.20);
        next.wolf_lowerhead.ori = Quaternion::rotation_z(0.0);
        next.wolf_lowerhead.scale = Vec3::one() * 0.98;

        next.wolf_tail.offset = Vec3::new(0.0, -13.0, 8.0 + wave_ultra_slow * 1.2) / 11.0;
        next.wolf_tail.ori = Quaternion::rotation_z(0.0 + wave_slow * 0.2 + wolf_tail.x)
            * Quaternion::rotation_x(wolf_tail.y);
        next.wolf_tail.scale = Vec3::one() / 11.0;

        next.wolf_torsoback.offset = Vec3::new(0.0, -11.7, 11.0 + wave_ultra_slow * 1.2) / 11.0;
        next.wolf_torsoback.ori = Quaternion::rotation_y(wave_slow_cos * 0.015);
        next.wolf_torsoback.scale = Vec3::one() / 11.0;

        next.wolf_torsomid.offset = Vec3::new(0.0, 0.0, 12.0 + wave_ultra_slow * 0.7) / 11.0;
        next.wolf_torsomid.ori = Quaternion::rotation_y(wave_slow * 0.015);
        next.wolf_torsomid.scale = Vec3::one() / 10.5;

        next.wolf_ears.offset = Vec3::new(0.0, 0.75, 5.25);
        next.wolf_ears.ori = Quaternion::rotation_x(0.0 + wave_slow * 0.1);
        next.wolf_ears.scale = Vec3::one() * 1.05;

        next.wolf_LFFoot.offset = Vec3::new(-5.0, 5.0, 2.5) / 11.0;
        next.wolf_LFFoot.ori = Quaternion::rotation_x(0.0);
        next.wolf_LFFoot.scale = Vec3::one() / 11.0;

        next.wolf_RFFoot.offset = Vec3::new(5.0, 5.0, 2.5) / 11.0;
        next.wolf_RFFoot.ori = Quaternion::rotation_x(0.0);
        next.wolf_RFFoot.scale = Vec3::one() / 11.0;

        next.wolf_LBFoot.offset = Vec3::new(-5.0, -10.0, 2.5) / 11.0;
        next.wolf_LBFoot.ori = Quaternion::rotation_x(0.0);
        next.wolf_LBFoot.scale = Vec3::one() / 11.0;

        next.wolf_RBFoot.offset = Vec3::new(5.0, -10.0, 2.5) / 11.0;
        next.wolf_RBFoot.ori = Quaternion::rotation_x(0.0);
        next.wolf_RBFoot.scale = Vec3::one() / 11.0;

        next
    }
}
