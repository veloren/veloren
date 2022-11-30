use super::{
    super::{vek::*, Animation},
    QuadrupedLowSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency<'a> = (f32, Vec3<f32>, Vec3<f32>, f32, Vec3<f32>, f32);
    type Skeleton = QuadrupedLowSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_low_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_low_run")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, orientation, last_ori, _global_time, avg_vel, acc_vel): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = (Vec2::<f32>::from(velocity).magnitude()).min(15.0);

        let speednorm = (speed / 15.0).powf(0.25);
        let mixed_vel = acc_vel + anim_time * 6.0; //sets run frequency using speed, with anim_time setting a floor

        let lab: f32 = 0.8 * s_a.tempo;
        let x_tilt = avg_vel.z.atan2(avg_vel.xy().magnitude()).max(-0.7) * speednorm;

        let short = ((1.0 / (0.72 + 0.28 * ((mixed_vel * lab + PI * 0.25).sin()).powi(2))).sqrt())
            * ((mixed_vel * lab + PI * 0.25).sin())
            * speednorm;
        let shortalt = (mixed_vel * lab + PI * 0.25).sin();

        let foothoril = ((1.0 / (0.4 + (0.6) * ((mixed_vel * lab + PI * 1.45).sin()).powi(2)))
            .sqrt())
            * ((mixed_vel * lab + PI * 1.45).sin())
            * speednorm;
        let footvertl = (mixed_vel * lab + PI * 0.0).sin() * speednorm;

        let foothorir = ((1.0 / (0.4 + (0.6) * ((mixed_vel * lab + PI * 0.45).sin()).powi(2)))
            .sqrt())
            * ((mixed_vel * lab + PI * 0.45).sin())
            * speednorm;
        let footvertr = (mixed_vel * lab + PI).sin() * speednorm;

        //back feet
        let foothorilb = ((1.0 / (0.4 + (0.6) * ((mixed_vel * lab + PI * 1.05).sin()).powi(2)))
            .sqrt())
            * ((mixed_vel * lab + PI * 1.05).sin())
            * speednorm;
        let footvertlb = (mixed_vel * lab + PI * (-0.4)).sin() * speednorm;

        let foothorirb = ((1.0 / (0.4 + (0.6) * ((mixed_vel * lab + PI * 0.05).sin()).powi(2)))
            .sqrt())
            * ((mixed_vel * lab + PI * 0.05).sin())
            * speednorm;
        let footvertrb = (mixed_vel * lab + PI * 0.6).sin() * speednorm;

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

        next.jaw.scale = Vec3::one() * 0.98;
        next.tail_front.scale = Vec3::one() * 0.98;
        next.tail_rear.scale = Vec3::one() * 0.98;

        next.head_upper.position = Vec3::new(0.0, s_a.head_upper.0, s_a.head_upper.1);
        next.head_upper.orientation = Quaternion::rotation_x(-s_a.lean.0 + x_tilt * -1.0)
            * Quaternion::rotation_y(tilt * 0.3)
            * Quaternion::rotation_z(short * -0.06 + tilt * -1.5);

        next.head_lower.position = Vec3::new(0.0, s_a.head_lower.0, s_a.head_lower.1);
        next.head_lower.orientation = Quaternion::rotation_y(tilt * 1.0)
            * Quaternion::rotation_z(short * -0.15 + tilt * -0.8)
            * Quaternion::rotation_x(x_tilt * 0.4);

        next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);

        next.tail_front.position = Vec3::new(
            0.0,
            s_a.tail_front.0 + s_a.lean.0 * 2.0,
            s_a.tail_front.1 + s_a.lean.0 * 2.0,
        );
        next.tail_front.orientation =
            Quaternion::rotation_z(shortalt * -0.18 * s_a.lean.1 + tilt * 1.8)
                * Quaternion::rotation_y(shortalt * -0.1)
                * Quaternion::rotation_x(0.06 - s_a.lean.0 * 1.2 + x_tilt * 0.2);

        next.tail_rear.position = Vec3::new(0.0, s_a.tail_rear.0, s_a.tail_rear.1);
        next.tail_rear.orientation =
            Quaternion::rotation_z(shortalt * -0.25 * s_a.lean.1 + tilt * 1.6)
                * Quaternion::rotation_y(shortalt * 0.08)
                * Quaternion::rotation_x(-0.04 + x_tilt * 0.5);

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);
        next.chest.orientation = Quaternion::rotation_z(short * 0.13 + tilt * -1.9)
            * Quaternion::rotation_y(short * 0.12 + tilt * 0.7)
            * Quaternion::rotation_x(x_tilt + s_a.lean.0);

        next.foot_fl.position = Vec3::new(
            -s_a.feet_f.0,
            s_a.feet_f.1 + foothoril * -2.0,
            s_a.feet_f.2 + 1.0 * speednorm + ((footvertl * -1.8).max(-0.0)),
        );
        next.foot_fl.orientation =
            Quaternion::rotation_x(-0.2 * speednorm + footvertl * -0.45 * s_a.lean.1 - s_a.lean.0)
                * Quaternion::rotation_y(tilt * -1.0)
                * Quaternion::rotation_z(foothoril * 0.4 * s_a.lean.1 + tilt * -2.0);

        next.foot_fr.position = Vec3::new(
            s_a.feet_f.0,
            s_a.feet_f.1 + foothorir * -2.0,
            s_a.feet_f.2 + 1.0 * speednorm + ((footvertr * -1.8).max(-0.0)),
        );
        next.foot_fr.orientation =
            Quaternion::rotation_x(-0.2 * speednorm + footvertr * -0.45 * s_a.lean.1 - s_a.lean.0)
                * Quaternion::rotation_y(tilt * -1.0)
                * Quaternion::rotation_z(foothorir * -0.4 * s_a.lean.1 + tilt * -2.0);

        next.foot_bl.position = Vec3::new(
            -s_a.feet_b.0,
            s_a.feet_b.1 + foothorilb * -1.0,
            s_a.feet_b.2 + ((footvertlb * -1.2).max(-0.0)),
        );
        next.foot_bl.orientation =
            Quaternion::rotation_x(-0.2 * speednorm + footvertlb * -0.5 - s_a.lean.0)
                * Quaternion::rotation_y(tilt * -1.0)
                * Quaternion::rotation_z(foothorilb * 0.4 + tilt * -2.0);

        next.foot_br.position = Vec3::new(
            s_a.feet_b.0,
            s_a.feet_b.1 + foothorirb * -1.0,
            s_a.feet_b.2 + ((footvertrb * -1.2).max(-0.0)),
        );
        next.foot_br.orientation =
            Quaternion::rotation_x(-0.2 * speednorm + footvertrb * -0.5 - s_a.lean.0)
                * Quaternion::rotation_y(tilt * -1.0)
                * Quaternion::rotation_z(foothorirb * -0.4 + tilt * -2.0);

        next
    }
}
