use super::{super::Animation, CritterAttr, CritterSkeleton};
//use std::f32::consts::PI;
use vek::*;

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Dependency = (f32, f64);
    type Skeleton = CritterSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        _anim_time: f64,
        _rate: &mut f32,
        _skeleton_attr: &CritterAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        next.head.offset = Vec3::new(0.0, 0.0, 0.0) / 18.0;
        next.head.ori = Quaternion::rotation_z(0.0);
        next.head.scale = Vec3::one() / 18.0;

        next.chest.offset = Vec3::new(0.0, 0.0, 0.0) / 18.0;
        next.chest.ori = Quaternion::rotation_x(0.0);
        next.chest.scale = Vec3::one() / 18.0;

        next.feet_f.offset = Vec3::new(0.0, 0.0, 0.0) / 18.0;
        next.feet_f.ori = Quaternion::rotation_z(0.0);
        next.feet_f.scale = Vec3::one() / 18.0;

        next.feet_b.offset = Vec3::new(0.0, 0.0, 0.0) / 18.0;
        next.feet_b.ori = Quaternion::rotation_x(0.0);
        next.feet_b.scale = Vec3::one() / 18.0;

        next.tail.offset = Vec3::new(0.0, 0.0, 0.0) / 18.0;
        next.tail.ori = Quaternion::rotation_x(0.0);
        next.tail.scale = Vec3::one() / 18.0;

        next
    }
}
