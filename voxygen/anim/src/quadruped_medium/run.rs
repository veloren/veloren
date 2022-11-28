use super::{
    super::{vek::*, Animation},
    QuadrupedMediumSkeleton, SkeletonAttr,
};
use core::{f32::consts::PI, ops::Mul};

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency<'a> = (f32, Vec3<f32>, Vec3<f32>, f32, Vec3<f32>, f32);
    type Skeleton = QuadrupedMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_medium_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_medium_run")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, orientation, last_ori, global_time, avg_vel, acc_vel): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = (Vec2::<f32>::from(velocity).magnitude()).min(24.0);
        *rate = 1.0;
        let lab: f32 = 0.72;
        let amplitude = (speed / 24.0).powf(0.6);
        let amplitude2 = (speed / 24.0).powf(0.6);
        let amplitude3 = (speed / 24.0).powf(0.6);
        let speedmult = s_a.tempo;
        let canceler = (speed / 24.0).powf(0.6);
        let mixed_vel = acc_vel + anim_time * 2.5; //sets run frequency using speed, with anim_time setting a floor

        let short = ((1.0
            / (0.72
                + 0.28
                    * ((mixed_vel * (1.0) * lab * speedmult + PI * -0.15 - 0.5).sin()).powi(2)))
        .sqrt())
            * ((mixed_vel * (1.0) * lab * speedmult + PI * -0.15 - 0.5).sin());

        //
        let shortalt = (mixed_vel * (1.0) * lab * speedmult + PI * 3.0 / 8.0 - 0.5).sin();
        let look = Vec2::new(
            (global_time / 2.0 + anim_time / 2.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.5,
            (global_time / 2.0 + anim_time / 2.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.25,
        );

        let speedadjust = if speed < 5.0 { 0.0 } else { speed / 24.0 };
        let shift1 = speedadjust - PI / 2.0 - speedadjust * PI * 3.0 / 4.0;
        let shift2 = speedadjust + PI / 2.0 + speedadjust * PI / 2.0;
        let shift3 = speedadjust + PI / 4.0 - speedadjust * PI / 4.0;
        let shift4 = speedadjust - PI * 3.0 / 4.0 + speedadjust * PI / 2.0;

        //FL
        let foot1a = (mixed_vel * (1.0) * lab * speedmult + 0.0 + canceler * 0.05 + shift1).sin(); //1.5
        let foot1b = (mixed_vel * (1.0) * lab * speedmult + 1.1 + canceler * 0.05 + shift1).sin(); //1.9
        //FR
        let foot2a = (mixed_vel * (1.0) * lab * speedmult + shift2).sin(); //1.0
        let foot2b = (mixed_vel * (1.0) * lab * speedmult + 1.1 + shift2).sin(); //1.0
        //BL
        let foot3a = (mixed_vel * (1.0) * lab * speedmult + shift3).sin(); //0.0
        let foot3b = (mixed_vel * (1.0) * lab * speedmult + PI / 2.0 + shift3).sin(); //0.4
        //BR
        let foot4a = (mixed_vel * (1.0) * lab * speedmult + 0.0 + canceler * 0.05 + shift4).sin(); //0.3
        let foot4b =
            (mixed_vel * (1.0) * lab * speedmult + PI / 2.0 + canceler * 0.05 + shift4).sin(); //0.7
        //
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
        let x_tilt = avg_vel.z.atan2(avg_vel.xy().magnitude()) * canceler;

        next.neck.scale = Vec3::one() * 1.02;
        next.jaw.scale = Vec3::one() * 1.02;
        next.leg_fl.scale = Vec3::one() * 1.02;
        next.leg_fr.scale = Vec3::one() * 1.02;
        next.leg_bl.scale = Vec3::one() * 1.02;
        next.leg_br.scale = Vec3::one() * 1.02;
        next.foot_fl.scale = Vec3::one() * 0.96;
        next.foot_fr.scale = Vec3::one() * 0.96;
        next.foot_bl.scale = Vec3::one() * 0.96;
        next.foot_br.scale = Vec3::one() * 0.96;
        next.ears.scale = Vec3::one() * 1.02;

        //Gallop
        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1 + shortalt * -0.2);
        next.head.orientation = Quaternion::rotation_x(
            look.y * 0.3 / ((canceler).max(0.5)) + amplitude * short * 0.05 - 0.1,
        ) * Quaternion::rotation_z(
            look.x * 0.3 / ((canceler).max(0.5)) + tilt * -1.2,
        ) * Quaternion::rotation_y(tilt * 0.8);

        next.neck.position = Vec3::new(0.0, s_a.neck.0, s_a.neck.1 + shortalt * -0.8);
        next.neck.orientation = Quaternion::rotation_z(tilt * -0.8)
            * Quaternion::rotation_x(amplitude * short * 0.05)
            * Quaternion::rotation_y(tilt * 0.3);

        next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
        next.jaw.orientation = Quaternion::rotation_x(0.0);

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.tail.orientation = Quaternion::rotation_x(amplitude * shortalt * -0.3)
            * Quaternion::rotation_z(tilt * 1.5);

        next.torso_front.position = Vec3::new(
            0.0,
            s_a.torso_front.0,
            s_a.torso_front.1
                + canceler * 1.0
                + canceler * shortalt * 2.5 * s_a.spring
                + x_tilt * 10.0 * canceler,
        );
        next.torso_front.orientation = Quaternion::rotation_x(
            ((amplitude * (short * -0.13).max(-0.2)) * s_a.spring).min(0.1)
                + x_tilt * (canceler * 6.0).min(1.0),
        ) * Quaternion::rotation_y(tilt * 0.8)
            * Quaternion::rotation_z(tilt * -1.5);

        next.torso_back.position = Vec3::new(
            0.0,
            s_a.torso_back.0,
            s_a.torso_back.1 + amplitude * shortalt * 0.2 - 0.2,
        );
        next.torso_back.orientation = Quaternion::rotation_x(amplitude * short * -0.07)
            * Quaternion::rotation_z(tilt * 1.8)
            * Quaternion::rotation_y(tilt * 0.6);

        next.ears.position = Vec3::new(0.0, s_a.ears.0, s_a.ears.1);
        next.ears.orientation = Quaternion::rotation_x(amplitude * shortalt * 0.2 + 0.2);

        next.leg_fl.position = Vec3::new(
            -s_a.leg_f.0,
            s_a.leg_f.1 + amplitude3 * foot1b * -1.6,
            s_a.leg_f.2 + amplitude3 * foot1a * 2.3,
        );
        next.leg_fl.orientation = Quaternion::rotation_x(0.2 * canceler + amplitude3 * foot1a)
            * Quaternion::rotation_z(tilt * -0.5)
            * Quaternion::rotation_y(tilt * 1.5);

        next.leg_fr.position = Vec3::new(
            s_a.leg_f.0,
            s_a.leg_f.1 + amplitude3 * foot2b * -1.6,
            s_a.leg_f.2 + amplitude3 * foot2a * 2.3,
        );
        next.leg_fr.orientation = Quaternion::rotation_x(0.2 * canceler + amplitude3 * foot2a)
            * Quaternion::rotation_z(tilt * -0.5)
            * Quaternion::rotation_y(tilt * 1.5);

        next.leg_bl.position = Vec3::new(
            -s_a.leg_b.0,
            s_a.leg_b.1 + amplitude3 * foot3b * -1.1,
            s_a.leg_b.2 + amplitude3 * foot3a * 1.1,
        );
        next.leg_bl.orientation =
            Quaternion::rotation_x(canceler * -0.2 + amplitude3 * foot3b * -0.55)
                * Quaternion::rotation_y(tilt * 1.5)
                * Quaternion::rotation_z(tilt * -1.5);

        next.leg_br.position = Vec3::new(
            s_a.leg_b.0,
            s_a.leg_b.1 + amplitude3 * foot4b * -1.1,
            s_a.leg_b.2 + amplitude3 * foot4a * 1.1,
        );
        next.leg_br.orientation =
            Quaternion::rotation_x(canceler * -0.2 + amplitude3 * foot4b * -0.55)
                * Quaternion::rotation_y(tilt * 1.5)
                * Quaternion::rotation_z(tilt * -1.5);

        next.foot_fl.position = Vec3::new(
            -s_a.feet_f.0,
            s_a.feet_f.1,
            s_a.feet_f.2 + (foot1a * 2.0).max(-1.0) * amplitude2 + 1.0 * canceler,
        );
        next.foot_fl.orientation =
            Quaternion::rotation_x(s_a.startangle * canceler + amplitude2 * foot1b * -0.85)
                * Quaternion::rotation_y(tilt * -1.0);

        next.foot_fr.position = Vec3::new(
            s_a.feet_f.0,
            s_a.feet_f.1,
            s_a.feet_f.2 + (foot2a * 2.0).max(-1.0) * amplitude2 + 1.0 * canceler,
        );
        next.foot_fr.orientation =
            Quaternion::rotation_x(s_a.startangle * canceler + amplitude2 * foot2b * -0.85)
                * Quaternion::rotation_y(tilt * -1.0);

        next.foot_bl.position = Vec3::new(
            -s_a.feet_b.0,
            s_a.feet_b.1,
            s_a.feet_b.2 + (foot3a * 0.8).max(0.0) * amplitude2 + 1.0 * canceler,
        );
        next.foot_bl.orientation =
            Quaternion::rotation_x(amplitude2 * foot3b * -0.7 - 0.2 * canceler)
                * Quaternion::rotation_y(tilt * -1.0);

        next.foot_br.position = Vec3::new(
            s_a.feet_b.0,
            s_a.feet_b.1,
            s_a.feet_b.2 + (foot4a * 0.8).max(0.0) * amplitude2 + 1.0 * canceler,
        );
        next.foot_br.orientation =
            Quaternion::rotation_x(amplitude2 * foot4b * -0.7 - 0.2 * canceler)
                * Quaternion::rotation_y(tilt * -1.0);
        next
    }
}
