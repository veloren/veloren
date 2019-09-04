use super::{
    super::{Animation, SkeletonAttr},
    GrizzlyBearSkeleton,
};
use std::f32::consts::PI;
use vek::*;

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Skeleton = GrizzlyBearSkeleton;
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

        next.grizzly_bear_upper_head.offset = Vec3::new(0.0, 12.0, 16.0 + wave_stop * 4.8) / 11.0;
        next.grizzly_bear_upper_head.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.25);
        next.grizzly_bear_upper_head.scale = Vec3::one() / 8.46;

        next.grizzly_bear_lower_head.offset = Vec3::new(0.0, 9.5, -4.5 + wave_slow * 0.1);
        next.grizzly_bear_lower_head.ori = Quaternion::rotation_z(wave_slow * 0.015);
        next.grizzly_bear_lower_head.scale = Vec3::one();

        next.grizzly_bear_upper_torso.offset = Vec3::new(-8.0, 5.5, 15.6) / 11.0;
        next.grizzly_bear_upper_torso.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.25);
        next.grizzly_bear_upper_torso.scale = Vec3::one() / 8.46;

        next.grizzly_bear_lower_torso.offset = Vec3::new(-8.0, -11.5, 13.0) / 11.0;
        next.grizzly_bear_lower_torso.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.25);
        next.grizzly_bear_lower_torso.scale = Vec3::one() / 8.46;

        next.grizzly_bear_leg_lf.offset = Vec3::new(11.2, 8.0, 18.0) / 11.0;
        next.grizzly_bear_leg_lf.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.25);
        next.grizzly_bear_leg_lf.scale = Vec3::one() / 8.46;

        next.grizzly_bear_leg_rf.offset = Vec3::new(-11.2, 8.0, 18.0) / 11.0;
        next.grizzly_bear_leg_rf.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.25);
        next.grizzly_bear_leg_rf.scale = Vec3::one() / 8.46;

        next.grizzly_bear_leg_lb.offset = Vec3::new(8.2, -16.0, 14.0) / 11.0;
        next.grizzly_bear_leg_lb.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.25);
        next.grizzly_bear_leg_lb.scale = Vec3::one() / 8.46;

        next.grizzly_bear_leg_rb.offset = Vec3::new(-8.2, -16.0, 14.0) / 11.0;
        next.grizzly_bear_leg_rb.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.25);
        next.grizzly_bear_leg_rb.scale = Vec3::one() / 8.46;

        next.grizzly_bear_foot_lf.offset = Vec3::new(1.0, 2.0, -11.0);
        next.grizzly_bear_foot_lf.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.45);
        next.grizzly_bear_foot_lf.scale = Vec3::one();

        next.grizzly_bear_foot_rf.offset = Vec3::new(-1.0, 2.0, -11.0);
        next.grizzly_bear_foot_rf.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_slow * -0.45);
        next.grizzly_bear_foot_rf.scale = Vec3::one();

        next.grizzly_bear_foot_lb.offset = Vec3::new(1.0, 0.0, -8.0);
        next.grizzly_bear_foot_lb.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_foot_lb.scale = Vec3::one();

        next.grizzly_bear_foot_rb.offset = Vec3::new(-1.0, 0.0, -8.0);
        next.grizzly_bear_foot_rb.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_foot_rb.scale = Vec3::one();

        next
    }
}

