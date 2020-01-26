use super::{super::Animation, BirdMediumSkeleton, SkeletonAttr};
//use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Skeleton = BirdMediumSkeleton;
    type Dependency = (f32, f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (_velocity, _global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 18.0).sin();
        let wave_cos = (anim_time as f32 * 18.0).cos();

        next.head.offset =
            Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1 + wave * 0.5) / 11.0;
        next.head.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0 + wave_cos * 0.03);
        next.head.scale = Vec3::one();

        next.torso.offset = Vec3::new(
            0.0,
            skeleton_attr.chest.0,
            wave * 0.3 + skeleton_attr.chest.1,
        ) / 11.0;
        next.torso.ori = Quaternion::rotation_y(wave * 0.03);
        next.torso.scale = Vec3::one() / 11.0;

        next.tail.offset = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1) / 11.0;
        next.tail.ori = Quaternion::rotation_x(wave_cos * 0.03);
        next.tail.scale = Vec3::one();

        next.wing_l.offset = Vec3::new(
            -skeleton_attr.wing.0,
            skeleton_attr.wing.1,
            skeleton_attr.wing.2,
        ) / 11.0;
        next.wing_l.ori = Quaternion::rotation_z(0.0);
        next.wing_l.scale = Vec3::one() * 1.05;

        next.wing_r.offset = Vec3::new(
            skeleton_attr.wing.0,
            skeleton_attr.wing.1,
            skeleton_attr.wing.2,
        ) / 11.0;
        next.wing_r.ori = Quaternion::rotation_y(0.0);
        next.wing_r.scale = Vec3::one() * 1.05;

        next.leg_l.offset = Vec3::new(
            -skeleton_attr.foot.0,
            skeleton_attr.foot.1,
            skeleton_attr.foot.2,
        ) / 11.0;
        next.leg_l.ori = Quaternion::rotation_x(wave_cos * 1.0);
        next.leg_l.scale = Vec3::one() / 11.0;

        next.leg_r.offset = Vec3::new(
            skeleton_attr.foot.0,
            skeleton_attr.foot.1,
            skeleton_attr.foot.2,
        ) / 11.0;
        next.leg_r.ori = Quaternion::rotation_x(wave * 1.0);
        next.leg_r.scale = Vec3::one() / 11.0;
        next
    }
}
