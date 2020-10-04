use super::{super::Animation, SkeletonAttr, TheropodSkeleton};
//use std::{f32::consts::PI, ops::Mul};
use super::super::vek::*;
use std::f32::consts::PI;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (f32, f64);
    type Skeleton = TheropodSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"theropod_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "theropod_run")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, _global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();
        *rate = 1.0;
        //let wave = (anim_time as f32 * 8.0).sin();
        //let wavealt = (anim_time as f32 * 8.0 + PI / 2.0).sin();
        //let wave_slow = (anim_time as f32 * 6.5 + PI).sin();

        let breathe = (anim_time as f32 * 0.8).sin();
        let topspeed = 18.0;

        let canceler = speed / topspeed;
        let lab = 0.5; //6
        let amplitude2 = (speed * 1.4 / topspeed).max(0.6);
        let amplitude3 = (speed / topspeed).max(0.35);
        let speedmult = if speed > 0.0 {
            1.2 * (1.0 * 1.0)
        } else {
            0.9 * (1.0 * 1.0)
        };

        let short = (((1.0)
            / (0.72
                + 0.28
                    * ((anim_time as f32 * (16.0) * lab as f32 * speedmult + PI * -0.15 - 0.5)
                        .sin())
                    .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * (16.0) * lab as f32 * speedmult + PI * -0.15 - 0.5).sin());

        //
        let shortalt =
            (anim_time as f32 * (16.0) * lab as f32 * speedmult + PI * 3.0 / 8.0 - 0.5).sin();

        //FL
        let foot1a = (anim_time as f32 * (16.0) * lab as f32 * speedmult + 0.0 + PI).sin(); //1.5
        let foot1b = (anim_time as f32 * (16.0) * lab as f32 * speedmult + 1.57 + PI).sin(); //1.9
        //FR
        let foot2a = (anim_time as f32 * (16.0) * lab as f32 * speedmult).sin(); //1.2
        let foot2b = (anim_time as f32 * (16.0) * lab as f32 * speedmult + 1.57).sin(); //1.6
        //BL
        //BR

        next.head.position = Vec3::new(
            0.0,
            skeleton_attr.head.0,
            skeleton_attr.head.1 + breathe * 0.3,
        );
        next.head.orientation =
            Quaternion::rotation_x(-0.1 + short * -0.05) * Quaternion::rotation_z(shortalt * -0.2);
        next.head.scale = Vec3::one() * 1.02;

        next.jaw.position = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
        next.jaw.orientation = Quaternion::rotation_x(short * -0.03);
        next.jaw.scale = Vec3::one() * 0.98;

        next.neck.position = Vec3::new(0.0, skeleton_attr.neck.0, skeleton_attr.neck.1);
        next.neck.orientation =
            Quaternion::rotation_x(-0.1 + short * -0.04) * Quaternion::rotation_z(shortalt * -0.1);
        next.neck.scale = Vec3::one() * 0.98;

        next.chest_front.position = Vec3::new(
            0.0,
            skeleton_attr.chest_front.0,
            skeleton_attr.chest_front.1 + short * 0.5,
        ) / skeleton_attr.scaler;
        next.chest_front.orientation =
            Quaternion::rotation_x(short * 0.07) * Quaternion::rotation_z(shortalt * 0.15);
        next.chest_front.scale = Vec3::one() / skeleton_attr.scaler;

        next.chest_back.position =
            Vec3::new(0.0, skeleton_attr.chest_back.0, skeleton_attr.chest_back.1);
        next.chest_back.orientation =
            Quaternion::rotation_x(short * -0.04) * Quaternion::rotation_z(shortalt * -0.15);
        next.chest_back.scale = Vec3::one();

        next.tail_front.position =
            Vec3::new(0.0, skeleton_attr.tail_front.0, skeleton_attr.tail_front.1);
        next.tail_front.orientation =
            Quaternion::rotation_x(0.1 + short * -0.02) * Quaternion::rotation_z(shortalt * -0.1);
        next.tail_front.scale = Vec3::one();

        next.tail_back.position =
            Vec3::new(0.0, skeleton_attr.tail_back.0, skeleton_attr.tail_back.1);
        next.tail_back.orientation =
            Quaternion::rotation_x(0.2 + short * -0.2) * Quaternion::rotation_z(shortalt * -0.2);
        next.tail_back.scale = Vec3::one();

        next.hand_l.position = Vec3::new(
            -skeleton_attr.hand.0,
            skeleton_attr.hand.1,
            skeleton_attr.hand.2,
        );
        next.hand_l.orientation = Quaternion::rotation_x(-0.2 + amplitude3 * foot2a * 0.3);
        next.hand_l.scale = Vec3::one();

        next.hand_r.position = Vec3::new(
            skeleton_attr.hand.0,
            skeleton_attr.hand.1,
            skeleton_attr.hand.2,
        );
        next.hand_r.orientation = Quaternion::rotation_x(-0.2 + amplitude3 * foot1a * 0.3);
        next.hand_r.scale = Vec3::one();

        next.leg_l.position = Vec3::new(
            -skeleton_attr.leg.0,
            skeleton_attr.leg.1 + amplitude3 * foot1b * -1.3,
            skeleton_attr.leg.2 + amplitude3 * foot1a * 1.4,
        );
        next.leg_l.orientation = Quaternion::rotation_x(-0.2 + amplitude3 * foot1a * 0.2)
            * Quaternion::rotation_z(foot1a * -0.3)
            * Quaternion::rotation_y(0.0);
        next.leg_l.scale = Vec3::one() * 1.0;

        next.leg_r.position = Vec3::new(
            skeleton_attr.leg.0,
            skeleton_attr.leg.1 + amplitude3 * foot2b * -1.3,
            skeleton_attr.leg.2 + amplitude3 * foot2a * 1.4,
        );
        next.leg_r.orientation = Quaternion::rotation_x(-0.2 + amplitude3 * foot2a * 0.2)
            * Quaternion::rotation_z(foot2a * 0.3)
            * Quaternion::rotation_y(0.0);
        next.leg_r.scale = Vec3::one() * 1.0;

        next.foot_l.position = Vec3::new(
            -skeleton_attr.foot.0,
            skeleton_attr.foot.1 + canceler * -2.0 + amplitude3 * foot1b * -2.0,
            skeleton_attr.foot.2 + canceler * 2.0 + (foot1a * 2.0).max(0.0) * amplitude2,
        );
        next.foot_l.orientation = Quaternion::rotation_x(-0.3 + amplitude2 * foot1b * -0.35)
            * Quaternion::rotation_y(0.0);
        next.foot_l.scale = Vec3::one() * 0.96;

        next.foot_r.position = Vec3::new(
            skeleton_attr.foot.0,
            skeleton_attr.foot.1 + canceler * -2.0 + amplitude3 * foot2b * -2.0,
            skeleton_attr.foot.2 + canceler * 2.0 + (foot2a * 2.0).max(0.0) * amplitude2,
        );
        next.foot_r.orientation = Quaternion::rotation_x(-0.3 + amplitude2 * foot2b * -0.35)
            * Quaternion::rotation_y(0.0);
        next.foot_r.scale = Vec3::one() * 0.96;

        next
    }
}
