use super::{
    super::{vek::*, Animation},
    QuadrupedMediumSkeleton, SkeletonAttr,
};
use std::{f32::consts::PI, ops::Mul};

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (f32, Vec3<f32>, Vec3<f32>, f64, Vec3<f32>);
    type Skeleton = QuadrupedMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_medium_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_medium_run")]
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
        //let increasefreqtest = (((1.0/speed)*3.0).round()).min(5.0);
        let lab = 0.6; //6
        let amplitude = (speed / 24.0).max(0.25);
        let amplitude2 = (speed * 1.4 / 24.0).max(0.6);
        let amplitude3 = (speed / 24.0).max(0.35);
        let speedmult = if speed > 0.0 {
            1.2 * (1.0 * skeleton_attr.tempo)
        } else {
            0.9 * (1.0 * skeleton_attr.tempo)
        };
        let canceler = speed / 24.0;
        let short = (((1.0)
            / (0.72
                + 0.28
                    * ((anim_time as f32 * (16.0) * lab as f32 * speedmult + PI * -0.15).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * (16.0) * lab as f32 * speedmult + PI * -0.15).sin());

        //
        let shortalt = (anim_time as f32 * (16.0) * lab as f32 * speedmult + PI * 0.35).sin();
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

        let speedadjust = if speed < 5.0 { 0.0 } else { speed };
        let shift1 = speedadjust / 24.0 * 0.0;
        let shift2 = speedadjust / 24.0 * PI / -4.0;
        let shift3 = speedadjust / 24.0 * PI / 2.0;
        let shift4 = speedadjust / 24.0 * PI / 4.0;

        //FL
        let foot1a = (anim_time as f32 * (16.0) * lab as f32 * speedmult
            + PI * (0.0 + canceler * 0.08 + shift1))
            .sin(); //1.5
        let foot1b = (anim_time as f32 * (16.0) * lab as f32 * speedmult
            + PI * (0.4 + canceler * 0.08 + shift1))
            .sin(); //1.9

        //FR
        let foot2a =
            (anim_time as f32 * (16.0) * lab as f32 * speedmult + PI * (PI / 4.0 + shift2)).sin(); //1.2
        let foot2b = (anim_time as f32 * (16.0) * lab as f32 * speedmult
            + PI * (0.4 + PI / 4.0 + shift2))
            .sin(); //1.6

        //BL
        let foot3a =
            (anim_time as f32 * (16.0) * lab as f32 * speedmult + PI * (0.0 + shift3)).sin(); //0.0
        let foot3b =
            (anim_time as f32 * (16.0) * lab as f32 * speedmult + PI * (0.4 + shift3)).sin(); //0.4

        //BR
        let foot4a = (anim_time as f32 * (16.0) * lab as f32 * speedmult
            + PI * (0.0 + canceler * 0.15 + PI / 4.0 + shift4))
            .sin(); //0.3
        let foot4b = (anim_time as f32 * (16.0) * lab as f32 * speedmult
            + PI * (0.4 + canceler * 0.15 + PI / 4.0 + shift4))
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

        //Gallop
        next.head_upper.position =
            Vec3::new(0.0, skeleton_attr.head_upper.0, skeleton_attr.head_upper.1);
        next.head_upper.orientation =
            Quaternion::rotation_x(look.y * 0.1 / canceler + amplitude * short * -0.03 - 0.1)
                * Quaternion::rotation_z(look.x * 0.1 / canceler + tilt * -1.2)
                * Quaternion::rotation_y(tilt * 0.8);
        next.head_upper.scale = Vec3::one();

        next.head_lower.position =
            Vec3::new(0.0, skeleton_attr.head_lower.0, skeleton_attr.head_lower.1);
        next.head_lower.orientation = Quaternion::rotation_z(tilt * -0.8)
            * Quaternion::rotation_x(amplitude * short * -0.05)
            * Quaternion::rotation_y(tilt * 0.3);
        next.head_lower.scale = Vec3::one() * 1.02;

        next.jaw.position = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
        next.jaw.orientation = Quaternion::rotation_x(0.0);
        next.jaw.scale = Vec3::one() * 1.02;

        next.tail.position = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
        next.tail.orientation =
            Quaternion::rotation_x(shortalt * 0.3) * Quaternion::rotation_z(tilt * 1.5);
        next.tail.scale = Vec3::one();

        next.torso_front.position = Vec3::new(
            0.0,
            skeleton_attr.torso_front.0,
            skeleton_attr.torso_front.1
                + canceler * 1.0
                + canceler * shortalt * 2.5
                + x_tilt * 10.0,
        ) * skeleton_attr.scaler
            / 11.0;
        next.torso_front.orientation =
            Quaternion::rotation_x((amplitude * (short * -0.13).max(-0.2)) + x_tilt)
                * Quaternion::rotation_y(tilt * 0.8)
                * Quaternion::rotation_z(tilt * -1.5);
        next.torso_front.scale = Vec3::one() * skeleton_attr.scaler / 11.0;

        next.torso_back.position = Vec3::new(
            0.0,
            skeleton_attr.torso_back.0,
            skeleton_attr.torso_back.1 + amplitude * shortalt * 0.2 - 0.2,
        );
        next.torso_back.orientation = Quaternion::rotation_x(amplitude * short * -0.1)
            * Quaternion::rotation_z(tilt * 1.8)
            * Quaternion::rotation_y(tilt * 0.6);
        next.torso_back.scale = Vec3::one();

        next.ears.position = Vec3::new(0.0, skeleton_attr.ears.0, skeleton_attr.ears.1);
        next.ears.orientation = Quaternion::rotation_x(amplitude * shortalt * 0.2 + 0.2);
        next.ears.scale = Vec3::one() * 1.02;

        next.leg_fl.position = Vec3::new(
            -skeleton_attr.leg_f.0,
            skeleton_attr.leg_f.1 + amplitude3 * foot1a * -1.3,
            skeleton_attr.leg_f.2 + canceler + amplitude3 * foot1b * -1.9,
        );
        next.leg_fl.orientation = Quaternion::rotation_x(amplitude3 * foot1b * -0.55)
            * Quaternion::rotation_z(tilt * -0.5)
            * Quaternion::rotation_y(tilt * 1.5);
        next.leg_fl.scale = Vec3::one() * 1.02;

        next.leg_fr.position = Vec3::new(
            skeleton_attr.leg_f.0,
            skeleton_attr.leg_f.1 + amplitude3 * foot2a * -1.3,
            skeleton_attr.leg_f.2 + canceler + amplitude3 * foot2b * -1.9,
        );
        next.leg_fr.orientation = Quaternion::rotation_x(amplitude3 * foot2b * -0.55)
            * Quaternion::rotation_z(tilt * -0.5)
            * Quaternion::rotation_y(tilt * 1.5);
        next.leg_fr.scale = Vec3::one() * 1.02;

        next.leg_bl.position = Vec3::new(
            -skeleton_attr.leg_b.0,
            skeleton_attr.leg_b.1 + amplitude3 * foot3a * -1.9,
            skeleton_attr.leg_b.2 + canceler + amplitude3 * ((foot3b * -2.6).min(1.0)),
        );
        next.leg_bl.orientation =
            Quaternion::rotation_x(amplitude3 * foot3b * -0.5 - 0.2 * canceler)
                * Quaternion::rotation_y(tilt * 1.5)
                * Quaternion::rotation_z(tilt * -1.5);
        next.leg_bl.scale = Vec3::one() * 1.02;

        next.leg_br.position = Vec3::new(
            skeleton_attr.leg_b.0,
            skeleton_attr.leg_b.1 + amplitude3 * foot4a * -1.9,
            skeleton_attr.leg_b.2 + canceler + amplitude3 * ((foot4b * -2.6).min(1.0)),
        );
        next.leg_br.orientation =
            Quaternion::rotation_x(amplitude3 * foot4b * -0.5 - 0.2 * canceler)
                * Quaternion::rotation_y(tilt * 1.5)
                * Quaternion::rotation_z(tilt * -1.5);
        next.leg_br.scale = Vec3::one() * 1.02;

        next.foot_fl.position = Vec3::new(
            -skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2
                + ((amplitude2 * foot1a * 2.7
                    + amplitude2 * foot1a * skeleton_attr.maximize * canceler)
                    .max(0.0)),
        );
        next.foot_fl.orientation = Quaternion::rotation_x(
            ((1.0 - skeleton_attr.dampen) * -1.0) * canceler + amplitude2 * foot1b * -0.7,
        ) * Quaternion::rotation_y(tilt * -1.0);
        next.foot_fl.scale = Vec3::one() * 0.96;

        next.foot_fr.position = Vec3::new(
            skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2
                + ((amplitude2 * foot2a * 2.7
                    + amplitude2 * foot2a * skeleton_attr.maximize * canceler)
                    .max(0.0)),
        );
        next.foot_fr.orientation = Quaternion::rotation_x(
            ((1.0 - skeleton_attr.dampen) * -1.0) * canceler + amplitude2 * foot2b * -0.7,
        ) * Quaternion::rotation_y(tilt * -1.0);
        next.foot_fr.scale = Vec3::one() * 0.96;

        next.foot_bl.position = Vec3::new(
            -skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2 + ((amplitude2 * foot3a * 2.5).max(-1.0)),
        );
        next.foot_bl.orientation =
            Quaternion::rotation_x(amplitude2 * foot3b * -0.5 - 0.2 * canceler)
                * Quaternion::rotation_y(tilt * -1.0);
        next.foot_bl.scale = Vec3::one() * 0.96;

        next.foot_br.position = Vec3::new(
            skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2 + ((amplitude2 * foot4a * 2.5).max(-1.0)),
        );
        next.foot_br.orientation =
            Quaternion::rotation_x(amplitude2 * foot4b * -0.5 - 0.2 * canceler)
                * Quaternion::rotation_y(tilt * -1.0);
        next.foot_br.scale = Vec3::one() * 0.96;
        next
    }
}
