use super::{super::Animation, BipedLargeSkeleton, SkeletonAttr};
use std::f32::consts::PI;
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (f32, f64);
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_run")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, _global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab = 0.55; //.65
        let foothoril = (((1.0)
            / (0.4
                + (0.6)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin());
        let foothorir = (((1.0)
            / (0.4
                + (0.6)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin());
        let footvertl = (anim_time as f32 * 16.0 * lab as f32).sin();
        let footvertr = (anim_time as f32 * 16.0 * lab as f32 + PI).sin();
        let handhoril = (anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin();
        let handhorir = (anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin();

        let footrotl = (((5.0)
            / (2.5
                + (2.5)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin());

        let footrotr = (((5.0)
            / (1.0
                + (4.0)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin());

        let short = (anim_time as f32 * lab as f32 * 16.0).sin();

        let shortalt = (anim_time as f32 * lab as f32 * 16.0 + PI / 2.0).sin();

        next.head.offset = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1) * 1.02;
        next.head.ori = Quaternion::rotation_z(short * -0.18) * Quaternion::rotation_x(-0.05);
        next.head.scale = Vec3::one() * 1.02;

        next.upper_torso.offset = Vec3::new(
            0.0,
            skeleton_attr.upper_torso.0,
            skeleton_attr.upper_torso.1 + shortalt * -1.5,
        ) / 8.0;
        next.upper_torso.ori = Quaternion::rotation_z(short * 0.18);
        next.upper_torso.scale = Vec3::one() / 8.0;

        next.lower_torso.offset = Vec3::new(
            0.0,
            skeleton_attr.lower_torso.0,
            skeleton_attr.lower_torso.1,
        );
        next.lower_torso.ori = Quaternion::rotation_z(short * 0.15) * Quaternion::rotation_x(0.14);
        next.lower_torso.scale = Vec3::one() * 1.02;

        next.shoulder_l.offset = Vec3::new(
            -skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1 + foothoril * -3.0,
            skeleton_attr.shoulder.2,
        );
        next.shoulder_l.ori = Quaternion::rotation_x(footrotl * -0.36)
            * Quaternion::rotation_y(0.1)
            * Quaternion::rotation_z(footrotl * 0.3);
        next.shoulder_l.scale = Vec3::one();

        next.shoulder_r.offset = Vec3::new(
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1 + foothorir * -3.0,
            skeleton_attr.shoulder.2,
        );
        next.shoulder_r.ori = Quaternion::rotation_x(footrotr * -0.36)
            * Quaternion::rotation_y(-0.1)
            * Quaternion::rotation_z(footrotr * -0.3);
        next.shoulder_r.scale = Vec3::one();

        next.hand_l.offset = Vec3::new(
            -skeleton_attr.hand.0,
            skeleton_attr.hand.1,
            skeleton_attr.hand.2,
        );
        next.hand_l.ori = Quaternion::rotation_x(0.3 + (handhoril * -0.6).max(0.0))
            * Quaternion::rotation_y(handhoril * -0.1);
        next.hand_l.scale = Vec3::one() * 1.02;

        next.hand_r.offset = Vec3::new(
            skeleton_attr.hand.0,
            skeleton_attr.hand.1,
            skeleton_attr.hand.2,
        );
        next.hand_r.ori = Quaternion::rotation_x(0.3 + (handhorir * -0.6).max(0.0))
            * Quaternion::rotation_y(handhorir * 0.1);
        next.hand_r.scale = Vec3::one() * 1.02;

        next.leg_l.offset = Vec3::new(
            -skeleton_attr.leg.0,
            skeleton_attr.leg.1,
            skeleton_attr.leg.2,
        ) * 0.98;
        next.leg_l.ori =
            Quaternion::rotation_z(short * 0.18) * Quaternion::rotation_x(foothoril * 0.3);
        next.leg_l.scale = Vec3::one() * 0.98;

        next.leg_r.offset = Vec3::new(
            skeleton_attr.leg.0,
            skeleton_attr.leg.1,
            skeleton_attr.leg.2,
        ) * 0.98;

        next.leg_r.ori =
            Quaternion::rotation_z(short * 0.18) * Quaternion::rotation_x(foothorir * 0.3);
        next.leg_r.scale = Vec3::one() * 0.98;

        next.foot_l.offset = Vec3::new(
            -skeleton_attr.foot.0,
            4.0 + skeleton_attr.foot.1 + foothoril * 8.5,
            skeleton_attr.foot.2 + ((footvertl * 6.5).max(0.0)),
        ) / 8.0;
        next.foot_l.ori =
            Quaternion::rotation_x(-0.5 + footrotl * 0.85) * Quaternion::rotation_y(0.0);
        next.foot_l.scale = Vec3::one() / 8.0;

        next.foot_r.offset = Vec3::new(
            skeleton_attr.foot.0,
            4.0 + skeleton_attr.foot.1 + foothorir * 8.5,
            skeleton_attr.foot.2 + ((footvertr * 6.5).max(0.0)),
        ) / 8.0;
        next.foot_r.ori =
            Quaternion::rotation_x(-0.5 + footrotr * 0.85) * Quaternion::rotation_y(0.0);
        next.foot_r.scale = Vec3::one() / 8.0;

        next.torso.offset = Vec3::new(0.0, 0.0, 0.0);
        next.torso.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.25);
        next.torso.scale = Vec3::one();
        next
    }
}
