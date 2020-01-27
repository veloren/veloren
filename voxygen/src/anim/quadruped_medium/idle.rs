use super::{super::Animation, QuadrupedMediumSkeleton, SkeletonAttr};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Skeleton = QuadrupedMediumSkeleton;
    type Dependency = f64;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave_ultra_slow = (anim_time as f32 * 1.0 + PI).sin();
        let wave_ultra_slow_cos = (anim_time as f32 * 1.0 + PI).cos();
        let wave_slow = (anim_time as f32 * 3.5 + PI).sin();
        let wave_slow_cos = (anim_time as f32 * 3.5 + PI).cos();

        let look = Vec2::new(
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
        let tailmove = Vec2::new(
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

        next.head_upper.offset = Vec3::new(
            0.0,
            skeleton_attr.head_upper.0,
            skeleton_attr.head_upper.1 + wave_ultra_slow * 0.4,
        ) / 11.0;
        next.head_upper.ori =
            Quaternion::rotation_z(0.8 * look.x) * Quaternion::rotation_x(0.8 * look.y);
        next.head_upper.scale = Vec3::one() / 10.98;

        next.head_lower.offset = Vec3::new(
            0.0,
            skeleton_attr.head_lower.0,
            skeleton_attr.head_lower.1 + wave_ultra_slow * 0.20,
        );
        next.head_lower.ori =
            Quaternion::rotation_z(-0.4 * look.x) * Quaternion::rotation_x(-0.4 * look.y);
        next.head_lower.scale = Vec3::one() * 1.02;

        next.jaw.offset = Vec3::new(
            0.0,
            skeleton_attr.jaw.0 - wave_ultra_slow_cos * 0.12,
            skeleton_attr.jaw.1 + wave_slow * 0.2,
        );
        next.jaw.ori = Quaternion::rotation_x(wave_slow * 0.05);
        next.jaw.scale = Vec3::one() * 0.98;

        next.tail.offset = Vec3::new(
            0.0,
            skeleton_attr.tail.0,
            skeleton_attr.tail.1 + wave_ultra_slow * 0.6,
        );
        next.tail.ori = Quaternion::rotation_z(0.0 + wave_slow * 0.2 + tailmove.x)
            * Quaternion::rotation_x(tailmove.y);
        next.tail.scale = Vec3::one();

        next.torso_back.offset = Vec3::new(
            0.0,
            skeleton_attr.torso_back.0,
            skeleton_attr.torso_back.1 + wave_ultra_slow * 0.6,
        ) / 11.0;
        next.torso_back.ori = Quaternion::rotation_y(wave_slow_cos * 0.015)
            * Quaternion::rotation_z(0.1 * look.x)
            * Quaternion::rotation_x(0.2 * look.y);
        next.torso_back.scale = Vec3::one() / 11.0;

        next.torso_mid.offset = Vec3::new(
            0.0,
            skeleton_attr.torso_mid.0,
            skeleton_attr.torso_mid.1 + wave_ultra_slow * 0.5,
        ) / 11.0;
        next.torso_mid.ori = Quaternion::rotation_y(wave_slow * 0.015);
        next.torso_mid.scale = Vec3::one() / 10.98;

        next.ears.offset = Vec3::new(0.0, skeleton_attr.ears.0, skeleton_attr.ears.1);
        next.ears.ori = Quaternion::rotation_x(0.0 + wave_ultra_slow * 0.03);
        next.ears.scale = Vec3::one() / 1.02;

        next.foot_lf.offset = Vec3::new(
            -skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2,
        ) / 11.0;
        next.foot_lf.ori = Quaternion::rotation_x(0.0);
        next.foot_lf.scale = Vec3::one() / 11.0;

        next.foot_rf.offset = Vec3::new(
            skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2,
        ) / 11.0;
        next.foot_rf.ori = Quaternion::rotation_x(0.0);
        next.foot_rf.scale = Vec3::one() / 11.0;

        next.foot_lb.offset = Vec3::new(
            -skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2,
        ) / 11.0;
        next.foot_lb.ori = Quaternion::rotation_x(0.0);
        next.foot_lb.scale = Vec3::one() / 11.0;

        next.foot_rb.offset = Vec3::new(
            skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2,
        ) / 11.0;
        next.foot_rb.ori = Quaternion::rotation_x(0.0);
        next.foot_rb.scale = Vec3::one() / 11.0;

        next
    }
}
