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

        let wave = (anim_time as f32 * 14.0).sin();
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

        next.grizzly_bear_upper_head.offset =
            Vec3::new(0.0, 9.5 + wave_quick_cos * 2.0, 15.0 + wave_med * 3.0) / 11.0;
        next.grizzly_bear_upper_head.ori =
            Quaternion::rotation_x(-0.12 + wave_quick_cos * 0.12 + wolf_look.y)
                * Quaternion::rotation_z(wolf_look.x);
        next.grizzly_bear_upper_head.scale = Vec3::one() / 10.88;

        next.grizzly_bear_lower_head.offset = Vec3::new(0.0, 4.5, 2.0 + wave_slow_cos * 1.0);
        next.grizzly_bear_lower_head.ori = Quaternion::rotation_x(wave_slow * 0.05);
        next.grizzly_bear_lower_head.scale = Vec3::one() * 1.01;

        next.grizzly_bear_upper_torso.offset = Vec3::new(0.0, 3.1, -4.5 + wave_med * 1.0);
        next.grizzly_bear_upper_torso.ori = Quaternion::rotation_z(0.0);
        next.grizzly_bear_upper_torso.scale = Vec3::one() * 0.98;

        next.grizzly_bear_lower_torso.offset = Vec3::new(0.0, -12.0, 10.0) / 11.0;
        next.grizzly_bear_lower_torso.ori = Quaternion::rotation_x(wave_quick * 0.18);
        next.grizzly_bear_lower_torso.scale = Vec3::one() / 11.0;

        next.grizzly_bear_ears.offset = Vec3::new(0.0, 0.75, 5.25);
        next.grizzly_bear_ears.ori = Quaternion::rotation_x(0.1);
        next.grizzly_bear_ears.scale = Vec3::one() * 1.05;

        next.grizzly_bear_leg_lf.offset = Vec3::new(-5.0, 5.0, 2.5) / 11.0;
        next.grizzly_bear_leg_lf.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_leg_lf.scale = Vec3::one() / 11.0;

        next.grizzly_bear_leg_rf.offset = Vec3::new(5.0, 5.0, 2.5) / 11.0;
        next.grizzly_bear_leg_rf.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_leg_rf.scale = Vec3::one() / 11.0;

        next.grizzly_bear_leg_lb.offset = Vec3::new(-5.0, -10.0, 2.5) / 11.0;
        next.grizzly_bear_leg_lb.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_leg_lb.scale = Vec3::one() / 11.0;

        next.grizzly_bear_leg_rb.offset = Vec3::new(5.0, -10.0, 2.5) / 11.0;
        next.grizzly_bear_leg_rb.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_leg_rb.scale = Vec3::one() / 11.0;

        next.grizzly_bear_foot_lf.offset = Vec3::new(-5.0, 5.0, 2.5) / 11.0;
        next.grizzly_bear_foot_lf.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_foot_lf.scale = Vec3::one() / 11.0;

        next.grizzly_bear_foot_rf.offset = Vec3::new(5.0, 5.0, 2.5) / 11.0;
        next.grizzly_bear_foot_rf.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_foot_rf.scale = Vec3::one() / 11.0;

        next.grizzly_bear_foot_lb.offset = Vec3::new(-5.0, -10.0, 2.5) / 11.0;
        next.grizzly_bear_foot_lb.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_foot_lb.scale = Vec3::one() / 11.0;

        next.grizzly_bear_foot_rb.offset = Vec3::new(5.0, -10.0, 2.5) / 11.0;
        next.grizzly_bear_foot_rb.ori = Quaternion::rotation_x(0.0);
        next.grizzly_bear_foot_rb.scale = Vec3::one() / 11.0;

        next
    }
}
