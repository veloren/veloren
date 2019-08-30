use super::{
    super::{Animation, SkeletonAttr},
    StagSkeleton,
};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Skeleton = StagSkeleton;
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

        let stag_look = Vec2::new(
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

        next.stag_head.offset = Vec3::new(-5.2, 20.0, 26.0 + wave_ultra_slow * 1.0) / 11.0;
        next.stag_head.ori = Quaternion::rotation_y(wave_slow_cos * 0.015);
        next.stag_head.scale = Vec3::one() / 8.0;
        
        next.stag_torso.offset = Vec3::new(-1.0, 3.0, 13.0 + wave_ultra_slow * 1.0) / 11.0;
        next.stag_torso.ori = Quaternion::rotation_y(wave_slow_cos * 0.015);
        next.stag_torso.scale = Vec3::one() / 9.0;
        
        next.stag_neck.offset = Vec3::new(0.2, 24.0, 13.0 + wave_ultra_slow * 1.0) / 11.0;
        next.stag_neck.ori = Quaternion::rotation_y(wave_slow_cos * 0.015);
        next.stag_neck.scale = Vec3::one() / 8.95;

        next.stag_leg_lf.offset = Vec3::new(-4.0, 8.0, 11.0 + wave_ultra_slow * 0.9) / 11.0;
        next.stag_leg_lf.ori = Quaternion::rotation_x(0.0);
        next.stag_leg_lf.scale = Vec3::one() / 9.0;

        next.stag_leg_rf.offset = Vec3::new(4.5, 8.0, 11.0 + wave_ultra_slow * 0.9) / 11.0;
        next.stag_leg_rf.ori = Quaternion::rotation_x(0.0);
        next.stag_leg_rf.scale = Vec3::one() / 9.0;
        
        next.stag_leg_lb.offset = Vec3::new(-3.0, -7.0, 10.5 + wave_ultra_slow * 0.85) / 11.0;
        next.stag_leg_lb.ori = Quaternion::rotation_x(0.0);
        next.stag_leg_lb.scale = Vec3::one() / 9.0;

        next.stag_leg_rb.offset = Vec3::new(4.5, -7.0, 10.5 + wave_ultra_slow * 0.85) / 11.0;
        next.stag_leg_rb.ori = Quaternion::rotation_x(0.0);
        next.stag_leg_rb.scale = Vec3::one() / 9.0;

        next.stag_foot_lf.offset = Vec3::new(-3.5, 14.0, 2.5) / 11.0;
        next.stag_foot_lf.ori = Quaternion::rotation_x(0.0);
        next.stag_foot_lf.scale = Vec3::one() / 9.0;

        next.stag_foot_rf.offset = Vec3::new(7.5, 14.0, 2.5) / 11.0;
        next.stag_foot_rf.ori = Quaternion::rotation_x(0.0);
        next.stag_foot_rf.scale = Vec3::one() / 9.0;

        next.stag_foot_lb.offset = Vec3::new(-2.0, 0.0, 2.5) / 11.0;
        next.stag_foot_lb.ori = Quaternion::rotation_x(0.0);
        next.stag_foot_lb.scale = Vec3::one() / 9.0;

        next.stag_foot_rb.offset = Vec3::new(6.5, 0.0, 2.5) / 11.0;
        next.stag_foot_rb.ori = Quaternion::rotation_x(0.0);
        next.stag_foot_rb.scale = Vec3::one() / 9.0;

        next
    }
}
