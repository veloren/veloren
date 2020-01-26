use super::{super::Animation, QuadrupedSmallSkeleton, SkeletonAttr};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Skeleton = QuadrupedSmallSkeleton;
    type Dependency = f64;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 14.0).sin();
        let wave_slow = (anim_time as f32 * 3.5 + PI).sin();
        let wave_slow_cos = (anim_time as f32 * 3.5 + PI).cos();

        let pig_head_look = Vec2::new(
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

        next.head.offset =
            Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1 + wave * 0.2) / 11.0;
        next.head.ori = Quaternion::rotation_z(pig_head_look.x)
            * Quaternion::rotation_x(pig_head_look.y + wave_slow_cos * 0.03);
        next.head.scale = Vec3::one() / 10.5;

        next.chest.offset = Vec3::new(
            wave_slow * 0.05,
            skeleton_attr.chest.0,
            skeleton_attr.chest.1 + wave_slow_cos * 0.2,
        ) / 11.0;
        next.chest.ori = Quaternion::rotation_y(wave_slow * 0.05);
        next.chest.scale = Vec3::one() / 11.0;

        next.leg_lf.offset = Vec3::new(
            -skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2,
        ) / 11.0;
        next.leg_lf.ori = Quaternion::rotation_x(wave_slow * 0.08);
        next.leg_lf.scale = Vec3::one() / 11.0;

        next.leg_rf.offset = Vec3::new(
            skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2,
        ) / 11.0;
        next.leg_rf.ori = Quaternion::rotation_x(wave_slow_cos * 0.08);
        next.leg_rf.scale = Vec3::one() / 11.0;

        next.leg_lb.offset = Vec3::new(
            -skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2,
        ) / 11.0;
        next.leg_lb.ori = Quaternion::rotation_x(wave_slow_cos * 0.08);
        next.leg_lb.scale = Vec3::one() / 11.0;

        next.leg_rb.offset = Vec3::new(
            skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2,
        ) / 11.0;
        next.leg_rb.ori = Quaternion::rotation_x(wave_slow * 0.08);
        next.leg_rb.scale = Vec3::one() / 11.0;

        next
    }
}
