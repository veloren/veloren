use super::{super::Animation, CritterAttr, CritterSkeleton};
//use std::{f32::consts::PI, ops::Mul};
use std::f32::consts::PI;
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (f32, f64);
    type Skeleton = CritterSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (_velocity, _global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &CritterAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 13.0).sin();
        let wave_cos = (anim_time as f32 * 13.0).sin();
        let wave_slow = (anim_time as f32 * 6.5 + PI).sin();

        next.head.offset = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1) / 18.0;
        next.head.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0 + wave * 0.03);
        next.head.scale = Vec3::one() / 18.0;

        next.chest.offset = Vec3::new(
            0.0,
            skeleton_attr.chest.0 + wave * 1.0,
            skeleton_attr.chest.1,
        ) / 18.0;
        next.chest.ori = Quaternion::rotation_y(wave_slow * 0.3);
        next.chest.scale = Vec3::one() / 18.0;

        next.feet_f.offset = Vec3::new(0.0, skeleton_attr.feet_f.0, skeleton_attr.feet_f.1) / 18.0;
        next.feet_f.ori = Quaternion::rotation_x(wave * 1.0);
        next.feet_f.scale = Vec3::one() / 18.0;

        next.feet_b.offset = Vec3::new(0.0, skeleton_attr.feet_b.0, skeleton_attr.feet_b.1) / 18.0;
        next.feet_b.ori = Quaternion::rotation_x(wave_cos * 1.0);
        next.feet_b.scale = Vec3::one() / 18.0;

        next.tail.offset =
            Vec3::new(0.0, skeleton_attr.tail.0 + wave * 1.0, skeleton_attr.tail.1) / 18.0;
        next.tail.ori = Quaternion::rotation_y(wave_slow * 0.25);
        next.tail.scale = Vec3::one() / 18.0;

        next
    }
}
