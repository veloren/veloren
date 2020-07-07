use super::{super::Animation, QuadrupedSmallSkeleton, SkeletonAttr};
use std::f32::consts::PI;
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (f32, f64, Vec3<f32>);
    type Skeleton = QuadrupedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_small_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_small_run")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, _global_time, avg_vel): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let slow = (anim_time as f32 * 14.0).sin();
        let fast = (anim_time as f32 * 20.0).sin();
        let fast_alt = (anim_time as f32 * 20.0 + PI / 2.0).sin();
        let slow_alt = (anim_time as f32 * 14.0 + PI / 2.0).sin();
        let x_tilt = avg_vel.z.atan2(avg_vel.xy().magnitude()).max(-0.7);

        next.head.offset =
            Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1 + slow * 1.5) / 11.0;
        next.head.ori =
            Quaternion::rotation_x(0.2 + slow * 0.05) * Quaternion::rotation_y(slow_alt * 0.03);
        next.head.scale = Vec3::one() / 10.5;

        next.chest.offset = Vec3::new(
            0.0,
            skeleton_attr.chest.0,
            skeleton_attr.chest.1 + slow_alt * 1.2,
        ) / 11.0;
        next.chest.ori = Quaternion::rotation_x(slow * 0.1 + x_tilt);
        next.chest.scale = Vec3::one() / 11.0;

        next.leg_fl.offset = Vec3::new(
            -skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1 + fast * 0.8,
            skeleton_attr.feet_f.2 + fast_alt * 1.5,
        ) / 11.0;
        next.leg_fl.ori = Quaternion::rotation_x(fast * 0.3);
        next.leg_fl.scale = Vec3::one() / 11.0;

        next.leg_fr.offset = Vec3::new(
            skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1 + fast_alt * -0.8,
            skeleton_attr.feet_f.2 + fast * 1.5,
        ) / 11.0;
        next.leg_fr.ori = Quaternion::rotation_x(fast_alt * -0.3);
        next.leg_fr.scale = Vec3::one() / 11.0;

        next.leg_bl.offset = Vec3::new(
            -skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1 + fast_alt * -0.8,
            skeleton_attr.feet_b.2 + fast * 1.5,
        ) / 11.0;
        next.leg_bl.ori = Quaternion::rotation_x(fast_alt * -0.3);
        next.leg_bl.scale = Vec3::one() / 11.0;

        next.leg_br.offset = Vec3::new(
            skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1 + fast * 0.8,
            skeleton_attr.feet_b.2 + fast_alt * 1.5,
        ) / 11.0;
        next.leg_br.ori = Quaternion::rotation_x(fast * 0.3);
        next.leg_br.scale = Vec3::one() / 11.0;

        next.tail.offset = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
        next.tail.ori = Quaternion::rotation_z(0.0);
        next.tail.scale = Vec3::one();
        next
    }
}
