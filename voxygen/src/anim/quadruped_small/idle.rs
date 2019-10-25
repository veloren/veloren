use super::{
    super::{Animation, SkeletonAttr},
    QuadrupedSmallSkeleton,
};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Skeleton = QuadrupedSmallSkeleton;
    type Dependency = (f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        _skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 14.0).sin();
        let wave_slow = (anim_time as f32 * 3.5 + PI).sin();
        let wave_slow_cos = (anim_time as f32 * 3.5 + PI).cos();

        let pig_head_look = Vec2::new(
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

        next.pig_head.offset = Vec3::new(0.0, -2.0, -1.5 + wave * 0.2) / 11.0;
        next.pig_head.ori = Quaternion::rotation_z(pig_head_look.x)
            * Quaternion::rotation_x(pig_head_look.y + wave_slow_cos * 0.03);
        next.pig_head.scale = Vec3::one() / 10.5;

        next.pig_chest.offset = Vec3::new(wave_slow * 0.05, -9.0, 1.5 + wave_slow_cos * 0.4) / 11.0;
        next.pig_chest.ori = Quaternion::rotation_y(wave_slow * 0.05);
        next.pig_chest.scale = Vec3::one() / 11.0;

        next.pig_leg_lf.offset = Vec3::new(-4.5, 2.0, 1.5) / 11.0;
        next.pig_leg_lf.ori = Quaternion::rotation_x(wave_slow * 0.08);
        next.pig_leg_lf.scale = Vec3::one() / 11.0;

        next.pig_leg_rf.offset = Vec3::new(2.5, 2.0, 1.5) / 11.0;
        next.pig_leg_rf.ori = Quaternion::rotation_x(wave_slow_cos * 0.08);
        next.pig_leg_rf.scale = Vec3::one() / 11.0;

        next.pig_leg_lb.offset = Vec3::new(-4.5, -3.0, 1.5) / 11.0;
        next.pig_leg_lb.ori = Quaternion::rotation_x(wave_slow_cos * 0.08);
        next.pig_leg_lb.scale = Vec3::one() / 11.0;

        next.pig_leg_rb.offset = Vec3::new(2.5, -3.0, 1.5) / 11.0;
        next.pig_leg_rb.ori = Quaternion::rotation_x(wave_slow * 0.08);
        next.pig_leg_rb.scale = Vec3::one() / 11.0;

        next
    }
}
