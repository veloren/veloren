use super::{
    super::{Animation, SkeletonAttr},
    BirdSmallSkeleton,
};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Skeleton = BirdSmallSkeleton;
    type Dependency = (f32, f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (_velocity, global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        _skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave_ultra_slow = (anim_time as f32 * 1.0 + PI).sin();
        let wave_ultra_slow_cos = (anim_time as f32 * 1.0 + PI).cos();
        let wave_slow = (anim_time as f32 * 3.5 + PI).sin();
        let wave_slow_cos = (anim_time as f32 * 3.5 + PI).cos();

        let duck_look = Vec2::new(
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

        next.crow_head.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.crow_head.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.crow_head.scale = Vec3::one() / 10.88;

        next.crow_torso.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.crow_torso.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.crow_torso.scale = Vec3::one() / 10.88;

        next.crow_wing_l.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.crow_wing_l.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.crow_wing_l.scale = Vec3::one() / 10.88;

        next.crow_wing_r.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.crow_wing_r.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.crow_wing_r.scale = Vec3::one() / 10.88;

        next
    }
}
