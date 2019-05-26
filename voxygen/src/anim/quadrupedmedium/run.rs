// Standard
use std::{f32::consts::PI, ops::Mul};

// Library
use vek::*;

// Local
use super::{super::Animation, QuadrupedMediumSkeleton, SCALE};

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Skeleton = QuadrupedMediumSkeleton;
    type Dependency = (f32, f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (velocity, global_time): Self::Dependency,
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
        let wave_quick = (anim_time as f32 * 18.0).sin();
        let wave_med = (anim_time as f32 * 12.0).sin();
        let wave_med_cos = (anim_time as f32 * 12.0).cos();

        let wave_quick_cos = (anim_time as f32 * 18.0).cos();

        let wolf_look = Vec2::new(
            ((global_time + anim_time) as f32 / 4.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.25,
            ((global_time + anim_time) as f32 / 4.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.125,
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

        next.wolf_upperhead.offset =
            Vec3::new(0.0, 9.5 + wave_quick_cos * 2.0, 15.0 + wave_med * 3.0) / 11.0;
        next.wolf_upperhead.ori =
            Quaternion::rotation_x(-0.12 + wave_quick_cos * 0.12 + wolf_look.y)
                * Quaternion::rotation_z(wolf_look.x);
        next.wolf_upperhead.scale = Vec3::one() / 10.88;

        next.wolf_jaw.offset = Vec3::new(0.0, 4.5, 2.0 + wave_slow_cos * 1.0);
        next.wolf_jaw.ori = Quaternion::rotation_x(wave_slow * 0.05);
        next.wolf_jaw.scale = Vec3::one() * 1.01;

        next.wolf_lowerhead.offset = Vec3::new(0.0, 3.1, -4.5 + wave_med * 1.0);
        next.wolf_lowerhead.ori = Quaternion::rotation_z(0.0);
        next.wolf_lowerhead.scale = Vec3::one() * 0.98;

        next.wolf_tail.offset = Vec3::new(0.0, -12.0, 10.0) / 11.0;
        next.wolf_tail.ori = Quaternion::rotation_x(wave_quick * 0.18);
        next.wolf_tail.scale = Vec3::one() / 11.0;

        next.wolf_torsoback.offset =
            Vec3::new(0.0, -9.5 + wave_quick_cos * 2.2, 13.0 + wave_med * 2.8) / 11.0;
        next.wolf_torsoback.ori = Quaternion::rotation_x(-0.15 + wave_med_cos * 0.14);
        next.wolf_torsoback.scale = Vec3::one() / 11.0;

        next.wolf_torsomid.offset =
            Vec3::new(0.0, 0.0 + wave_quick_cos * 2.2, 14.0 + wave_med * 3.2) / 11.0;
        next.wolf_torsomid.ori = Quaternion::rotation_x(-0.15 + wave_med_cos * 0.12);
        next.wolf_torsomid.scale = Vec3::one() / 10.9;

        next.wolf_ears.offset = Vec3::new(0.0, 0.75 + wave * 0.4, 6.25);
        next.wolf_ears.ori = Quaternion::rotation_x(wave * 0.2);
        next.wolf_ears.scale = Vec3::one() * 1.05;

        next.wolf_LFFoot.offset =
            Vec3::new(-5.0, 5.0 + wave_quick * 3.0, 7.0 + wave_quick_cos * 4.0) / 11.0;
        next.wolf_LFFoot.ori = Quaternion::rotation_x(0.0 + wave_quick * 0.8);
        next.wolf_LFFoot.scale = Vec3::one() / 11.0;

        next.wolf_RFFoot.offset =
            Vec3::new(5.0, 5.0 - wave_quick_cos * 3.0, 7.0 + wave_quick * 4.0) / 11.0;
        next.wolf_RFFoot.ori = Quaternion::rotation_x(0.0 - wave_quick_cos * 0.8);
        next.wolf_RFFoot.scale = Vec3::one() / 11.0;

        next.wolf_LBFoot.offset =
            Vec3::new(-5.0, -10.0 - wave_quick_cos * 3.0, 7.0 + wave_quick * 4.0) / 11.0;
        next.wolf_LBFoot.ori = Quaternion::rotation_x(0.0 - wave_quick_cos * 0.8);
        next.wolf_LBFoot.scale = Vec3::one() / 11.0;

        next.wolf_RBFoot.offset =
            Vec3::new(5.0, -10.0 + wave_quick * 3.0, 7.0 + wave_quick_cos * 4.0) / 11.0;
        next.wolf_RBFoot.ori = Quaternion::rotation_x(0.0 + wave_quick * 0.8);
        next.wolf_RBFoot.scale = Vec3::one() / 11.0;

        next
    }
}
