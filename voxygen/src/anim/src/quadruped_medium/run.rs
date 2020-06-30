use super::{super::Animation, QuadrupedMediumSkeleton, SkeletonAttr};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (f32, Vec3<f32>, Vec3<f32>, f64);
    type Skeleton = QuadrupedMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_medium_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_medium_run")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, orientation, last_ori, _global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();
        *rate = 1.0;
        let lab = 0.6;

        let speedmult = if speed > 8.0 { 1.2* (1.0/skeleton_attr.scaler) } else { 1.0*(1.0/skeleton_attr.scaler) };

        let short = (((1.0)
            / (0.72
                + 0.28
                    * ((anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 1.0).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 1.0).sin());

        let foothoril = (((1.0)
            / (0.5
                + (0.5)
                    * ((anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 1.45).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 1.45).sin());
        let foothorir = (((1.0)
            / (0.2
                + (0.8)
                    * ((anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 0.45).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 0.45).sin());
        let footvertl = (anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 0.0).sin();
        let footvertr = (anim_time as f32 * 16.0 * lab as f32 * speedmult + PI).sin();

        let footrotl = (((1.0)
            / (0.5
                + (0.5)
                    * ((anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 1.4).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 1.4).sin());

        let footrotr = (((1.0)
            / (0.5
                + (0.5)
                    * ((anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 0.4).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 0.4).sin());
        //
        let foothorilb = (((5.0)
            / (1.0
                + (4.0)
                    * ((anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 1.05).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 1.05).sin());
        let foothorirb = (((1.0)
            / (0.2
                + (0.8)
                    * ((anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 0.05).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 0.05).sin());
        let footvertlb = (anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * (-0.4)).sin();
        let footvertrb = (anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 0.6).sin();

        let footrotlb = (((1.0)
            / (0.5
                + (0.5)
                    * ((anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 1.0).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 1.0).sin());

        let footrotrb = (((1.0)
            / (0.5
                + (0.5)
                    * ((anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 0.0).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 0.0).sin());

        let shortalt = (anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 0.5).sin();

        let vertchest = (anim_time as f32 * lab as f32 * speedmult + PI * 0.3)
            .sin()
            .max(0.2);
        let horichest = (anim_time as f32 * lab as f32 * speedmult + PI * 0.8).sin();

//
        let ori = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if Vec2::new(ori, last_ori)
            .map(|o| Vec2::<f32>::from(o).magnitude_squared())
            .map(|m| m > 0.001 && m.is_finite())
            .reduce_and()
            && ori.angle_between(last_ori).is_finite()
        {
            ori.angle_between(last_ori).min(0.2)
                * last_ori.determine_side(Vec2::zero(), ori).signum()
        } else {
            0.0
        } * 1.3;
//let tilt = 0.0;
        if speed < 8.0 {
            //Trot
            next.head_upper.offset =
                Vec3::new(0.0, skeleton_attr.head_upper.0, skeleton_attr.head_upper.1);
            next.head_upper.ori = Quaternion::rotation_x(0.0) * Quaternion::rotation_z(tilt *-1.2);
            next.head_upper.scale = Vec3::one();

            next.head_lower.offset = Vec3::new(
                0.0,
                skeleton_attr.head_lower.0 + horichest * 0.4,
                skeleton_attr.head_lower.1 + vertchest * -0.8,
            );
            next.head_lower.ori = Quaternion::rotation_z(tilt *-0.8);
            next.head_lower.scale = Vec3::one() * 1.02;

            next.jaw.offset = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
            next.jaw.ori = Quaternion::rotation_x(0.0);
            next.jaw.scale = Vec3::one() * 1.02;

            next.tail.offset = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
            next.tail.ori = Quaternion::rotation_x(shortalt * 0.08)* Quaternion::rotation_z(tilt *1.5);
            next.tail.scale = Vec3::one();

            next.torso_front.offset = Vec3::new(
                0.0,
                skeleton_attr.torso_front.0,
                skeleton_attr.torso_front.1 + shortalt * 0.8,
            ) *skeleton_attr.scaler/11.0;
            next.torso_front.ori = Quaternion::rotation_x(short * 0.03);
            next.torso_front.scale = Vec3::one() *skeleton_attr.scaler/11.0;

            next.torso_back.offset =
                Vec3::new(0.0, skeleton_attr.torso_back.0, skeleton_attr.torso_back.1);
            next.torso_back.ori = Quaternion::rotation_x(short * -0.03)*Quaternion::rotation_z(tilt*1.8);
            next.torso_back.scale = Vec3::one();

            next.ears.offset = Vec3::new(0.0, skeleton_attr.ears.0, skeleton_attr.ears.1);
            next.ears.ori = Quaternion::rotation_x(0.0);
            next.ears.scale = Vec3::one() * 1.02;

            ////left and right functions currently swapped on some bones to change gait
            next.leg_fl.offset = Vec3::new(
                -skeleton_attr.leg_f.0,
                skeleton_attr.leg_f.1 + foothoril * -1.0,
                skeleton_attr.leg_f.2 + footvertl * -0.4,
            );
            next.leg_fl.ori = Quaternion::rotation_x(footrotl * -0.3)*Quaternion::rotation_z(tilt * -1.5);
            next.leg_fl.scale = Vec3::one() * 0.99;

            next.leg_fr.offset = Vec3::new(
                skeleton_attr.leg_f.0,
                skeleton_attr.leg_f.1 + foothorir * -1.0,
                skeleton_attr.leg_f.2 + footvertr * -0.4,
            );
            next.leg_fr.ori = Quaternion::rotation_x(footrotr * -0.3)*Quaternion::rotation_z(tilt * -1.5);
            next.leg_fr.scale = Vec3::one() * 0.99;

            next.leg_bl.offset = Vec3::new(
                -skeleton_attr.leg_b.0,
                skeleton_attr.leg_b.1 + foothorilb * -1.0,
                skeleton_attr.leg_b.2 + footvertlb * -0.4,
            );
            next.leg_bl.ori = Quaternion::rotation_x(footrotlb * -0.3)*Quaternion::rotation_z(tilt * -1.5);
            next.leg_bl.scale = Vec3::one() * 0.99;

            next.leg_br.offset = Vec3::new(
                skeleton_attr.leg_b.0,
                skeleton_attr.leg_b.1 + foothorirb * -1.0,
                skeleton_attr.leg_b.2 + footvertrb * -0.4,
            );
            next.leg_br.ori = Quaternion::rotation_x(footrotrb * -0.3)*Quaternion::rotation_z(tilt * -1.5);
            next.leg_br.scale = Vec3::one() * 0.99;

            next.foot_fl.offset = Vec3::new(
                -skeleton_attr.feet_f.0,
                skeleton_attr.feet_f.1,
                skeleton_attr.feet_f.2 + 1.0 + ((footvertl * -1.0).max(-1.0)),
            );
            next.foot_fl.ori = Quaternion::rotation_x(footrotl * -0.3);
            next.foot_fl.scale = Vec3::one() * 0.97;

            next.foot_fr.offset = Vec3::new(
                skeleton_attr.feet_f.0,
                skeleton_attr.feet_f.1,
                skeleton_attr.feet_f.2 + 1.0 + ((footvertr * -1.0).max(-1.0)),
            );
            next.foot_fr.ori = Quaternion::rotation_x(footrotr * -0.3);
            next.foot_fr.scale = Vec3::one() * 0.98;

            next.foot_bl.offset = Vec3::new(
                -skeleton_attr.feet_b.0,
                skeleton_attr.feet_b.1,
                skeleton_attr.feet_b.2 + 1.0 + ((footvertlb * -1.0).max(-1.0)),
            );
            next.foot_bl.ori = Quaternion::rotation_x(footrotlb * -0.3);
            next.foot_bl.scale = Vec3::one() * 0.98;

            next.foot_br.offset = Vec3::new(
                skeleton_attr.feet_b.0,
                skeleton_attr.feet_b.1,
                skeleton_attr.feet_b.2 + 1.0 + ((footvertrb * -1.0).max(-1.0)),
            );
            next.foot_br.ori = Quaternion::rotation_x(footrotrb * -0.3);
            next.foot_br.scale = Vec3::one() * 0.98;
        } else {
            //Gallop
            next.head_upper.offset =
                Vec3::new(0.0, skeleton_attr.head_upper.0, skeleton_attr.head_upper.1);
            next.head_upper.ori = Quaternion::rotation_x(0.0) * Quaternion::rotation_z(tilt *-1.2);
            next.head_upper.scale = Vec3::one();

            next.head_lower.offset = Vec3::new(
                0.0,
                skeleton_attr.head_lower.0 + horichest * 0.4,
                skeleton_attr.head_lower.1 + vertchest * -0.8,
            );
            next.head_lower.ori = Quaternion::rotation_z(tilt *-0.8);
            next.head_lower.scale = Vec3::one() * 1.02;

            next.jaw.offset = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
            next.jaw.ori = Quaternion::rotation_x(0.0);
            next.jaw.scale = Vec3::one() * 1.02;

            next.tail.offset = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
            next.tail.ori = Quaternion::rotation_x(shortalt * 0.3)*Quaternion::rotation_z(tilt * 1.5);
            next.tail.scale = Vec3::one();

            next.torso_front.offset = Vec3::new(
                0.0,
                skeleton_attr.torso_front.0,
                skeleton_attr.torso_front.1 + shortalt * 2.0,
            ) *skeleton_attr.scaler/11.0;
            next.torso_front.ori = Quaternion::rotation_x(short * 0.13)*Quaternion::rotation_z(tilt * -1.5);
            next.torso_front.scale = Vec3::one() *skeleton_attr.scaler/11.0;

            next.torso_back.offset =
                Vec3::new(0.0, skeleton_attr.torso_back.0, skeleton_attr.torso_back.1);
            next.torso_back.ori = Quaternion::rotation_x(short * 0.1)*Quaternion::rotation_z(tilt*1.8);
            next.torso_back.scale = Vec3::one();

            next.ears.offset = Vec3::new(0.0, skeleton_attr.ears.0, skeleton_attr.ears.1);
            next.ears.ori = Quaternion::rotation_x(0.0);
            next.ears.scale = Vec3::one() * 1.02;

            ////left and right functions currently swapped on some bones to change gait
            next.leg_fl.offset = Vec3::new(
                -skeleton_attr.leg_f.0,
                skeleton_attr.leg_f.1 + foothoril * -2.5,
                skeleton_attr.leg_f.2 + 1.0 + footvertl * -1.0,
            );
            next.leg_fl.ori = Quaternion::rotation_x(footrotl * -0.6)*Quaternion::rotation_z(tilt * -0.5);
            next.leg_fl.scale = Vec3::one() * 0.99;

            next.leg_fr.offset = Vec3::new(
                skeleton_attr.leg_f.0,
                skeleton_attr.leg_f.1 + foothoril * -2.5,
                skeleton_attr.leg_f.2 + 1.0 + footvertl * -1.0,
            );
            next.leg_fr.ori = Quaternion::rotation_x(footrotl * -0.6)*Quaternion::rotation_z(tilt * -0.5);
            next.leg_fr.scale = Vec3::one() * 0.99;

            next.leg_bl.offset = Vec3::new(
                -skeleton_attr.leg_b.0,
                skeleton_attr.leg_b.1 + foothorirb * -2.5,
                skeleton_attr.leg_b.2 + 1.0 + footvertrb * -1.2,
            );
            next.leg_bl.ori = Quaternion::rotation_x(footrotrb * -0.6)*Quaternion::rotation_z(tilt * -1.5);
            next.leg_bl.scale = Vec3::one() * 0.99;

            next.leg_br.offset = Vec3::new(
                skeleton_attr.leg_b.0,
                skeleton_attr.leg_b.1 + foothorirb * -2.5,
                skeleton_attr.leg_b.2 + 1.0 + footvertrb * -1.2,
            );
            next.leg_br.ori = Quaternion::rotation_x(footrotrb * -0.6)*Quaternion::rotation_z(tilt * -1.5);
            next.leg_br.scale = Vec3::one() * 0.99;

            next.foot_fl.offset = Vec3::new(
                -skeleton_attr.feet_f.0,
                skeleton_attr.feet_f.1,
                skeleton_attr.feet_f.2 + 2.0 + ((footvertl * -2.0).max(-1.0)),
            );
            next.foot_fl.ori = Quaternion::rotation_x(footrotl * -0.7);
            next.foot_fl.scale = Vec3::one() * 0.97;

            next.foot_fr.offset = Vec3::new(
                skeleton_attr.feet_f.0,
                skeleton_attr.feet_f.1,
                skeleton_attr.feet_f.2 + 2.0 + ((footvertl * -2.0).max(-1.0)),
            );
            next.foot_fr.ori = Quaternion::rotation_x(footrotl * -0.7);
            next.foot_fr.scale = Vec3::one() * 0.98;

            next.foot_bl.offset = Vec3::new(
                -skeleton_attr.feet_b.0,
                skeleton_attr.feet_b.1,
                skeleton_attr.feet_b.2 + 2.0 + ((footvertrb * -2.0).max(-1.0)),
            );
            next.foot_bl.ori = Quaternion::rotation_x(footrotrb * -0.7);
            next.foot_bl.scale = Vec3::one() * 0.98;

            next.foot_br.offset = Vec3::new(
                skeleton_attr.feet_b.0,
                skeleton_attr.feet_b.1,
                skeleton_attr.feet_b.2 + 2.0 + ((footvertrb * -2.0).max(-1.0)),
            );
            next.foot_br.ori = Quaternion::rotation_x(footrotrb * -0.7);
            next.foot_br.scale = Vec3::one() * 0.98;
        }
        next
    }
}
