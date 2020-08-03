use super::{super::Animation, CritterAttr, CritterSkeleton};
//use std::f32::consts::PI;
use super::super::vek::*;

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Dependency = (f32, f64);
    type Skeleton = CritterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"critter_jump\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "critter_jump")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        _anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &CritterAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        next.head.offset = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1);
        next.head.ori = Quaternion::rotation_z(0.8) * Quaternion::rotation_x(0.5);
        next.head.scale = Vec3::one();

        next.chest.offset = Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1) / 18.0;
        next.chest.ori = Quaternion::rotation_y(0.0);
        next.chest.scale = Vec3::one() / 18.0;

        next.feet_f.offset = Vec3::new(0.0, skeleton_attr.feet_f.0, skeleton_attr.feet_f.1);
        next.feet_f.ori = Quaternion::rotation_z(0.0);
        next.feet_f.scale = Vec3::one();

        next.feet_b.offset = Vec3::new(0.0, skeleton_attr.feet_b.0, skeleton_attr.feet_b.1);
        next.feet_b.ori = Quaternion::rotation_x(0.0);
        next.feet_b.scale = Vec3::one();

        next.tail.offset = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
        next.tail.ori = Quaternion::rotation_y(0.0);
        next.tail.scale = Vec3::one();

        next
    }
}
