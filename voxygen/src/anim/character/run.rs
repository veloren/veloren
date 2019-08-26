use super::{
    super::{Animation, SkeletonAttr},
    CharacterSkeleton,
};
use std::f32::consts::PI;
use std::ops::Mul;
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Skeleton = CharacterSkeleton;
    type Dependency = (f32, f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (velocity, global_time): Self::Dependency,
        anim_time: f64,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 12.0).sin();
        let wave_cos = (anim_time as f32 * 12.0).cos();
        let wave_diff = (anim_time as f32 * 12.0 + PI / 2.0).sin();
        let wave_cos_dub = (anim_time as f32 * 24.0).cos();
        let wave_stop = (anim_time as f32 * 2.6).min(PI / 2.0).sin();

        next.l_foot.offset = Vec3::new(-3.4, 0.0 + wave_cos * 1.0, 6.0 - wave_cos_dub * 0.7);
        next.l_foot.ori = Quaternion::rotation_x(-0.0 - wave_cos * 1.5);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(3.4, 0.0 - wave_cos * 1.0, 6.0 - wave_cos_dub * 0.7);
        next.r_foot.ori = Quaternion::rotation_x(-0.0 + wave_cos * 1.5);
        next.r_foot.scale = Vec3::one();
        
        next
    }
}
