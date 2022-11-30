use super::{
    super::{vek::*, Animation},
    QuadrupedSmallSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency<'a> = (f32, Vec3<f32>, Vec3<f32>, f32, Vec3<f32>, f32);
    type Skeleton = QuadrupedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_small_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_small_run")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, orientation, last_ori, _global_time, avg_vel, acc_vel): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab: f32 = 0.42;
        let speed = (Vec2::<f32>::from(velocity).magnitude()).min(12.0);
        let speednorm = (speed / 12.0).powf(0.4);

        // acc_vel and anim_time mix to make sure phase lenght isn't starting at
        // +infinite
        let mixed_vel = acc_vel + anim_time * 12.0; //sets run frequency using speed, with anim_time setting a floor

        let speedmult = s_a.tempo;
        let short = (mixed_vel * lab * speedmult + PI * 1.0).sin() * speednorm;
        let shortalt = (mixed_vel * lab * speedmult + PI * 0.5).sin() * speednorm;

        let footvert = (mixed_vel * lab * speedmult + PI * 0.0).sin() * speednorm;
        let footvertt = (mixed_vel * lab * speedmult + PI * 0.4).sin() * speednorm;

        let footvertf = (mixed_vel * lab * speedmult + PI * 0.3).sin() * speednorm;
        let footverttf = (mixed_vel * lab * speedmult + PI * 0.7).sin() * speednorm;

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
        let x_tilt = avg_vel.z.atan2(avg_vel.xy().magnitude()) * speednorm;
        let vertcancel = 1.0 - s_a.lateral;
        next.leg_fl.scale = Vec3::one() * 1.02;
        next.leg_fr.scale = Vec3::one() * 1.02;
        next.leg_bl.scale = Vec3::one() * 1.02;
        next.leg_br.scale = Vec3::one() * 1.02;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_x(x_tilt * -0.5 + vertcancel * short * -0.2)
            * Quaternion::rotation_y(tilt * 0.8)
            * Quaternion::rotation_z(s_a.lateral * -short * 0.2 + tilt * -1.2);

        next.chest.position = Vec3::new(
            0.0,
            s_a.chest.0,
            s_a.chest.1 + 2.0 * speednorm * s_a.spring + shortalt * 3.0 * s_a.spring,
        );
        next.chest.orientation =
            Quaternion::rotation_x(vertcancel * short * 0.2 * s_a.spring + x_tilt)
                * Quaternion::rotation_y(tilt * 0.8)
                * Quaternion::rotation_z(s_a.lateral * short * 0.2 + tilt * -1.5);

        next.leg_fl.position = Vec3::new(
            -s_a.feet_f.0,
            s_a.feet_f.1 + footverttf * 3.0 * s_a.minimize,
            s_a.feet_f.2 + ((footvertf * -1.5).max(-1.0)),
        );
        next.leg_fl.orientation =
            Quaternion::rotation_x(0.2 * speednorm + s_a.maximize * footverttf * 0.65)
                * Quaternion::rotation_z(tilt * -0.5)
                * Quaternion::rotation_y(tilt * 1.5);

        next.leg_fr.position = Vec3::new(
            s_a.feet_f.0,
            s_a.feet_f.1 + footvertt * 3.0 * s_a.minimize,
            s_a.feet_f.2 + ((footvert * -1.5).max(-1.0)),
        );
        next.leg_fr.orientation =
            Quaternion::rotation_x(0.2 * speednorm + s_a.maximize * footvertt * 0.65)
                * Quaternion::rotation_z(tilt * -0.5)
                * Quaternion::rotation_y(tilt * 1.5);

        next.leg_bl.position = Vec3::new(
            -s_a.feet_b.0,
            s_a.feet_b.1 + footvertt * -1.4,
            s_a.feet_b.2 + ((footvert * 1.5).max(-1.0)),
        );
        next.leg_bl.orientation =
            Quaternion::rotation_x(-0.25 * speednorm + s_a.maximize * footvertt * -0.8)
                * Quaternion::rotation_y(tilt * 1.5)
                * Quaternion::rotation_z(tilt * -1.5);

        next.leg_br.position = Vec3::new(
            s_a.feet_b.0,
            s_a.feet_b.1 + footverttf * -1.4,
            s_a.feet_b.2 + ((footvertf * 1.5).max(-1.0)),
        );
        next.leg_br.orientation =
            Quaternion::rotation_x(-0.25 * speednorm + s_a.maximize * footverttf * -0.8)
                * Quaternion::rotation_y(tilt * 1.5)
                * Quaternion::rotation_z(tilt * -1.5);

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.tail.orientation = Quaternion::rotation_x(vertcancel * short * 0.2 + x_tilt)
            * Quaternion::rotation_y(tilt * 0.8)
            * Quaternion::rotation_z(s_a.lateral * -short * 0.2 + tilt * 1.5);
        next
    }
}
