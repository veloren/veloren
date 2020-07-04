use super::{super::Animation, QuadrupedMediumSkeleton, SkeletonAttr};
use std::f32::consts::PI;
use vek::*;

pub struct AlphaAnimation;

impl Animation for AlphaAnimation {
    type Dependency = f64;
    type Skeleton = QuadrupedMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_medium_alpha\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_medium_alpha")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let short = (((1.0)
            / (0.1 + 0.9 * ((anim_time as f32 * 4.0 + PI * 2.5).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 4.0 + PI * 2.5).sin());
        let quick = (((1.0)
            / (0.001 + 0.9999 * ((anim_time as f32 * 4.0 + PI * 0.5).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 4.0 + PI * 0.5).sin());

        next.head_upper.offset =
            Vec3::new(0.0, skeleton_attr.head_upper.0, skeleton_attr.head_upper.1);
        next.head_upper.ori =
            Quaternion::rotation_y(short * -0.2) * Quaternion::rotation_x(0.1 + short * 0.2);
        next.head_upper.scale = Vec3::one();

        next.head_lower.offset =
            Vec3::new(0.0, skeleton_attr.head_lower.0, skeleton_attr.head_lower.1);
        next.head_lower.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.head_lower.scale = Vec3::one() * 1.02;

        next.jaw.offset = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
        next.jaw.ori = Quaternion::rotation_x(-0.3 + quick * 0.4);
        next.jaw.scale = Vec3::one() * 1.02;

        next.tail.offset = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
        next.tail.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.tail.scale = Vec3::one();

        next.torso_front.offset = Vec3::new(
            0.0,
            skeleton_attr.torso_front.0 + short * 2.8,
            skeleton_attr.torso_front.1 + short * 1.0,
        ) * skeleton_attr.scaler
            / 11.0;
        next.torso_front.ori = Quaternion::rotation_y(short * -0.1);
        next.torso_front.scale = Vec3::one() * skeleton_attr.scaler / 11.0;

        next.torso_back.offset =
            Vec3::new(0.0, skeleton_attr.torso_back.0, skeleton_attr.torso_back.1);
        next.torso_back.ori = Quaternion::rotation_y(short * -0.1)
            * Quaternion::rotation_z(0.0)
            * Quaternion::rotation_x(0.0);
        next.torso_back.scale = Vec3::one();

        next.ears.offset = Vec3::new(0.0, skeleton_attr.ears.0, skeleton_attr.ears.1);
        next.ears.ori = Quaternion::rotation_x(0.0);
        next.ears.scale = Vec3::one() * 1.02;

        next.leg_fl.offset = Vec3::new(
            -skeleton_attr.leg_f.0,
            skeleton_attr.leg_f.1,
            skeleton_attr.leg_f.2,
        );

        next.leg_fl.ori =
            Quaternion::rotation_x(short * -0.1) * Quaternion::rotation_y(short * 0.15);
        next.leg_fl.scale = Vec3::one();

        next.leg_fr.offset = Vec3::new(
            skeleton_attr.leg_f.0,
            skeleton_attr.leg_f.1,
            skeleton_attr.leg_f.2,
        );
        next.leg_fr.ori =
            Quaternion::rotation_x(short * 0.3) * Quaternion::rotation_y(short * -0.2);
        next.leg_fr.scale = Vec3::one();

        next.leg_bl.offset = Vec3::new(
            -skeleton_attr.leg_b.0,
            skeleton_attr.leg_b.1,
            skeleton_attr.leg_b.2 + 1.0,
        );
        next.leg_bl.ori =
            Quaternion::rotation_x(-0.1 + short * -0.2) * Quaternion::rotation_y(short * 0.2);
        next.leg_bl.scale = Vec3::one();

        next.leg_br.offset = Vec3::new(
            skeleton_attr.leg_b.0,
            skeleton_attr.leg_b.1,
            skeleton_attr.leg_b.2 + 1.0,
        );
        next.leg_br.ori =
            Quaternion::rotation_x(-0.1 + short * -0.2) * Quaternion::rotation_y(0.1 + short * 0.2);
        next.leg_br.scale = Vec3::one();

        next.foot_fl.offset = Vec3::new(
            -skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2 + short * -0.2,
        );
        next.foot_fl.ori = Quaternion::rotation_x(short * -0.05);
        next.foot_fl.scale = Vec3::one();

        next.foot_fr.offset = Vec3::new(
            skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2 + short * -1.5,
        );
        next.foot_fr.ori =
            Quaternion::rotation_x(short * -0.2) * Quaternion::rotation_y(short * 0.15);
        next.foot_fr.scale = Vec3::one();

        next.foot_bl.offset = Vec3::new(
            -skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2 + short * -0.8,
        );
        next.foot_bl.ori =
            Quaternion::rotation_x(-0.2 + short * 0.2) * Quaternion::rotation_y(short * 0.15);
        next.foot_bl.scale = Vec3::one();

        next.foot_br.offset = Vec3::new(
            skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2 + short * -1.5,
        );
        next.foot_br.ori =
            Quaternion::rotation_x(-0.2 + short * 0.2) * Quaternion::rotation_y(short * 0.15);
        next.foot_br.scale = Vec3::one();
        next
    }
}
