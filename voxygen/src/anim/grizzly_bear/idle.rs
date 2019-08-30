use super::{
    super::{Animation, SkeletonAttr},
    GrizzlyBearSkeleton,
};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Skeleton = GrizzlyBearSkeleton;
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

        let grizzly_bear_look = Vec2::new(
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

        next.grizzly_bear_upper_head.offset = Vec3::new(-3.0, 14.5, 9.0 + wave_ultra_slow * 1.0) / 11.0;
        next.grizzly_bear_upper_head.ori = Quaternion::rotation_y(wave_slow_cos * 0.015);
        next.grizzly_bear_upper_head.scale = Vec3::one() / 11.0;

        next.grizzly_bear_lower_head.offset = Vec3::new(-2.0, 21.5, 10.0 + wave_ultra_slow * 1.0) / 11.0;
        next.grizzly_bear_lower_head.ori = Quaternion::rotation_y(wave_slow_cos * 0.015);
        next.grizzly_bear_lower_head.scale = Vec3::one() / 11.0;
        
        next.grizzly_bear_upper_torso.offset = Vec3::new(-6.0, 5.5, 12.0 + wave_ultra_slow * 1.2) / 11.0;
        next.grizzly_bear_upper_torso.ori = Quaternion::rotation_y(wave_slow_cos * 0.015);
        next.grizzly_bear_upper_torso.scale = Vec3::one() / 11.0;
        
        next.grizzly_bear_lower_torso.offset = Vec3::new(-6.0, -8.5, 12.0 + wave_ultra_slow * 0.7) / 11.0;
        next.grizzly_bear_lower_torso.ori = Quaternion::rotation_y(wave_slow * 0.015);
        next.grizzly_bear_lower_torso.scale = Vec3::one() / 11.0;

        next.grizzly_bear_leg_lf.offset = Vec3::new(8.5, 5.0, 9.0 + wave_ultra_slow * 0.7) / 11.0;
        next.grizzly_bear_leg_lf.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_leg_lf.scale = Vec3::one() / 11.0;

        next.grizzly_bear_leg_rf.offset = Vec3::new(-8.5, 5.0, 9.0 + wave_ultra_slow * 0.7) / 11.0;
        next.grizzly_bear_leg_rf.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_leg_rf.scale = Vec3::one() / 11.0;
        
        next.grizzly_bear_leg_lb.offset = Vec3::new(5.5, -15.5, 9.0 + wave_ultra_slow * 0.7) / 11.0;
        next.grizzly_bear_leg_lb.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_leg_lb.scale = Vec3::one() / 11.0;

        next.grizzly_bear_leg_rb.offset = Vec3::new(-7.6, -15.5, 9.0 + wave_ultra_slow * 0.7) / 11.0;
        next.grizzly_bear_leg_rb.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_leg_rb.scale = Vec3::one() / 11.0;

        next.grizzly_bear_foot_lf.offset = Vec3::new(-8.5, 8.0, 2.5) / 11.0;
        next.grizzly_bear_foot_lf.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_foot_lf.scale = Vec3::one() / 11.0;

        next.grizzly_bear_foot_rf.offset = Vec3::new(6.5, 8.0, 2.5) / 11.0;
        next.grizzly_bear_foot_rf.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_foot_rf.scale = Vec3::one() / 11.0;

        next.grizzly_bear_foot_lb.offset = Vec3::new(-7.5, -14.0, 2.5) / 11.0;
        next.grizzly_bear_foot_lb.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_foot_lb.scale = Vec3::one() / 11.0;

        next.grizzly_bear_foot_rb.offset = Vec3::new(5.5, -14.0, 2.5) / 11.0;
        next.grizzly_bear_foot_rb.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_foot_rb.scale = Vec3::one() / 11.0;

        next
    }
}
