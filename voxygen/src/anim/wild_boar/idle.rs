use super::{
    super::{Animation, SkeletonAttr},
    WildBoarSkeleton,
};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Skeleton = WildBoarSkeleton;
    type Dependency = (f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency,
        anim_time: f64,
        _skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave_ultra_slow = (anim_time as f32 * 1.0 + PI).sin();
        let wave_ultra_slow_cos = (anim_time as f32 * 1.0 + PI).cos();
        let wave_slow = (anim_time as f32 * 3.5 + PI).sin();
        let wave_slow_cos = (anim_time as f32 * 3.5 + PI).cos();

        let wild_boar_look = Vec2::new(
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

        next.wild_boar_head.offset = Vec3::new(-1.5, 5.5, 6.0 + wave_ultra_slow * 0.8) / 11.0;
        next.wild_boar_head.ori = Quaternion::rotation_x(wave_slow_cos * 0.015);
        next.wild_boar_head.scale = Vec3::one() / 11.0;
        
        next.wild_boar_torso.offset = Vec3::new(-1.5, 2.5, 10.0 + wave_ultra_slow * 1.0) / 11.0;
        next.wild_boar_torso.ori = Quaternion::rotation_y(wave_slow_cos * 0.015);
        next.wild_boar_torso.scale = Vec3::one() / 11.0;
        
        next.wild_boar_tail.offset = Vec3::new(0.5, -1.5, 8.5 + wave_ultra_slow * 1.0) / 11.0;
        next.wild_boar_tail.ori = Quaternion::rotation_y(wave_slow * 0.15);
        next.wild_boar_tail.scale = Vec3::one() / 11.0;

        next.wild_boar_leg_lf.offset = Vec3::new(-3.0, 1.0, 8.0 + wave_ultra_slow * 0.9) / 11.0;
        next.wild_boar_leg_lf.ori = Quaternion::rotation_x(0.0);
        next.wild_boar_leg_lf.scale = Vec3::one() / 11.0;

        next.wild_boar_leg_rf.offset = Vec3::new(6.0, 1.0, 8.0 + wave_ultra_slow * 0.9) / 11.0;
        next.wild_boar_leg_rf.ori = Quaternion::rotation_x(0.0);
        next.wild_boar_leg_rf.scale = Vec3::one() / 11.0;
        
        next.wild_boar_leg_lb.offset = Vec3::new(-2.0, -6.0, 7.5 + wave_ultra_slow * 0.85) / 11.0;
        next.wild_boar_leg_lb.ori = Quaternion::rotation_x(0.0);
        next.wild_boar_leg_lb.scale = Vec3::one() / 11.0;

        next.wild_boar_leg_rb.offset = Vec3::new(5.0, -6.0, 7.5 + wave_ultra_slow * 0.85) / 11.0;
        next.wild_boar_leg_rb.ori = Quaternion::rotation_x(0.0);
        next.wild_boar_leg_rb.scale = Vec3::one() / 11.0;

        next.wild_boar_foot_lf.offset = Vec3::new(-3.5, 5.0, 2.5) / 11.0;
        next.wild_boar_foot_lf.ori = Quaternion::rotation_x(0.0);
        next.wild_boar_foot_lf.scale = Vec3::one() / 11.0;

        next.wild_boar_foot_rf.offset = Vec3::new(6.5, 5.0, 2.5) / 11.0;
        next.wild_boar_foot_rf.ori = Quaternion::rotation_x(0.0);
        next.wild_boar_foot_rf.scale = Vec3::one() / 11.0;

        next.wild_boar_foot_lb.offset = Vec3::new(-2.5, -4.0, 2.5) / 11.0;
        next.wild_boar_foot_lb.ori = Quaternion::rotation_x(0.0);
        next.wild_boar_foot_lb.scale = Vec3::one() / 11.0;

        next.wild_boar_foot_rb.offset = Vec3::new(5.5, -4.0, 2.5) / 11.0;
        next.wild_boar_foot_rb.ori = Quaternion::rotation_x(0.0);
        next.wild_boar_foot_rb.scale = Vec3::one() / 11.0;

        next
    }
}
