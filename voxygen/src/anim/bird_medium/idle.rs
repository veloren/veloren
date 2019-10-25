use super::{
    super::{Animation, SkeletonAttr},
    BirdMediumSkeleton,
};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Skeleton = BirdMediumSkeleton;
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

        next.duck_m_head.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.duck_m_head.ori =
            Quaternion::rotation_z(duck_m_look.x) * Quaternion::rotation_x(duck_m_look.y);
        next.duck_m_head.scale = Vec3::one() / 10.88;

        next.duck_m_torso.offset = Vec3::new(0.0, 4.5 - wave_ultra_slow_cos * 0.12, 2.0);
        next.duck_m_torso.ori = Quaternion::rotation_x(0.0);
        next.duck_m_torso.scale = Vec3::one() * 1.01;

        next.duck_m_tail.offset = Vec3::new(0.0, 3.1, -4.5);
        next.duck_m_tail.ori = Quaternion::rotation_z(0.0);
        next.duck_m_tail.scale = Vec3::one() * 0.98;

        next.duck_m_wing_l.offset = Vec3::new(0.0, -13.0, 8.0) / 11.0;
        next.duck_m_wing_l.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.duck_m_wing_l.scale = Vec3::one() / 11.0;

        next.duck_m_wing_r.offset = Vec3::new(0.0, -11.7, 11.0) / 11.0;
        next.duck_m_wing_r.ori = Quaternion::rotation_y(0.0);
        next.duck_m_wing_r.scale = Vec3::one() / 11.0;

        next.duck_m_leg_l.offset = Vec3::new(0.0, 0.0, 12.0) / 11.0;
        next.duck_m_leg_l.ori = Quaternion::rotation_y(0.0);
        next.duck_m_leg_l.scale = Vec3::one() / 10.5;

        next.duck_m_leg_r.offset = Vec3::new(0.0, 0.75, 5.25);
        next.duck_m_leg_r.ori = Quaternion::rotation_x(0.0);
        next.duck_m_leg_r.scale = Vec3::one() * 1.00;
        next
    }
}
