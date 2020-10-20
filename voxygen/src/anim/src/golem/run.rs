use super::{
    super::{vek::*, Animation},
    GolemSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (f32, Vec3<f32>, Vec3<f32>, f64);
    type Skeleton = GolemSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"golem_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "golem_run")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, orientation, last_ori, _global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
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
        let ori: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if ::vek::Vec2::new(ori, last_ori)
            .map(|o| o.magnitude_squared())
            .map(|m| m > 0.001 && m.is_finite())
            .reduce_and()
            && ori.angle_between(last_ori).is_finite()
        {
            ori.angle_between(last_ori).min(0.2)
                * last_ori.determine_side(Vec2::zero(), ori).signum()
        } else {
            0.0
        } * 1.3;
        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1) * 1.02;
        next.head.orientation = Quaternion::rotation_z(short * -0.3) * Quaternion::rotation_x(-0.2);
        next.head.scale = Vec3::one() * 1.02;

        next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1) * 1.02;
        next.jaw.scale = Vec3::one() * 1.02;

        next.upper_torso.position =
            Vec3::new(0.0, s_a.upper_torso.0, s_a.upper_torso.1 + short * 1.0) / 8.0;
        next.upper_torso.orientation =
            Quaternion::rotation_z(tilt * -4.0 + short * 0.40) * Quaternion::rotation_x(0.0);
        next.upper_torso.scale = Vec3::one() / 8.0;

        next.lower_torso.position = Vec3::new(0.0, s_a.lower_torso.0, s_a.lower_torso.1);
        next.lower_torso.orientation = Quaternion::rotation_z(tilt * 4.0 + shortalt * 0.2);
        next.lower_torso.scale = Vec3::one();

        next.shoulder_l.position = Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_l.orientation = Quaternion::rotation_z(footrotl * 0.07)
            * Quaternion::rotation_y(0.15)
            * Quaternion::rotation_x(footrotl * -0.25);
        next.shoulder_l.scale = Vec3::one();

        next.shoulder_r.position = Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_r.orientation = Quaternion::rotation_z(footrotr * -0.07)
            * Quaternion::rotation_y(-0.15)
            * Quaternion::rotation_x(footrotr * -0.25);
        next.shoulder_r.scale = Vec3::one();

        next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2);
        next.hand_l.orientation = Quaternion::rotation_x(0.3 + footrotl * -0.06)
            * Quaternion::rotation_y(0.1)
            * Quaternion::rotation_z(-0.35 + footrotl * -0.1);
        next.hand_l.scale = Vec3::one() * 1.02;

        next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2);
        next.hand_r.orientation = Quaternion::rotation_x(0.3 + footrotr * -0.06)
            * Quaternion::rotation_y(-0.1)
            * Quaternion::rotation_z(0.35 + footrotr * 0.1);
        next.hand_r.scale = Vec3::one() * 1.02;

        next.leg_l.position = Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2) * 1.02;
        next.leg_l.orientation = Quaternion::rotation_x(footrotl * 0.3)
            * Quaternion::rotation_y(0.1)
            * Quaternion::rotation_z(footrotl * -0.2);
        next.leg_l.scale = Vec3::one() * 1.02;

        next.leg_r.position = Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2) * 1.02;

        next.leg_r.orientation = Quaternion::rotation_x(footrotr * 0.3)
            * Quaternion::rotation_y(-0.1)
            * Quaternion::rotation_z(footrotr * 0.2);
        next.leg_r.scale = Vec3::one() * 1.02;

        next.foot_l.position = Vec3::new(
            -s_a.foot.0,
            s_a.foot.1 + foothoril * 2.0,
            s_a.foot.2 + (footvertl * 3.0).max(0.0),
        );
        next.foot_l.orientation =
            Quaternion::rotation_x(footrotl * 0.2) * Quaternion::rotation_y(-0.08);
        next.foot_l.scale = Vec3::one() * 0.98;

        next.foot_r.position = Vec3::new(
            s_a.foot.0,
            s_a.foot.1 + foothorir * 2.0,
            s_a.foot.2 + (footvertr * 3.0).max(0.0),
        );
        next.foot_r.orientation = Quaternion::rotation_z(0.0)
            * Quaternion::rotation_x(footrotr * 0.2)
            * Quaternion::rotation_y(0.08);
        next.foot_r.scale = Vec3::one() * 0.98;

        next.torso.position = Vec3::new(0.0, 0.0, 0.0);
        next.torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.2);
        next.torso.scale = Vec3::one();
        next
    }
}
