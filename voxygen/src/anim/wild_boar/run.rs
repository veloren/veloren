use super::{
    super::{Animation, SkeletonAttr},
    WildBoarSkeleton,
};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Skeleton = WildBoarSkeleton;
    type Dependency = (f32, f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (_velocity, global_time): Self::Dependency,
        anim_time: f64,
        _skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 14.0).sin();
        let wave_ultra_slow = (anim_time as f32 * 1.0 + PI).sin();
        let wave_ultra_slow_cos = (anim_time as f32 * 1.0 + PI).cos();
        let wave_slow = (anim_time as f32 * 3.5 + PI).sin();
        let wave_slow_cos = (anim_time as f32 * 3.5 + PI).cos();
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

        next.wild_boar_head.offset = Vec3::new(-0.15, 0.45, 0.65 + wave_med * 0.25);
        next.wild_boar_head.ori = Quaternion::rotation_x(wave_quick * 0.12);
        next.wild_boar_head.scale = Vec3::one() / 11.0;

        next.wild_boar_torso.offset = Vec3::new(-0.15, 0.2, 1.0 + wave_med * 0.15);
        next.wild_boar_torso.ori = Quaternion::rotation_x(wave_quick * 0.08);
        next.wild_boar_torso.scale = Vec3::one() / 11.0;

        next.wild_boar_tail.offset = Vec3::new(0.05, -0.15, 1.0 + wave_med * 0.15);
        next.wild_boar_tail.ori = Quaternion::rotation_x(wave_quick * 0.20);
        next.wild_boar_tail.scale = Vec3::one() / 11.0;

        next.wild_boar_leg_lf.offset = Vec3::new(-0.3, 0.1, 0.8 + wave_med * 0.25);
        next.wild_boar_leg_lf.ori = Quaternion::rotation_x(wave_quick * 0.15);
        next.wild_boar_leg_lf.scale = Vec3::one() / 11.0;

        next.wild_boar_leg_rf.offset = Vec3::new(0.55, 0.1, 0.8 + wave_med * 0.25);
        next.wild_boar_leg_rf.ori = Quaternion::rotation_x(wave_quick * 0.15);
        next.wild_boar_leg_rf.scale = Vec3::one() / 11.0;

        next.wild_boar_leg_lb.offset = Vec3::new(-0.2, -0.6, 0.75 + wave_med * 0.25);
        next.wild_boar_leg_lb.ori = Quaternion::rotation_x(wave_quick * 0.15);
        next.wild_boar_leg_lb.scale = Vec3::one() / 11.0;

        next.wild_boar_leg_rb.offset = Vec3::new(0.45, -0.6, 0.75 + wave_med * 0.25);
        next.wild_boar_leg_rb.ori = Quaternion::rotation_x(wave_quick * 0.15);
        next.wild_boar_leg_rb.scale = Vec3::one() / 11.0;

        next.wild_boar_foot_lf.offset =
            Vec3::new(-3.5, 5.0 + wave_quick * 3.0 + wave_quick_cos * 2.5, 3.0) / 11.0;
        next.wild_boar_foot_lf.ori = Quaternion::rotation_x(0.0 + wave_quick * 0.6);
        next.wild_boar_foot_lf.scale = Vec3::one() / 11.0;

        next.wild_boar_foot_rf.offset =
            Vec3::new(6.5, 5.0 - wave_quick_cos * 2.5, 3.0 + wave_quick * 3.0) / 11.0;
        next.wild_boar_foot_rf.ori = Quaternion::rotation_x(0.0 + wave_quick * 0.6);
        next.wild_boar_foot_rf.scale = Vec3::one() / 11.0;

        next.wild_boar_foot_lb.offset =
            Vec3::new(-2.5, -4.0 - wave_quick_cos * 2.5, 3.0 + wave_quick * 3.0) / 11.0;
        next.wild_boar_foot_lb.ori = Quaternion::rotation_x(0.0 + wave_quick * 0.6);
        next.wild_boar_foot_lb.scale = Vec3::one() / 11.0;

        next.wild_boar_foot_rb.offset =
            Vec3::new(5.5, -4.0 + wave_quick * 3.0 + wave_quick_cos * 2.5, 3.0) / 11.0;
        next.wild_boar_foot_rb.ori = Quaternion::rotation_x(0.0 + wave_quick * 0.6);
        next.wild_boar_foot_rb.scale = Vec3::one() / 11.0;

        next
    }
}
