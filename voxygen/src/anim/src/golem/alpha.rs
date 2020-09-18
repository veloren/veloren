use super::{
    super::{vek::*, Animation},
    GolemSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct AlphaAnimation;

impl Animation for AlphaAnimation {
    type Dependency = (f32, f64);
    type Skeleton = GolemSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"golem_alpha\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "golem_alpha")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab = 1.0;

        let slower = (((1.0)
            / (0.05
                + 0.95
                    * ((anim_time as f32 * lab as f32 * 8.0 - 0.5 * PI).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 8.0 - 0.5 * PI).sin())
            + 1.0;
        let twist = (anim_time as f32 * lab as f32 * 4.0).sin() + 0.5;

        let random = ((((2.0
            * (((global_time as f32 - anim_time as f32) * 10.0)
                - (((global_time as f32 - anim_time as f32) * 10.0).round())))
        .abs())
            * 10.0)
            .round())
            / 10.0;

        let switch = if random > 0.5 { 1.0 } else { -1.0 };
        println!("{:?}", random);
        if switch > 0.0 {
            next.head.position = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1) * 1.02;
            next.head.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.2);
            next.head.scale = Vec3::one() * 1.02;

            next.upper_torso.position = Vec3::new(
                0.0,
                skeleton_attr.upper_torso.0,
                skeleton_attr.upper_torso.1,
            ) / 8.0;
            next.upper_torso.orientation =
                Quaternion::rotation_z(twist * 1.1) * Quaternion::rotation_x(0.0);
            next.upper_torso.scale = Vec3::one() / 8.0;

            next.lower_torso.position = Vec3::new(
                0.0,
                skeleton_attr.lower_torso.0,
                skeleton_attr.lower_torso.1,
            );
            next.lower_torso.orientation =
                Quaternion::rotation_z(twist * -1.1) * Quaternion::rotation_x(0.0);
            next.lower_torso.scale = Vec3::one();

            next.shoulder_l.position = Vec3::new(
                -skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
            next.shoulder_l.scale = Vec3::one();

            next.shoulder_r.position = Vec3::new(
                skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_r.orientation =
                Quaternion::rotation_z(0.0) * Quaternion::rotation_x(slower * 0.4);
            next.shoulder_r.scale = Vec3::one();

            next.hand_l.position = Vec3::new(
                -skeleton_attr.hand.0,
                skeleton_attr.hand.1,
                skeleton_attr.hand.2,
            );
            next.hand_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
            next.hand_l.scale = Vec3::one() * 1.02;

            next.hand_r.position = Vec3::new(
                skeleton_attr.hand.0,
                skeleton_attr.hand.1,
                skeleton_attr.hand.2,
            );
            next.hand_r.orientation =
                Quaternion::rotation_z(0.0) * Quaternion::rotation_x(slower * 0.35);
            next.hand_r.scale = Vec3::one() * 1.02;
        } else {
            next.head.position = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1) * 1.02;
            next.head.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.2);
            next.head.scale = Vec3::one() * 1.02;

            next.upper_torso.position = Vec3::new(
                0.0,
                skeleton_attr.upper_torso.0,
                skeleton_attr.upper_torso.1,
            ) / 8.0;
            next.upper_torso.orientation =
                Quaternion::rotation_z(twist * -1.1) * Quaternion::rotation_x(0.0);
            next.upper_torso.scale = Vec3::one() / 8.0;

            next.lower_torso.position = Vec3::new(
                0.0,
                skeleton_attr.lower_torso.0,
                skeleton_attr.lower_torso.1,
            );
            next.lower_torso.orientation =
                Quaternion::rotation_z(twist * 1.1) * Quaternion::rotation_x(0.0);
            next.lower_torso.scale = Vec3::one();

            next.shoulder_l.position = Vec3::new(
                -skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_l.orientation =
                Quaternion::rotation_z(0.0) * Quaternion::rotation_x(slower * 0.4);
            next.shoulder_l.scale = Vec3::one();

            next.shoulder_r.position = Vec3::new(
                skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
            next.shoulder_r.scale = Vec3::one();

            next.hand_l.position = Vec3::new(
                -skeleton_attr.hand.0,
                skeleton_attr.hand.1,
                skeleton_attr.hand.2,
            );
            next.hand_l.orientation =
                Quaternion::rotation_z(0.0) * Quaternion::rotation_x(slower * 0.35);
            next.hand_l.scale = Vec3::one() * 1.02;

            next.hand_r.position = Vec3::new(
                skeleton_attr.hand.0,
                skeleton_attr.hand.1,
                skeleton_attr.hand.2,
            );
            next.hand_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
            next.hand_r.scale = Vec3::one() * 1.02;
        };
        /*
                next.leg_l.position = Vec3::new(
                    -skeleton_attr.leg.0,
                    skeleton_attr.leg.1,
                    skeleton_attr.leg.2,
                ) * 1.02;
                next.leg_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
                next.leg_l.scale = Vec3::one() * 1.02;

                next.leg_r.position = Vec3::new(
                    skeleton_attr.leg.0,
                    skeleton_attr.leg.1,
                    skeleton_attr.leg.2,
                ) * 1.02;
                next.leg_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
                next.leg_r.scale = Vec3::one() * 1.02;

                next.foot_l.position = Vec3::new(
                    -skeleton_attr.foot.0,
                    skeleton_attr.foot.1,
                    skeleton_attr.foot.2,
                );
                next.foot_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
                next.foot_l.scale = Vec3::one();

                next.foot_r.position = Vec3::new(
                    skeleton_attr.foot.0,
                    skeleton_attr.foot.1,
                    skeleton_attr.foot.2,
                );
                next.foot_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
                next.foot_r.scale = Vec3::one();
        */
        next.torso.position = Vec3::new(0.0, 0.0, 0.0);
        next.torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.torso.scale = Vec3::one();
        next
    }
}
