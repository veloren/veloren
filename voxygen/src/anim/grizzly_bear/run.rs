use super::{
    super::{Animation, SkeletonAttr},
    GrizzlyBearSkeleton,
};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Skeleton = GrizzlyBearSkeleton;
    type Dependency = (f32, f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (_velocity, global_time): Self::Dependency,
        anim_time: f64,
        _skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 9.0).sin();
        let wavecos = (anim_time as f32 * 9.0).cos();
        let wave_ultra_slow = (anim_time as f32 * 1.0 + PI).sin();
        let wave_ultra_slow_cos = (anim_time as f32 * 1.0 + PI).cos();
        let wave_slow = (anim_time as f32 * 3.5 + PI).sin();
        let wave_slow_cos = (anim_time as f32 * 3.5 + PI).cos();
        let wave_quick = (anim_time as f32 * 18.0).sin();
        let wave_med = (anim_time as f32 * 12.0).sin();
        let wave_med_cos = (anim_time as f32 * 12.0).cos();
        let wave_quick_cos = (anim_time as f32 * 18.0).cos();

         let grizzly_bear_look = Vec2::new(
            ((global_time + anim_time) as f32 / 8.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.12,
            ((global_time + anim_time) as f32 / 8.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.06,
        );

        next.grizzly_bear_upper_head.offset = Vec3::new(0.0, 12.0 + wave * 0.9, 16.0 - wavecos * 1.4) / 8.46;
        next.grizzly_bear_upper_head.ori = Quaternion::rotation_x(grizzly_bear_look.y + wavecos * -0.07) * Quaternion::rotation_x(grizzly_bear_look.x);
        next.grizzly_bear_upper_head.scale = Vec3::one() / 8.46 * 1.02;

        next.grizzly_bear_lower_head.offset = Vec3::new(0.0, 9.0, -4.5);
        next.grizzly_bear_lower_head.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_lower_head.scale = Vec3::one();

        next.grizzly_bear_upper_torso.offset = Vec3::new(0.0, 5.5 + wave * 0.7, 13.0  - wavecos * 1.1) / 8.46;
        next.grizzly_bear_upper_torso.ori = Quaternion::rotation_x(wavecos * -0.07)*Quaternion::rotation_y(wave * -0.04);
        next.grizzly_bear_upper_torso.scale = Vec3::one() / 8.46;

        next.grizzly_bear_lower_torso.offset = Vec3::new(0.0, -11.5 + wave * 0.7, 13.0 - wavecos * 1.1) / 8.46;
        next.grizzly_bear_lower_torso.ori = Quaternion::rotation_x(wave * -0.07);
        next.grizzly_bear_lower_torso.scale = Vec3::one() / 8.46;

        next.grizzly_bear_leg_lf.offset = Vec3::new(-9.0, 10.0 + wave * 0.7 - wave * 0.8, 11.0 + wave * 1.0 - wavecos * 1.1) / 8.46;
        next.grizzly_bear_leg_lf.ori = Quaternion::rotation_x(wavecos * -0.3);
        next.grizzly_bear_leg_lf.scale = Vec3::one() / 8.46;

        next.grizzly_bear_leg_rf.offset = Vec3::new(9.0, 10.0 + wave * 0.7 + wavecos * 0.8, 11.0 - wave * 1.0 - wavecos * 1.1) / 8.46;
        next.grizzly_bear_leg_rf.ori = Quaternion::rotation_x(wavecos * 0.3);
        next.grizzly_bear_leg_rf.scale = Vec3::one() / 8.46;

        next.grizzly_bear_leg_lb.offset = Vec3::new(-6.5, -12.0, 14.0 ) / 8.46;
        next.grizzly_bear_leg_lb.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_leg_lb.scale = Vec3::one() / 8.46;

        next.grizzly_bear_leg_rb.offset = Vec3::new(6.5, -12.0, 14.0) / 8.46;
        next.grizzly_bear_leg_rb.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_leg_rb.scale = Vec3::one() / 8.46;

        next.grizzly_bear_foot_lf.offset = Vec3::new(1.0, 2.0, -4.0 + wave * 1.2);
        next.grizzly_bear_foot_lf.ori = Quaternion::rotation_x(wavecos * -0.45);
        next.grizzly_bear_foot_rb.scale = Vec3::one();

        next.grizzly_bear_foot_rf.offset = Vec3::new(-1.0, 2.0, -4.0 - wave * 1.2);
        next.grizzly_bear_foot_rf.ori = Quaternion::rotation_x(wavecos * 0.45);
        next.grizzly_bear_foot_rb.scale = Vec3::one();

        next.grizzly_bear_foot_lb.offset = Vec3::new(-1.0, 0.0, -6.5);
        next.grizzly_bear_foot_lb.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_foot_rb.scale = Vec3::one();

        next.grizzly_bear_foot_rb.offset = Vec3::new(1.0, 0.0, -6.5);
        next.grizzly_bear_foot_rb.ori = Quaternion::rotation_x(0.0 );
        next.grizzly_bear_foot_rb.scale = Vec3::one();

        next
    }
}
