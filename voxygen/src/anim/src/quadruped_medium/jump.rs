use super::{super::Animation, QuadrupedMediumSkeleton, SkeletonAttr};
use vek::*;

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Dependency = (f32, f64);
    type Skeleton = QuadrupedMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_medium_jump\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_medium_jump")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        _anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        next.head_upper.offset = Vec3::new(
            0.0,
            skeleton_attr.head_upper.0,
            skeleton_attr.head_upper.1 + 3.0,
        ) / 11.0;
        next.head_upper.ori = Quaternion::rotation_z(0.8) * Quaternion::rotation_x(0.5);
        next.head_upper.scale = Vec3::one() / 10.98;

        next.head_lower.offset =
            Vec3::new(0.0, skeleton_attr.head_lower.0, skeleton_attr.head_lower.1);
        next.head_lower.ori = Quaternion::rotation_x(-0.4);
        next.head_lower.scale = Vec3::one() * 1.02;

        next.jaw.offset = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
        next.jaw.ori = Quaternion::rotation_x(0.0);
        next.jaw.scale = Vec3::one() * 0.98;

        next.tail.offset = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
        next.tail.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.tail.scale = Vec3::one();

        next.torso_back.offset = Vec3::new(
            0.0,
            skeleton_attr.torso_back.0,
            skeleton_attr.torso_back.1 + 2.0,
        ) / 11.0;
        next.torso_back.ori = Quaternion::rotation_z(-0.8)
            * Quaternion::rotation_z(0.0)
            * Quaternion::rotation_x(0.2);
        next.torso_back.scale = Vec3::one() / 11.0;

        next.torso_front.offset =
            Vec3::new(0.0, skeleton_attr.torso_front.0, skeleton_attr.torso_front.1) / 11.0;
        next.torso_front.ori = Quaternion::rotation_x(-0.4);
        next.torso_front.scale = Vec3::one() / 10.98;

        next.ears.offset = Vec3::new(0.0, skeleton_attr.ears.0, skeleton_attr.ears.1);
        next.ears.ori = Quaternion::rotation_x(0.0);
        next.ears.scale = Vec3::one() / 1.02;

        next.foot_fl.offset = Vec3::new(
            -skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2,
        ) / 11.0;
        next.foot_fl.ori = Quaternion::rotation_x(0.0);
        next.foot_fl.scale = Vec3::one() / 11.0;

        next.foot_fr.offset = Vec3::new(
            skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2,
        ) / 11.0;
        next.foot_fr.ori = Quaternion::rotation_x(0.0);
        next.foot_fr.scale = Vec3::one() / 11.0;

        next.foot_bl.offset = Vec3::new(
            -skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2,
        ) / 11.0;
        next.foot_bl.ori = Quaternion::rotation_x(0.0);
        next.foot_bl.scale = Vec3::one() / 11.0;

        next.foot_br.offset = Vec3::new(
            skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2,
        ) / 11.0;
        next.foot_br.ori = Quaternion::rotation_x(0.0);
        next.foot_br.scale = Vec3::one() / 11.0;

        next
    }
}
