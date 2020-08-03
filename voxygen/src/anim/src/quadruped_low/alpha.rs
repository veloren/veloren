use super::{
    super::{vek::*, Animation},
    QuadrupedLowSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct AlphaAnimation;

impl Animation for AlphaAnimation {
    type Dependency = f64;
    type Skeleton = QuadrupedLowSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_low_alpha\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_low_alpha")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let short = (((1.0)
            / (0.1 + 0.9 * ((anim_time as f32 * 8.0 + PI * 2.5).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 8.0 + PI * 2.5).sin());
        let quick = (((1.0)
            / (0.001 + 0.9999 * ((anim_time as f32 * 7.0 + PI * 0.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 7.0 + PI * 0.0).sin());
        next.head_upper.offset =
            Vec3::new(0.0, skeleton_attr.head_upper.0, skeleton_attr.head_upper.1);
        next.head_upper.ori = Quaternion::rotation_z(short * 0.3) * Quaternion::rotation_x(0.0);
        next.head_upper.scale = Vec3::one();

        next.head_lower.offset =
            Vec3::new(0.0, skeleton_attr.head_lower.0, skeleton_attr.head_lower.1);
        next.head_lower.ori =
            Quaternion::rotation_z(short * 0.2) * Quaternion::rotation_y(short * -0.4);
        next.head_lower.scale = Vec3::one();

        next.jaw.offset = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
        next.jaw.ori = Quaternion::rotation_x(-0.2 + quick * 0.3);
        next.jaw.scale = Vec3::one() * 0.98;

        next.chest.offset = Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1)
            * skeleton_attr.scaler
            / 11.0;
        next.chest.ori = Quaternion::rotation_y(short * -0.07);
        next.chest.scale = Vec3::one() * skeleton_attr.scaler / 11.0;

        next.tail_front.offset =
            Vec3::new(0.0, skeleton_attr.tail_front.0, skeleton_attr.tail_front.1);
        next.tail_front.ori = Quaternion::rotation_x(0.15)
            * Quaternion::rotation_y(short * 0.2)
            * Quaternion::rotation_z(short * 0.3);
        next.tail_front.scale = Vec3::one() * 0.98;

        next.tail_rear.offset =
            Vec3::new(0.0, skeleton_attr.tail_rear.0, skeleton_attr.tail_rear.1);
        next.tail_rear.ori = Quaternion::rotation_y(short * 0.5)
            * Quaternion::rotation_x(-0.12)
            * Quaternion::rotation_z(short * 0.3);
        next.tail_rear.scale = Vec3::one() * 0.98;

        next.foot_fl.offset = Vec3::new(
            -skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2,
        );
        next.foot_fl.ori = Quaternion::rotation_y(short * 0.12);
        next.foot_fl.scale = Vec3::one();

        next.foot_fr.offset = Vec3::new(
            skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2,
        );
        next.foot_fr.ori = Quaternion::rotation_y(short * 0.12);
        next.foot_fr.scale = Vec3::one();

        next.foot_bl.offset = Vec3::new(
            -skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2,
        );
        next.foot_bl.ori = Quaternion::rotation_y(short * 0.12);
        next.foot_bl.scale = Vec3::one();

        next.foot_br.offset = Vec3::new(
            skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2,
        );
        next.foot_br.ori = Quaternion::rotation_y(short * 0.12);
        next.foot_br.scale = Vec3::one();

        next
    }
}
