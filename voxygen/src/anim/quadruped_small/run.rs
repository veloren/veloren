use super::{
    super::{Animation, SkeletonAttr},
    QuadrupedSmallSkeleton,
};
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Skeleton = QuadrupedSmallSkeleton;
    type Dependency = (f32, f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (_velocity, _global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        _skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 14.0).sin();
        let wave_quick = (anim_time as f32 * 20.0).sin();
        let wave_quick_cos = (anim_time as f32 * 20.0).cos();
        let wave_cos = (anim_time as f32 * 14.0).cos();

        next.head.offset = Vec3::new(0.0, 0.0, -1.5 + wave * 1.5) / 11.0;
        next.head.ori =
            Quaternion::rotation_x(0.2 + wave * 0.05) * Quaternion::rotation_y(wave_cos * 0.03);
        next.head.scale = Vec3::one() / 10.5;

        next.chest.offset = Vec3::new(0.0, -9.0, 1.5 + wave_cos * 1.2) / 11.0;
        next.chest.ori = Quaternion::rotation_x(wave * 0.1);
        next.chest.scale = Vec3::one() / 11.0;

        next.leg_lf.offset =
            Vec3::new(-4.5, 2.0 + wave_quick * 0.8, 2.5 + wave_quick_cos * 1.5) / 11.0;
        next.leg_lf.ori = Quaternion::rotation_x(wave_quick * 0.3);
        next.leg_lf.scale = Vec3::one() / 11.0;

        next.leg_rf.offset =
            Vec3::new(2.5, 2.0 - wave_quick_cos * 0.8, 2.5 + wave_quick * 1.5) / 11.0;
        next.leg_rf.ori = Quaternion::rotation_x(wave_quick_cos * -0.3);
        next.leg_rf.scale = Vec3::one() / 11.0;

        next.leg_lb.offset =
            Vec3::new(-4.5, -3.0 - wave_quick_cos * 0.8, 2.5 + wave_quick * 1.5) / 11.0;
        next.leg_lb.ori = Quaternion::rotation_x(wave_quick_cos * -0.3);
        next.leg_lb.scale = Vec3::one() / 11.0;

        next.leg_rb.offset =
            Vec3::new(2.5, -3.0 + wave_quick * 0.8, 2.5 + wave_quick_cos * 1.5) / 11.0;
        next.leg_rb.ori = Quaternion::rotation_x(wave_quick * 0.3);
        next.leg_rb.scale = Vec3::one() / 11.0;

        next
    }
}
