use super::{
    super::{vek::*, Animation},
    BirdLargeSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (Vec3<f32>, Vec3<f32>, Vec3<f32>, f32, Vec3<f32>, f32);
    type Skeleton = BirdLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"bird_large_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "bird_large_run")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, orientation, last_ori, _global_time, avg_vel, acc_vel): Self::Dependency,
        _anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = (Vec2::<f32>::from(velocity).magnitude()).min(22.0);
        *rate = 1.0;

        //let speednorm = speed / 13.0;
        let speednorm = (speed / 13.0).powf(0.25);

        let speedmult = 2.0;
        let lab: f32 = 0.6; //6

        let short = ((1.0
            / (0.72
                + 0.28 * ((acc_vel * 1.0 * lab * speedmult + PI * -0.15 - 0.5).sin()).powi(2)))
        .sqrt())
            * ((acc_vel * 1.0 * lab * speedmult + PI * -0.15 - 0.5).sin())
            * speednorm;

        //
        let shortalt = (acc_vel * 1.0 * lab * speedmult + PI * 3.0 / 8.0 - 0.5).sin() * speednorm;

        //FL
        let foot1a = (acc_vel * 1.0 * lab * speedmult + 0.0 + PI).sin() * speednorm; //1.5
        let foot1b = (acc_vel * 1.0 * lab * speedmult + 1.57 + PI).sin() * speednorm; //1.9
        //FR
        let foot2a = (acc_vel * 1.0 * lab * speedmult).sin() * speednorm; //1.2
        let foot2b = (acc_vel * 1.0 * lab * speedmult + 1.57).sin() * speednorm; //1.6
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

        next.head.scale = Vec3::one() * 0.98;
        next.neck.scale = Vec3::one() * 1.02;
        next.beak.scale = Vec3::one() * 0.98;
        next.leg_l.scale = Vec3::one() * 0.98;
        next.leg_r.scale = Vec3::one() * 0.98;    
        next.foot_l.scale = Vec3::one() * 0.98;
        next.foot_r.scale = Vec3::one() * 0.98;
        next.chest.scale = Vec3::one() * s_a.scaler / 4.0;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_x(-0.1 * speednorm + short * -0.05)
            * Quaternion::rotation_y(tilt * 0.2)
            * Quaternion::rotation_z(shortalt * -0.05 - tilt * 1.5);

        next.beak.position = Vec3::new(0.0, s_a.beak.0, s_a.beak.1);
        next.beak.orientation = Quaternion::rotation_x(short * -0.02 - 0.02);

        next.neck.position = Vec3::new(0.0, s_a.neck.0, s_a.neck.1);
        next.neck.orientation = Quaternion::rotation_x(-0.1 * speednorm + short * -0.04)
            * Quaternion::rotation_y(tilt * 0.1)
            * Quaternion::rotation_z(shortalt * -0.1 - tilt * 0.5);

        next.chest.position = Vec3::new(
            0.0,
            s_a.chest.0,
            s_a.chest.1 + short * 0.5 + 0.5 * speednorm,
        ) * s_a.scaler
            / 4.0;
        next.chest.orientation = Quaternion::rotation_x(short * 0.07)
            * Quaternion::rotation_y(tilt * 0.8)
            * Quaternion::rotation_z(shortalt * 0.10);

        next.tail_front.position = Vec3::new(0.0, s_a.tail_front.0, s_a.tail_front.1);
        next.tail_front.orientation = Quaternion::rotation_x(short * -0.02);

        next.tail_rear.position = Vec3::new(0.0, s_a.tail_rear.0, s_a.tail_rear.1);
        next.tail_rear.orientation = Quaternion::rotation_x(short * -0.1);

        next.wing_in_l.position = Vec3::new(-s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);
        next.wing_in_r.position = Vec3::new(s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);

        next.wing_in_l.orientation = Quaternion::rotation_y(-s_a.wings_angle) * Quaternion::rotation_z(0.2);
        next.wing_in_r.orientation = Quaternion::rotation_y(s_a.wings_angle) * Quaternion::rotation_z(-0.2);

        next.wing_mid_l.position = Vec3::new(-s_a.wing_mid.0, s_a.wing_mid.1, s_a.wing_mid.2);
        next.wing_mid_r.position = Vec3::new(s_a.wing_mid.0, s_a.wing_mid.1, s_a.wing_mid.2);
        next.wing_mid_l.orientation = Quaternion::rotation_y(-0.1) * Quaternion::rotation_z(0.7);
        next.wing_mid_r.orientation = Quaternion::rotation_y(0.1) * Quaternion::rotation_z(-0.7);

        next.wing_out_l.position = Vec3::new(-s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
        next.wing_out_r.position = Vec3::new(s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
        next.wing_out_l.orientation =
            Quaternion::rotation_y(-0.2 + short * 0.05) * Quaternion::rotation_z(0.2);
        next.wing_out_r.orientation =
            Quaternion::rotation_y(0.2 + short * -0.05) * Quaternion::rotation_z(-0.2);

        next.leg_l.position = Vec3::new(
            -s_a.leg.0 + speednorm * 1.5,
            s_a.leg.1 + foot1b * -2.3,
            s_a.leg.2,
        );
        next.leg_l.orientation = Quaternion::rotation_x(-0.2 * speednorm + foot1a * 0.15)
            * Quaternion::rotation_y(tilt * 0.5);

        next.leg_r.position = Vec3::new(
            s_a.leg.0 + speednorm * -1.5,
            s_a.leg.1 + foot2b * -2.3,
            s_a.leg.2,
        );
        next.leg_r.orientation = Quaternion::rotation_x(-0.2 * speednorm + foot2a * 0.15)
            * Quaternion::rotation_y(tilt * 0.5);

        next.foot_l.position = Vec3::new(
            -s_a.foot.0,
            s_a.foot.1 + foot1b * -1.0,
            s_a.foot.2 + (foot1a * 1.5).max(0.0),
        );
        next.foot_l.orientation = Quaternion::rotation_x(0.2 * speednorm + foot1b * -0.5 + 0.1)
            * Quaternion::rotation_y(tilt * -1.0)
            * Quaternion::rotation_z(tilt * -0.5);

        next.foot_r.position = Vec3::new(
            s_a.foot.0,
            s_a.foot.1 + foot2b * -1.0,
            s_a.foot.2 + (foot2a * 1.5).max(0.0),
        );
        next.foot_r.orientation = Quaternion::rotation_x(0.2 * speednorm + foot2b * -0.5 + 0.1)
            * Quaternion::rotation_y(tilt * -1.0)
            * Quaternion::rotation_z(tilt * -0.5);

        next
    }
}
