use super::{
    super::{vek::*, Animation},
    QuadrupedMediumSkeleton, SkeletonAttr,
};

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Dependency = f64;
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

        next.head.position = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1);
        next.head.orientation = Quaternion::rotation_z(0.4) * Quaternion::rotation_x(0.3);
        next.head.scale = Vec3::one();

        next.neck.position = Vec3::new(0.0, skeleton_attr.neck.0, skeleton_attr.neck.1);
        next.neck.orientation = Quaternion::rotation_z(0.2) * Quaternion::rotation_x(0.3);
        next.neck.scale = Vec3::one() * 1.02;

        next.jaw.position = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
        next.jaw.orientation = Quaternion::rotation_x(-0.4);
        next.jaw.scale = Vec3::one() * 1.02;

        next.tail.position = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
        next.tail.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.3);
        next.tail.scale = Vec3::one();

        next.torso_front.position = Vec3::new(
            0.0,
            skeleton_attr.torso_front.0,
            skeleton_attr.torso_front.1,
        ) * skeleton_attr.scaler
            / 11.0;
        next.torso_front.orientation = Quaternion::rotation_y(0.0);
        next.torso_front.scale = Vec3::one() * skeleton_attr.scaler / 11.0;

        next.torso_back.position =
            Vec3::new(0.0, skeleton_attr.torso_back.0, skeleton_attr.torso_back.1);
        next.torso_back.orientation = Quaternion::rotation_z(-0.3)
            * Quaternion::rotation_z(0.0)
            * Quaternion::rotation_x(0.0);
        next.torso_back.scale = Vec3::one();

        next.ears.position = Vec3::new(0.0, skeleton_attr.ears.0, skeleton_attr.ears.1);
        next.ears.orientation = Quaternion::rotation_x(0.6);
        next.ears.scale = Vec3::one() * 1.02;

        next.leg_fl.position = Vec3::new(
            -skeleton_attr.leg_f.0,
            skeleton_attr.leg_f.1,
            skeleton_attr.leg_f.2,
        );
        next.leg_fl.orientation = Quaternion::rotation_x(-0.4);
        next.leg_fl.scale = Vec3::one();

        next.leg_fr.position = Vec3::new(
            skeleton_attr.leg_f.0,
            skeleton_attr.leg_f.1,
            skeleton_attr.leg_f.2,
        );
        next.leg_fr.orientation = Quaternion::rotation_x(0.4);
        next.leg_fr.scale = Vec3::one();

        next.leg_bl.position = Vec3::new(
            -skeleton_attr.leg_b.0,
            skeleton_attr.leg_b.1,
            skeleton_attr.leg_b.2,
        );
        next.leg_bl.orientation = Quaternion::rotation_y(0.0);
        next.leg_bl.scale = Vec3::one();

        next.leg_br.position = Vec3::new(
            skeleton_attr.leg_b.0,
            skeleton_attr.leg_b.1,
            skeleton_attr.leg_b.2,
        );
        next.leg_br.orientation = Quaternion::rotation_y(0.0);
        next.leg_br.scale = Vec3::one();

        next.foot_fl.position = Vec3::new(
            -skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2,
        );
        next.foot_fl.orientation = Quaternion::rotation_x(-0.3);
        next.foot_fl.scale = Vec3::one();

        next.foot_fr.position = Vec3::new(
            skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2,
        );
        next.foot_fr.orientation = Quaternion::rotation_x(0.2);
        next.foot_fr.scale = Vec3::one();

        next.foot_bl.position = Vec3::new(
            -skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2,
        );
        next.foot_bl.orientation = Quaternion::rotation_x(0.0);
        next.foot_bl.scale = Vec3::one();

        next.foot_br.position = Vec3::new(
            skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2,
        );
        next.foot_br.orientation = Quaternion::rotation_x(0.0);
        next.foot_br.scale = Vec3::one();

        next
    }
}
