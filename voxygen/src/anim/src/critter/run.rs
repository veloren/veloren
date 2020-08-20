use super::{super::Animation, CritterAttr, CritterSkeleton};
//use std::{f32::consts::PI, ops::Mul};
use super::super::vek::*;
use std::f32::consts::PI;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (f32, f64);
    type Skeleton = CritterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"critter_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "critter_run")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, _global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &CritterAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 8.0).sin();
        let wavealt = (anim_time as f32 * 8.0 + PI / 2.0).sin();
        let wave_slow = (anim_time as f32 * 6.5 + PI).sin();

        next.head.position = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1);
        next.head.orientation =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0 + wave * 0.03);
        next.head.scale = Vec3::one();

        next.chest.position = Vec3::new(
            0.0,
            skeleton_attr.chest.0 + wave * 1.0,
            skeleton_attr.chest.1,
        ) / 18.0;
        next.chest.orientation = Quaternion::rotation_x(wave * 0.1);
        next.chest.scale = Vec3::one() / 18.0;

        next.feet_f.position = Vec3::new(0.0, skeleton_attr.feet_f.0, skeleton_attr.feet_f.1);
        next.feet_f.orientation =
            Quaternion::rotation_x(wave * 0.8) * Quaternion::rotation_z(wavealt / 6.0);
        next.feet_f.scale = Vec3::one();

        next.feet_b.position = Vec3::new(0.0, skeleton_attr.feet_b.0, skeleton_attr.feet_b.1);
        next.feet_b.orientation =
            Quaternion::rotation_x(wavealt * 0.8) * Quaternion::rotation_z(wavealt / 6.0);
        next.feet_b.scale = Vec3::one();

        next.tail.position =
            Vec3::new(0.0, skeleton_attr.tail.0 + wave * 1.0, skeleton_attr.tail.1);
        next.tail.orientation = Quaternion::rotation_y(wave_slow * 0.08);
        next.tail.scale = Vec3::one();

        next
    }
}
