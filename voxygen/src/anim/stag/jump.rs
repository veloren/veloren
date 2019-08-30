use super::{
    super::{Animation, SkeletonAttr},
    StagSkeleton,
};
use std::f32::consts::PI;
use vek::*;

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Skeleton = StagSkeleton;
    type Dependency = (f32, f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        anim_time: f64,
        _skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 14.0).sin();
        let wave_slow = (anim_time as f32 * 3.5 + PI).sin();
        let wave_stop = (anim_time as f32 * 5.0).min(PI / 2.0).sin();

        next.stag_head.offset = Vec3::new(-5.2, 19.0, 25.0 + wave_stop * 4.8) / 11.0;
        next.stag_head.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.25);
        next.stag_head.scale = Vec3::one() / 8.0;

        next.stag_torso.offset = Vec3::new(-1.0, 3.0, 13.0) / 11.0;
        next.stag_torso.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.25);
        next.stag_torso.scale = Vec3::one() / 9.0;

        next.stag_neck.offset = Vec3::new(0.2, 24.0, 17.0) / 11.0;
        next.stag_neck.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.25);
        next.stag_neck.scale = Vec3::one() / 8.95;

        next.stag_leg_lf.offset = Vec3::new(-4.0, 8.0, 11.0) / 11.0;
        next.stag_leg_lf.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.25);
        next.stag_leg_lf.scale = Vec3::one() / 9.0;

        next.stag_leg_rf.offset = Vec3::new(4.5, 8.0, 11.0) / 11.0;
        next.stag_leg_rf.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.25);
        next.stag_leg_rf.scale = Vec3::one() / 9.0;

        next.stag_leg_lb.offset = Vec3::new(-3.0, -7.0, 10.5) / 11.0;
        next.stag_leg_lb.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.25);
        next.stag_leg_lb.scale = Vec3::one() / 9.0;

        next.stag_leg_rb.offset = Vec3::new(4.5, -7.0, 10.5) / 11.0;
        next.stag_leg_rb.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.25);
        next.stag_leg_rb.scale = Vec3::one() / 9.0;

        next.stag_foot_lf.offset = Vec3::new(-3.5, 14.0, 2.5) / 11.0;
        next.stag_foot_lf.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.45);
        next.stag_foot_lf.scale = Vec3::one() / 9.0;

        next.stag_foot_rf.offset = Vec3::new(7.5, 14.0, 2.5) / 11.0;
        next.stag_foot_rf.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.45);
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

