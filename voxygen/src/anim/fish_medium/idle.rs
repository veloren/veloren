use super::{
    super::{Animation, SkeletonAttr},
    FishMediumSkeleton,
};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Skeleton = FishMediumSkeleton;
    type Dependency = (f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        _skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave_ultra_slow = (anim_time as f32 * 1.0 + PI).sin();
        let wave_ultra_slow_cos = (anim_time as f32 * 1.0 + PI).cos();
        let wave_slow = (anim_time as f32 * 3.5 + PI).sin();
        let wave_slow_cos = (anim_time as f32 * 3.5 + PI).cos();

        let duck_m_look = Vec2::new(
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

        next.marlin_head.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.marlin_head.ori =
            Quaternion::rotation_z(duck_m_look.x) * Quaternion::rotation_x(duck_m_look.y);
        next.marlin_head.scale = Vec3::one() / 10.88;

        next.marlin_torso.offset = Vec3::new(0.0, 4.5 - wave_ultra_slow_cos * 0.12, 2.0);
        next.marlin_torso.ori = Quaternion::rotation_x(0.0);
        next.marlin_torso.scale = Vec3::one() * 1.01;

        next.marlin_rear.offset = Vec3::new(0.0, 3.1, -4.5);
        next.marlin_rear.ori = Quaternion::rotation_z(0.0);
        next.marlin_rear.scale = Vec3::one() * 0.98;

        next.marlin_tail.offset = Vec3::new(0.0, -13.0, 8.0) / 11.0;
        next.marlin_tail.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.marlin_tail.scale = Vec3::one() / 11.0;

        next.marlin_fin_l.offset = Vec3::new(0.0, -11.7, 11.0) / 11.0;
        next.marlin_fin_l.ori = Quaternion::rotation_y(0.0);
        next.marlin_fin_l.scale = Vec3::one() / 11.0;

        next.marlin_fin_r.offset = Vec3::new(0.0, 0.0, 12.0) / 11.0;
        next.marlin_fin_r.ori = Quaternion::rotation_y(0.0);
        next.marlin_fin_r.scale = Vec3::one() / 10.5;
        next
    }
}
