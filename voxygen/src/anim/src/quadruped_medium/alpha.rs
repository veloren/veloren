use super::{
    super::{vek::*, Animation},
    QuadrupedMediumSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct AlphaAnimation;

impl Animation for AlphaAnimation {
    type Dependency = (f32, f64);
    type Skeleton = QuadrupedMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_medium_alpha\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_medium_alpha")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, _global_time): Self::Dependency,
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

        next.head_upper.position =
            Vec3::new(0.0, skeleton_attr.head_upper.0, skeleton_attr.head_upper.1);
        next.head_upper.orientation =
            Quaternion::rotation_y(short * -0.2) * Quaternion::rotation_x(0.1 + short * 0.2);
        next.head_upper.scale = Vec3::one();

        next.head_lower.position =
            Vec3::new(0.0, skeleton_attr.head_lower.0, skeleton_attr.head_lower.1);
        next.head_lower.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.head_lower.scale = Vec3::one() * 1.02;

        next.jaw.position = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
        next.jaw.orientation = Quaternion::rotation_x(-0.3 + quick * 0.4);
        next.jaw.scale = Vec3::one() * 1.02;

        next.tail.position = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
        next.tail.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.tail.scale = Vec3::one();

        next.torso_front.position = Vec3::new(
            0.0,
            skeleton_attr.torso_front.0 + short * 2.8,
            skeleton_attr.torso_front.1 + short * 1.0,
        ) * skeleton_attr.scaler
            / 11.0;
        next.torso_front.orientation = Quaternion::rotation_y(short * -0.1);
        next.torso_front.scale = Vec3::one() * skeleton_attr.scaler / 11.0;

        next.torso_back.position =
            Vec3::new(0.0, skeleton_attr.torso_back.0, skeleton_attr.torso_back.1);
        next.torso_back.orientation = Quaternion::rotation_y(short * -0.1)
            * Quaternion::rotation_z(0.0)
            * Quaternion::rotation_x(0.0);
        next.torso_back.scale = Vec3::one();

        next.ears.position = Vec3::new(0.0, skeleton_attr.ears.0, skeleton_attr.ears.1);
        next.ears.orientation = Quaternion::rotation_x(0.0);
        next.ears.scale = Vec3::one() * 1.02;
        if velocity < 1.0 {
            next.leg_fl.position = Vec3::new(
                -skeleton_attr.leg_f.0,
                skeleton_attr.leg_f.1,
                skeleton_attr.leg_f.2,
            );

            next.leg_fl.orientation =
                Quaternion::rotation_x(short * -0.1) * Quaternion::rotation_y(short * 0.15);
            next.leg_fl.scale = Vec3::one();

            next.leg_fr.position = Vec3::new(
                skeleton_attr.leg_f.0,
                skeleton_attr.leg_f.1,
                skeleton_attr.leg_f.2,
            );
            next.leg_fr.orientation =
                Quaternion::rotation_x(short * 0.3) * Quaternion::rotation_y(short * -0.2);
            next.leg_fr.scale = Vec3::one();

            next.leg_bl.position = Vec3::new(
                -skeleton_attr.leg_b.0,
                skeleton_attr.leg_b.1,
                skeleton_attr.leg_b.2 + 1.0,
            );
            next.leg_bl.orientation =
                Quaternion::rotation_x(-0.1 + short * -0.2) * Quaternion::rotation_y(short * 0.2);
            next.leg_bl.scale = Vec3::one();

            next.leg_br.position = Vec3::new(
                skeleton_attr.leg_b.0,
                skeleton_attr.leg_b.1,
                skeleton_attr.leg_b.2 + 1.0,
            );
            next.leg_br.orientation = Quaternion::rotation_x(-0.1 + short * -0.2)
                * Quaternion::rotation_y(0.1 + short * 0.2);
            next.leg_br.scale = Vec3::one();

            next.foot_fl.position = Vec3::new(
                -skeleton_attr.feet_f.0,
                skeleton_attr.feet_f.1,
                skeleton_attr.feet_f.2 + short * -0.2,
            );
            next.foot_fl.orientation = Quaternion::rotation_x(short * -0.05);
            next.foot_fl.scale = Vec3::one();

            next.foot_fr.position = Vec3::new(
                skeleton_attr.feet_f.0,
                skeleton_attr.feet_f.1,
                skeleton_attr.feet_f.2,
            );
            next.foot_fr.orientation =
                Quaternion::rotation_x(short * -0.4) * Quaternion::rotation_y(short * 0.15);
            next.foot_fr.scale = Vec3::one();

            next.foot_bl.position = Vec3::new(
                -skeleton_attr.feet_b.0,
                skeleton_attr.feet_b.1,
                skeleton_attr.feet_b.2 + short * -0.8,
            );
            next.foot_bl.orientation =
                Quaternion::rotation_x(-0.2 + short * 0.2) * Quaternion::rotation_y(short * 0.15);
            next.foot_bl.scale = Vec3::one();

            next.foot_br.position = Vec3::new(
                skeleton_attr.feet_b.0,
                skeleton_attr.feet_b.1,
                skeleton_attr.feet_b.2,
            );
            next.foot_br.orientation =
                Quaternion::rotation_x(-0.2 + short * 0.2) * Quaternion::rotation_y(short * 0.15);
            next.foot_br.scale = Vec3::one();
        } else {
        };
        next
    }
}
