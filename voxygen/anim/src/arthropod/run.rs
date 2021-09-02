use super::{super::Animation, ArthropodSkeleton, SkeletonAttr};
//use std::{f32::consts::PI, ops::Mul};
use super::super::vek::*;
use std::f32::consts::PI;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency<'a> = (Vec3<f32>, Vec3<f32>, Vec3<f32>, f32, Vec3<f32>, f32);
    type Skeleton = ArthropodSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"arthropod_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "arthropod_run")]
    fn update_skeleton_inner<'a>(
        skeleton: &Self::Skeleton,
        (velocity, orientation, last_ori, _global_time, avg_vel, acc_vel): Self::Dependency<'a>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = (Vec2::<f32>::from(velocity).magnitude()).min(22.0);
        *rate = 1.0;

        //let speednorm = speed / 13.0;
        let speednorm = (speed / 13.0).powf(0.25);
        let mixed_vel = (acc_vel + anim_time * 6.0) * 0.8; //sets run frequency using speed, with anim_time setting a floor

        let speedmult = 1.0;
        let lab: f32 = 0.6; //6

        let short = ((1.0
            / (0.72
                + 0.28 * ((mixed_vel * 1.0 * lab * speedmult + PI * -0.15 - 0.5).sin()).powi(2)))
        .sqrt())
            * ((mixed_vel * 1.0 * lab * speedmult + PI * -0.15 - 0.5).sin())
            * speednorm;

        //
        let shortalt = (mixed_vel * 1.0 * lab * speedmult + PI * 3.0 / 8.0 - 0.5).sin() * speednorm;

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
        let x_tilt = avg_vel.z.atan2(avg_vel.xy().magnitude()) * speednorm;

        next.chest.scale = Vec3::one() / s_a.scaler;
        next.wing_bl.scale = Vec3::one() * 0.98;
        next.wing_br.scale = Vec3::one() * 0.98;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_x((mixed_vel).sin() * 0.1)
            * Quaternion::rotation_y((mixed_vel).sin().min(0.0) * 0.08)
            * Quaternion::rotation_z((mixed_vel + PI * 1.5).sin() * 0.08);

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);
        next.chest.orientation = Quaternion::rotation_x((mixed_vel).sin().max(0.0) * 0.06)
            * Quaternion::rotation_z((mixed_vel + PI / 2.0).sin() * 0.06);

        next.mandible_l.position = Vec3::new(-s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);
        next.mandible_r.position = Vec3::new(s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);

        next.wing_fl.position = Vec3::new(-s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);
        next.wing_fr.position = Vec3::new(s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);

        next.wing_bl.position = Vec3::new(-s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);
        next.wing_br.position = Vec3::new(s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);

        next.leg_fl.position = Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fr.position = Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fl.orientation = Quaternion::rotation_x((mixed_vel + PI).sin().max(0.0) * 0.7)
            * Quaternion::rotation_z(s_a.leg_ori.0 + (mixed_vel - PI / 2.0).sin() * 0.4);
        next.leg_fr.orientation = Quaternion::rotation_x((mixed_vel).sin().max(0.0) * 0.7)
            * Quaternion::rotation_z(-s_a.leg_ori.0 - (mixed_vel + PI / 2.0).sin() * 0.4);

        next.leg_fcl.position = Vec3::new(-s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);
        next.leg_fcr.position = Vec3::new(s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);
        next.leg_fcl.orientation = Quaternion::rotation_x((mixed_vel + PI).sin() * 0.2)
            * Quaternion::rotation_y((mixed_vel).sin().max(0.0) * 0.7)
            * Quaternion::rotation_z(s_a.leg_ori.1 + (mixed_vel + PI / 2.0).sin() * 0.2);
        next.leg_fcr.orientation = Quaternion::rotation_x((mixed_vel).sin() * 0.2)
            * Quaternion::rotation_y((mixed_vel).sin().min(0.0) * 0.7)
            * Quaternion::rotation_z(-s_a.leg_ori.1 - (mixed_vel + PI * 1.5).sin() * 0.2);

        next.leg_bcl.position = Vec3::new(-s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);
        next.leg_bcr.position = Vec3::new(s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);
        next.leg_bcl.orientation = Quaternion::rotation_x((mixed_vel + PI).sin() * 0.2)
            * Quaternion::rotation_y((mixed_vel + PI).sin().max(0.0) * 0.7)
            * Quaternion::rotation_z(s_a.leg_ori.2 + (mixed_vel + PI * 1.5).sin() * 0.3);
        next.leg_bcr.orientation = Quaternion::rotation_x((mixed_vel).sin() * 0.2)
            * Quaternion::rotation_y((mixed_vel + PI).sin().min(0.0) * 0.7)
            * Quaternion::rotation_z(-s_a.leg_ori.2 - (mixed_vel + PI / 2.0).sin() * 0.3);

        next.leg_bl.position = Vec3::new(-s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_br.position = Vec3::new(s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_bl.orientation = Quaternion::rotation_x((mixed_vel + PI).sin() * 0.2)
            * Quaternion::rotation_y((mixed_vel).sin().max(0.0) * 0.7)
            * Quaternion::rotation_z(s_a.leg_ori.3 + (mixed_vel + PI / 2.0).sin() * 0.2);
        next.leg_br.orientation = Quaternion::rotation_x((mixed_vel).sin() * 0.2)
            * Quaternion::rotation_y((mixed_vel).sin().min(0.0) * 0.7)
            * Quaternion::rotation_z(s_a.leg_ori.3 - (mixed_vel + PI * 1.5).sin() * 0.2);

        next
    }
}
