use super::{
    super::{vek::*, Animation},
    GolemSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency<'a> = (Vec3<f32>, Vec3<f32>, Vec3<f32>, f32, f32);
    type Skeleton = GolemSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"golem_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "golem_run")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, orientation, last_ori, _global_time, acc_vel): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();
        let mixed_vel = acc_vel + anim_time * 2.0; //sets run frequency using speed, with anim_time setting a floor

        let lab: f32 = 0.45 * s_a.tempo;
        let speednorm = (speed / 7.0).powf(0.6);
        let foothoril =
            ((1.0 / (0.4 + (0.6) * ((mixed_vel * 2.0 * lab + PI * 1.4).sin()).powi(2))).sqrt())
                * ((mixed_vel * 2.0 * lab + PI * 1.4).sin())
                * speednorm;
        let foothorir =
            ((1.0 / (0.4 + (0.6) * ((mixed_vel * 2.0 * lab + PI * 0.4).sin()).powi(2))).sqrt())
                * ((mixed_vel * 2.0 * lab + PI * 0.4).sin())
                * speednorm;
        let footvertl = (mixed_vel * 2.0 * lab).sin() * speednorm;
        let footvertr = (mixed_vel * 2.0 * lab + PI).sin() * speednorm;

        let footrotl = ((1.0 / (0.5 + (0.5) * ((mixed_vel * 2.0 * lab + PI * 1.4).sin()).powi(2)))
            .sqrt())
            * ((mixed_vel * 2.0 * lab + PI * 1.4).sin())
            * speednorm;

        let footrotr = ((1.0 / (0.2 + (0.8) * ((mixed_vel * 2.0 * lab + PI * 0.4).sin()).powi(2)))
            .sqrt())
            * ((mixed_vel * 2.0 * lab + PI * 0.4).sin())
            * speednorm;

        let short = (mixed_vel * lab * 2.0).sin() * speednorm;
        let shortalt = (mixed_vel * lab * 2.0 + PI / 2.0).sin() * speednorm;
        let ori: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if vek::Vec2::new(ori, last_ori)
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

        next.head.scale = Vec3::one() * 1.02;
        next.jaw.scale = Vec3::one() * 1.02;
        next.hand_l.scale = Vec3::one() * 1.04;
        next.hand_r.scale = Vec3::one() * 1.04;
        next.leg_l.scale = Vec3::one() * 1.02;
        next.leg_r.scale = Vec3::one() * 1.02;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1) * 1.02;
        next.head.orientation =
            Quaternion::rotation_z(short * -0.3) * Quaternion::rotation_x(-0.2 * speednorm);

        next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1) * 1.02;

        next.upper_torso.position =
            Vec3::new(0.0, s_a.upper_torso.0, s_a.upper_torso.1 + short * 1.0);
        next.upper_torso.orientation = Quaternion::rotation_z(tilt * -4.0 + short * 0.40);

        next.lower_torso.position = Vec3::new(0.0, s_a.lower_torso.0, s_a.lower_torso.1);
        next.lower_torso.orientation = Quaternion::rotation_z(tilt * 4.0 + shortalt * 0.2);

        next.shoulder_l.position = Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_l.orientation = Quaternion::rotation_z(footrotl * 0.07)
            * Quaternion::rotation_y(0.15)
            * Quaternion::rotation_x(-0.2 + footrotl * -0.25);

        next.shoulder_r.position = Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_r.orientation = Quaternion::rotation_z(footrotr * -0.07)
            * Quaternion::rotation_y(-0.15 * speednorm)
            * Quaternion::rotation_x(-0.2 + footrotr * -0.25);

        next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2);
        next.hand_l.orientation = Quaternion::rotation_x(0.3 + footrotl * -0.06)
            * Quaternion::rotation_y(0.1 * speednorm)
            * Quaternion::rotation_z(-0.35 * speednorm + footrotl * -0.1);

        next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2);
        next.hand_r.orientation = Quaternion::rotation_x(0.3 + footrotr * -0.06)
            * Quaternion::rotation_y(-0.1 * speednorm)
            * Quaternion::rotation_z(0.35 * speednorm + footrotr * 0.1);

        next.leg_l.position = Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2) * 1.02;
        next.leg_l.orientation = Quaternion::rotation_x(footrotl * 0.3)
            * Quaternion::rotation_y(0.1 * speednorm)
            * Quaternion::rotation_z(footrotl * -0.2);

        next.leg_r.position = Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2) * 1.02;

        next.leg_r.orientation = Quaternion::rotation_x(footrotr * 0.3)
            * Quaternion::rotation_y(-0.1 * speednorm)
            * Quaternion::rotation_z(footrotr * 0.2);

        next.foot_l.position = Vec3::new(
            -s_a.foot.0,
            s_a.foot.1 + foothoril * 2.0,
            s_a.foot.2 + (footvertl * 3.0).max(0.0),
        );
        next.foot_l.orientation =
            Quaternion::rotation_x(footrotl * 0.2) * Quaternion::rotation_y(-0.08 * speednorm);

        next.foot_r.position = Vec3::new(
            s_a.foot.0,
            s_a.foot.1 + foothorir * 2.0,
            s_a.foot.2 + (footvertr * 3.0).max(0.0),
        );
        next.foot_r.orientation = Quaternion::rotation_z(0.0)
            * Quaternion::rotation_x(footrotr * 0.2)
            * Quaternion::rotation_y(0.08 * speednorm);

        next.torso.position = Vec3::new(0.0, 0.0, 0.0);
        next.torso.orientation = Quaternion::rotation_x(-0.2 * speednorm);
        next
    }
}
