use super::{
    super::{vek::*, Animation},
    GolemSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (f32, f64);
    type Skeleton = GolemSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"golem_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "golem_run")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, _global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab = 0.45; //.65
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

        next.head.position = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1) * 1.02;
        next.head.orientation = Quaternion::rotation_z(short * -0.3) * Quaternion::rotation_x(-0.2);
        next.head.scale = Vec3::one() * 1.02;

        next.upper_torso.position = Vec3::new(
            0.0,
            skeleton_attr.upper_torso.0,
            skeleton_attr.upper_torso.1 + short * 1.0,
        ) / 8.0;
        next.upper_torso.orientation =
            Quaternion::rotation_z(short * 0.40) * Quaternion::rotation_x(0.0);
        next.upper_torso.scale = Vec3::one() / 8.0;

        next.lower_torso.position = Vec3::new(
            0.0,
            skeleton_attr.lower_torso.0,
            skeleton_attr.lower_torso.1,
        );
        next.lower_torso.orientation = Quaternion::rotation_z(shortalt * 0.60);
        next.lower_torso.scale = Vec3::one();

        next.shoulder_l.position = Vec3::new(
            -skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.shoulder_l.orientation = Quaternion::rotation_z(footrotl * 0.5)
            * Quaternion::rotation_y(0.15)
            * Quaternion::rotation_x(footrotl * -0.95);
        next.shoulder_l.scale = Vec3::one();

        next.shoulder_r.position = Vec3::new(
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.shoulder_r.orientation = Quaternion::rotation_z(footrotr * -0.5)
            * Quaternion::rotation_y(-0.15)
            * Quaternion::rotation_x(footrotr * -0.95);
        next.shoulder_r.scale = Vec3::one();

        next.hand_l.position = Vec3::new(
            -skeleton_attr.hand.0,
            skeleton_attr.hand.1,
            skeleton_attr.hand.2,
        );
        next.hand_l.orientation = Quaternion::rotation_x(0.5 + footrotl * -1.1)
            * Quaternion::rotation_y(0.5)
            * Quaternion::rotation_z(-0.35 + footrotl * -1.0);
        next.hand_l.scale = Vec3::one() * 1.02;

        next.hand_r.position = Vec3::new(
            skeleton_attr.hand.0,
            skeleton_attr.hand.1,
            skeleton_attr.hand.2,
        );
        next.hand_r.orientation = Quaternion::rotation_x(0.5 + footrotr * -1.1)
            * Quaternion::rotation_y(-0.5)
            * Quaternion::rotation_z(0.35 + footrotr * 1.0);
        next.hand_r.scale = Vec3::one() * 1.02;

        next.leg_l.position = Vec3::new(
            -skeleton_attr.leg.0,
            skeleton_attr.leg.1,
            skeleton_attr.leg.2,
        ) * 1.02;
        next.leg_l.orientation = Quaternion::rotation_x(footrotl * 1.5)
            * Quaternion::rotation_y(-0.3)
            * Quaternion::rotation_z(footrotl * -0.5);
        next.leg_l.scale = Vec3::one() * 1.02;

        next.leg_r.position = Vec3::new(
            skeleton_attr.leg.0,
            skeleton_attr.leg.1,
            skeleton_attr.leg.2,
        ) * 1.02;

        next.leg_r.orientation = Quaternion::rotation_x(footrotr * 1.5)
            * Quaternion::rotation_y(0.3)
            * Quaternion::rotation_z(footrotr * 0.5);
        next.leg_r.scale = Vec3::one() * 1.02;

        next.foot_l.position = Vec3::new(
            -skeleton_attr.foot.0,
            skeleton_attr.foot.1 + foothoril * 13.0,
            skeleton_attr.foot.2 - 3.0 + (footvertl * 15.0).max(-2.0),
        );
        next.foot_l.orientation = Quaternion::rotation_x(footrotl * 1.8);
        next.foot_l.scale = Vec3::one() * 0.98;

        next.foot_r.position = Vec3::new(
            skeleton_attr.foot.0,
            skeleton_attr.foot.1 + foothorir * 13.0,
            skeleton_attr.foot.2 - 3.0 + (footvertr * 15.0).max(-2.0),
        );
        next.foot_r.orientation =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(footrotr * 1.8);
        next.foot_r.scale = Vec3::one() * 0.98;

        next.torso.position = Vec3::new(0.0, 0.0, short * 0.15);
        next.torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.2);
        next.torso.scale = Vec3::one();
        next
    }
}
