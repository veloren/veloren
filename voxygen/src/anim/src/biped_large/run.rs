use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use std::{f32::consts::PI, ops::Mul};

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (f32, Vec3<f32>, Vec3<f32>, f64, Vec3<f32>);
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_run")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, orientation, last_ori, global_time, avg_vel): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();
        *rate = 1.0;

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

        let foothoril2 = (((1.0)
            / (0.5
                + (0.6)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.5).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.5).sin());
        let foothorir2 = (((1.0)
            / (0.5
                + (0.6)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.5).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.5).sin());

        //let short = (anim_time as f32 * lab as f32 * 16.0).sin();

        //let shortalt = (anim_time as f32 * lab as f32 * 16.0 + PI / 2.0).sin();

        let lab = 0.65; //0.72
        let amplitude = (speed / 21.0).max(0.25);
        let amplitude2 = (speed * 1.4 / 21.0).powf(0.5).max(0.6);
        let amplitude3 = (speed / 21.0).powf(0.5).max(0.35);
        let speedmult = 1.0;
        let canceler = (speed / 21.0).powf(0.5);

        let short = (anim_time as f32 * (16.0) * lab as f32 * speedmult + PI * -0.15).sin();
        //
        let shortalt =
            (anim_time as f32 * (16.0) * lab as f32 * speedmult + PI * 3.0 / 8.0 + 0.7).sin();
        let look = Vec2::new(
            ((global_time + anim_time) as f32 / 2.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.5,
            ((global_time + anim_time) as f32 / 2.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.25,
        );

        let speedadjust = if speed < 5.0 { 0.0 } else { speed / 21.0 };
        let shift1 = speedadjust - PI / 2.0 - speedadjust * PI * 3.0 / 4.0;
        let shift2 = speedadjust + PI / 2.0 + speedadjust * PI / 2.0;
        let shift3 = speedadjust + PI / 4.0 - speedadjust * PI / 4.0;
        let shift4 = speedadjust - PI * 3.0 / 4.0 + speedadjust * PI / 2.0;

        //FL
        let foot1a =
            (anim_time as f32 * (16.0) * lab as f32 * speedmult + 0.0 + canceler * 0.05 + shift1)
                .sin(); //1.5
        let foot1b =
            (anim_time as f32 * (16.0) * lab as f32 * speedmult + 1.1 + canceler * 0.05 + shift1)
                .sin(); //1.9
        //FR
        let foot2a = (anim_time as f32 * (16.0) * lab as f32 * speedmult + shift2).sin(); //1.2
        let foot2b = (anim_time as f32 * (16.0) * lab as f32 * speedmult + 1.1 + shift2).sin(); //1.6
        //BL
        let foot3a = (anim_time as f32 * (16.0) * lab as f32 * speedmult + shift3).sin(); //0.0
        let foot3b = (anim_time as f32 * (16.0) * lab as f32 * speedmult + 1.57 + shift3).sin(); //0.4
        //BR
        let foot4a =
            (anim_time as f32 * (16.0) * lab as f32 * speedmult + 0.0 + canceler * 0.05 + shift4)
                .sin(); //0.3
        let foot4b =
            (anim_time as f32 * (16.0) * lab as f32 * speedmult + 1.57 + canceler * 0.05 + shift4)
                .sin(); //0.7
        //

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

        let x_tilt = avg_vel.z.atan2(avg_vel.xy().magnitude());

        if skeleton_attr.beast {
            next.head.position = Vec3::new(0.0, skeleton_attr.head.0, 3.0 + skeleton_attr.head.1);
            next.head.orientation = Quaternion::rotation_x(
                look.y * 0.3 / ((canceler).max(0.5)) + amplitude * short * -0.18 + 0.6,
            ) * Quaternion::rotation_z(
                look.x * 0.3 / ((canceler).max(0.5)) + tilt * -1.2,
            ) * Quaternion::rotation_y(tilt * 0.8);
            next.head.scale = Vec3::one();

            next.jaw.position = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
            next.jaw.orientation = Quaternion::rotation_x(0.0);
            next.jaw.scale = Vec3::one() * 1.02;

            next.tail.position = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
            next.tail.orientation =
                Quaternion::rotation_x(canceler * 1.0 + amplitude * shortalt * 0.3)
                    * Quaternion::rotation_z(tilt * 1.5);
            next.tail.scale = Vec3::one();

            next.upper_torso.position = Vec3::new(
                0.0,
                skeleton_attr.upper_torso.0,
                skeleton_attr.upper_torso.1, //xtilt
            );
            next.upper_torso.orientation = Quaternion::rotation_x(
                (amplitude * (short * -0.0).max(-0.2)) + 0.0 * (canceler * 6.0).min(1.0), //x_tilt
            ) * Quaternion::rotation_y(tilt * 0.8)
                * Quaternion::rotation_z(tilt * -1.5);
            next.upper_torso.scale = Vec3::one();

            next.lower_torso.position = Vec3::new(
                0.0,
                skeleton_attr.lower_torso.0,
                skeleton_attr.lower_torso.1,
            );
            next.lower_torso.orientation =
                Quaternion::rotation_x(amplitude * short * -0.25 + canceler * -0.4)
                    * Quaternion::rotation_z(tilt * 1.8)
                    * Quaternion::rotation_y(tilt * 0.6);
            next.lower_torso.scale = Vec3::one();

            next.arm_control_l.position = Vec3::new(
                0.0,
                0.0 + amplitude3 * foot1b * -1.5 + canceler * -2.0,
                0.0 + amplitude3 * foot1a * 1.5,
            );
            next.arm_control_l.orientation =
                Quaternion::rotation_x(0.3 * canceler + amplitude3 * foot1a * 0.4)
                    * Quaternion::rotation_z(tilt * -0.5)
                    * Quaternion::rotation_y(tilt * 1.5);
            next.arm_control_l.scale = Vec3::one() * 1.02;

            next.arm_control_r.position = Vec3::new(
                0.0,
                0.0 + amplitude3 * foot2b * -1.5 + canceler * -2.0,
                0.0 + amplitude3 * foot2a * 1.5,
            );
            next.arm_control_r.orientation =
                Quaternion::rotation_x(0.3 * canceler + amplitude3 * foot2a * 0.4)
                    * Quaternion::rotation_z(tilt * -0.5)
                    * Quaternion::rotation_y(tilt * 1.5);
            next.arm_control_r.scale = Vec3::one() * 1.02;

            next.shoulder_l.position = Vec3::new(
                -skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_l.scale = Vec3::one() * 1.02;

            next.shoulder_r.position = Vec3::new(
                skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_r.scale = Vec3::one() * 1.02;

            next.hand_l.position = Vec3::new(
                -skeleton_attr.hand.0,
                skeleton_attr.hand.1 + foot1a * 1.0,
                skeleton_attr.hand.2 + (foot1a * -3.0).max(1.0) * amplitude2,
            );
            next.hand_l.orientation =
                Quaternion::rotation_x((amplitude2 * foot1b * 0.9 + canceler * 0.9).max(0.5))
                    * Quaternion::rotation_y(tilt * -1.0);
            next.hand_l.scale = Vec3::one() * 0.96;

            next.hand_r.position = Vec3::new(
                skeleton_attr.hand.0,
                skeleton_attr.hand.1 + foot2a * 1.0,
                skeleton_attr.hand.2 + (foot2a * -3.0).max(1.0) * amplitude2,
            );
            next.hand_r.orientation =
                Quaternion::rotation_x((amplitude2 * foot2b * 0.9 + canceler * 0.7).max(0.5))
                    * Quaternion::rotation_y(tilt * -1.0);
            next.hand_r.scale = Vec3::one() * 0.96;

            next.leg_control_l.position = Vec3::new(
                0.0,
                0.0 + amplitude3 * foot3b * -1.0 + canceler * -2.0,
                0.0 + amplitude3 * foot3a * -2.5,
            );
            next.leg_control_l.orientation =
                Quaternion::rotation_x(canceler * -0.4 + amplitude3 * foot3b * 0.3)
                    * Quaternion::rotation_y(tilt * 1.5)
                    * Quaternion::rotation_z(tilt * -1.5);
            next.leg_control_l.scale = Vec3::one() * 1.02;

            next.leg_control_r.position = Vec3::new(
                0.0,
                0.0 + amplitude3 * foot4b * -1.0 + canceler * -2.0,
                0.0 + amplitude3 * foot4a * -2.5,
            );
            next.leg_control_r.orientation =
                Quaternion::rotation_x(canceler * -0.4 + amplitude3 * foot4b * 0.3)
                    * Quaternion::rotation_y(tilt * 1.5)
                    * Quaternion::rotation_z(tilt * -1.5);
            next.leg_control_r.scale = Vec3::one() * 1.02;

            next.leg_l.position = Vec3::new(
                -skeleton_attr.leg.0,
                skeleton_attr.leg.1,
                skeleton_attr.leg.2,
            );
            next.leg_l.scale = Vec3::one() * 1.0;

            next.leg_r.position = Vec3::new(
                skeleton_attr.leg.0,
                skeleton_attr.leg.1,
                skeleton_attr.leg.2,
            );
            next.leg_r.scale = Vec3::one() * 1.0;

            next.foot_l.position = Vec3::new(
                -skeleton_attr.foot.0,
                skeleton_attr.foot.1 + foot3a * 2.0,
                skeleton_attr.foot.2 + (foot3b * 4.0 * amplitude2).max(-1.0),
            );
            next.foot_l.orientation = Quaternion::rotation_x(amplitude2 * foot3b * 0.45 + 0.5)
                * Quaternion::rotation_y(tilt * -1.0);
            next.foot_l.scale = Vec3::one() * 0.96;

            next.foot_r.position = Vec3::new(
                skeleton_attr.foot.0,
                skeleton_attr.foot.1 + foot4a * 2.0,
                skeleton_attr.foot.2 + (foot4b * 4.0 * amplitude2).max(-1.0),
            );
            next.foot_r.orientation = Quaternion::rotation_x(amplitude2 * foot4b * 0.45 + 0.5)
                * Quaternion::rotation_y(tilt * -1.0);
            next.foot_r.scale = Vec3::one() * 0.96;

            next.torso.position = Vec3::new(
                0.0,
                0.0 + (short * 0.75).max(-2.0),
                canceler * 2.0 + (short * 0.75).max(-2.0),
            ) / 8.0;
            next.torso.orientation =
                Quaternion::rotation_x(x_tilt + amplitude * short * 0.1 + canceler * -0.7);
            next.torso.scale = Vec3::one() / 8.0;

            next.hold.scale = Vec3::one() * 0.0;
        } else {
            next.head.position = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1) * 1.02;
            next.head.orientation =
                Quaternion::rotation_z(short * -0.18) * Quaternion::rotation_x(-0.05);
            next.head.scale = Vec3::one() * 1.02;

            next.upper_torso.position = Vec3::new(
                0.0,
                skeleton_attr.upper_torso.0,
                skeleton_attr.upper_torso.1 + shortalt * -1.5,
            );
            next.upper_torso.orientation = Quaternion::rotation_z(short * 0.18);
            next.upper_torso.scale = Vec3::one();

            next.lower_torso.position = Vec3::new(
                0.0,
                skeleton_attr.lower_torso.0,
                skeleton_attr.lower_torso.1,
            );
            next.lower_torso.orientation =
                Quaternion::rotation_z(short * 0.15) * Quaternion::rotation_x(0.14);
            next.lower_torso.scale = Vec3::one() * 1.02;

            next.jaw.position = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
            next.jaw.orientation = Quaternion::rotation_x(0.0);
            next.jaw.scale = Vec3::one() * 1.02;

            next.tail.position = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
            next.tail.orientation = Quaternion::rotation_x(shortalt * 0.3);
            next.tail.scale = Vec3::one();

            next.second.position = Vec3::new(0.0, 0.0, 0.0);
            next.second.orientation = Quaternion::rotation_x(PI)
                * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(0.0);
            next.second.scale = Vec3::one() * 0.0;

            next.control.position = Vec3::new(0.0, 0.0, 0.0);
            next.control.orientation = Quaternion::rotation_z(0.0);
            next.control.scale = Vec3::one();

            next.main.position = Vec3::new(-5.0, -7.0, 7.0);
            next.main.orientation = Quaternion::rotation_x(PI)
                * Quaternion::rotation_y(0.6)
                * Quaternion::rotation_z(1.57);
            next.main.scale = Vec3::one() * 1.02;

            next.shoulder_l.position = Vec3::new(
                -skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1 + foothoril * -3.0,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_l.orientation = Quaternion::rotation_x(footrotl * -0.36)
                * Quaternion::rotation_y(0.1)
                * Quaternion::rotation_z(footrotl * 0.3);
            next.shoulder_l.scale = Vec3::one();

            next.shoulder_r.position = Vec3::new(
                skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1 + foothorir * -3.0,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_r.orientation = Quaternion::rotation_x(footrotr * -0.36)
                * Quaternion::rotation_y(-0.1)
                * Quaternion::rotation_z(footrotr * -0.3);
            next.shoulder_r.scale = Vec3::one();

            next.hand_l.position = Vec3::new(
                -1.0 + -skeleton_attr.hand.0,
                skeleton_attr.hand.1 + foothoril * -4.0,
                skeleton_attr.hand.2 + foothoril * 1.0,
            );
            next.hand_l.orientation = Quaternion::rotation_x(0.15 + (handhoril * -1.2).max(-0.3))
                * Quaternion::rotation_y(handhoril * -0.1);
            next.hand_l.scale = Vec3::one() * 1.02;

            next.hand_r.position = Vec3::new(
                1.0 + skeleton_attr.hand.0,
                skeleton_attr.hand.1 + foothorir * -4.0,
                skeleton_attr.hand.2 + foothorir * 1.0,
            );
            next.hand_r.orientation = Quaternion::rotation_x(0.15 + (handhorir * -1.2).max(-0.3))
                * Quaternion::rotation_y(handhorir * 0.1);
            next.hand_r.scale = Vec3::one() * 1.02;

            next.leg_l.position = Vec3::new(
                -skeleton_attr.leg.0,
                skeleton_attr.leg.1,
                skeleton_attr.leg.2,
            ) * 0.98;
            next.leg_l.orientation =
                Quaternion::rotation_z(short * 0.18) * Quaternion::rotation_x(foothoril * 0.3);
            next.leg_l.scale = Vec3::one() * 0.98;

            next.leg_r.position = Vec3::new(
                skeleton_attr.leg.0,
                skeleton_attr.leg.1,
                skeleton_attr.leg.2,
            ) * 0.98;

            next.leg_r.orientation =
                Quaternion::rotation_z(short * 0.18) * Quaternion::rotation_x(foothorir * 0.3);
            next.leg_r.scale = Vec3::one() * 0.98;

            next.foot_l.position = Vec3::new(
                -skeleton_attr.foot.0,
                skeleton_attr.foot.1 + foothoril * 8.5,
                skeleton_attr.foot.2 + ((footvertl * 6.5).max(0.0)),
            );
            next.foot_l.orientation =
                Quaternion::rotation_x(-0.5 + footrotl * 0.85) * Quaternion::rotation_y(0.0);
            next.foot_l.scale = Vec3::one();

            next.foot_r.position = Vec3::new(
                skeleton_attr.foot.0,
                skeleton_attr.foot.1 + foothorir * 8.5,
                skeleton_attr.foot.2 + ((footvertr * 6.5).max(0.0)),
            );
            next.foot_r.orientation =
                Quaternion::rotation_x(-0.5 + footrotr * 0.85) * Quaternion::rotation_y(0.0);
            next.foot_r.scale = Vec3::one();

            next.torso.position = Vec3::new(0.0, 0.0, 0.0) / 8.0;
            next.torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.25);
            next.torso.scale = Vec3::one() / 8.0;

            next.leg_control_l.scale = Vec3::one() * 1.0;
            next.leg_control_r.scale = Vec3::one() * 1.0;
            next.arm_control_l.scale = Vec3::one() * 1.0;
            next.arm_control_r.scale = Vec3::one() * 1.0;

            next.hold.scale = Vec3::one() * 0.0;
        }
        next
    }
}
