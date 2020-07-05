use super::{super::Animation, QuadrupedMediumSkeleton, SkeletonAttr};
use std::f32::consts::PI;
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (f32, Vec3<f32>, Vec3<f32>, f64, Vec3<f32>);
    type Skeleton = QuadrupedMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_medium_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_medium_run")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, orientation, last_ori, _global_time, avg_vel): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();
        *rate = 1.0;
        let lab = 0.6; //6

        let speedmult = if speed > 8.0 {
            1.2 * (1.0 * skeleton_attr.tempo)
        } else {
            0.9 * (1.0 * skeleton_attr.tempo)
        };

        let short = (((1.0)
            / (0.72
                + 0.28
                    * ((anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 1.0).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 1.0).sin());

        //

        let shortalt = (anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 0.5).sin();

        let footvert = (anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 0.0).sin();
        let footvertt = (anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 0.4).sin();
        let footvertalt = (anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 1.2).sin();
        let footverttalt = (anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 1.6).sin();

        let footvertf = (anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 0.3).sin();
        let footverttf = (anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 0.7).sin();
        let footvertaltf = (anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 1.5).sin();
        let footverttaltf = (anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 1.9).sin();

        let footvertfslow = (anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 0.6).sin();
        let footverttfslow = (anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 1.0).sin();
        let footvertaltfslow = (anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 1.8).sin();
        let footverttaltfslow = (anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 2.2).sin();
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
        let x_tilt = avg_vel.z.atan2(avg_vel.xy().magnitude());
        //let tilt = 0.0;
        if speed < 8.0 {
            //Trot
            next.head_upper.offset =
                Vec3::new(0.0, skeleton_attr.head_upper.0, skeleton_attr.head_upper.1);
            next.head_upper.ori =
                Quaternion::rotation_x(short * -0.03 - 0.1) * Quaternion::rotation_z(tilt * -1.2);
            next.head_upper.scale = Vec3::one();

            next.head_lower.offset =
                Vec3::new(0.0, skeleton_attr.head_lower.0, skeleton_attr.head_lower.1);
            next.head_lower.ori =
                Quaternion::rotation_z(tilt * -0.8) * Quaternion::rotation_x(short * -0.05);
            next.head_lower.scale = Vec3::one() * 1.02;

            next.jaw.offset = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
            next.jaw.ori = Quaternion::rotation_x(0.0);
            next.jaw.scale = Vec3::one() * 1.02;

            next.tail.offset = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
            next.tail.ori =
                Quaternion::rotation_x(shortalt * 0.3) * Quaternion::rotation_z(tilt * 1.5);
            next.tail.scale = Vec3::one();

            next.torso_front.offset = Vec3::new(
                0.0,
                skeleton_attr.torso_front.0,
                skeleton_attr.torso_front.1 + shortalt * 1.0 + x_tilt,
            ) * skeleton_attr.scaler
                / 11.0;
            next.torso_front.ori = Quaternion::rotation_x(short * 0.03 + x_tilt)
                * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(tilt * -1.5);
            next.torso_front.scale = Vec3::one() * skeleton_attr.scaler / 11.0;

            next.torso_back.offset = Vec3::new(
                0.0,
                skeleton_attr.torso_back.0,
                skeleton_attr.torso_back.1 + shortalt * 0.04 - 0.2,
            );
            next.torso_back.ori =
                Quaternion::rotation_x(short * 0.06) * Quaternion::rotation_z(tilt * 1.8);
            next.torso_back.scale = Vec3::one();

            next.ears.offset = Vec3::new(0.0, skeleton_attr.ears.0, skeleton_attr.ears.1);
            next.ears.ori = Quaternion::rotation_x(shortalt * 0.04 + 0.2);
            next.ears.scale = Vec3::one() * 1.02;

            next.leg_fl.offset = Vec3::new(
                -skeleton_attr.leg_f.0,
                skeleton_attr.leg_f.1 + footvertaltfslow * -1.4,
                skeleton_attr.leg_f.2 + 1.0 + footverttaltfslow * -0.3,
            );
            next.leg_fl.ori = Quaternion::rotation_x(footverttaltfslow * -0.35)
                * Quaternion::rotation_z(tilt * -0.5);
            next.leg_fl.scale = Vec3::one() * 1.02;

            next.leg_fr.offset = Vec3::new(
                skeleton_attr.leg_f.0,
                skeleton_attr.leg_f.1 + footvertalt * -1.4,
                skeleton_attr.leg_f.2 + 1.0 + footverttalt * -0.3,
            );
            next.leg_fr.ori =
                Quaternion::rotation_x(footverttalt * -0.35) * Quaternion::rotation_z(tilt * -0.5);
            next.leg_fr.scale = Vec3::one() * 1.02;

            next.leg_bl.offset = Vec3::new(
                -skeleton_attr.leg_b.0,
                skeleton_attr.leg_b.1 + footvertalt * -1.0,
                skeleton_attr.leg_b.2 + 1.0 + footverttalt * -0.3,
            );
            next.leg_bl.ori =
                Quaternion::rotation_x(footverttalt * -0.2) * Quaternion::rotation_z(tilt * -1.5);
            next.leg_bl.scale = Vec3::one() * 1.02;

            next.leg_br.offset = Vec3::new(
                skeleton_attr.leg_b.0,
                skeleton_attr.leg_b.1 + footvertaltfslow * -1.0,
                skeleton_attr.leg_b.2 + 1.0 + footverttaltfslow * -0.3,
            );
            next.leg_br.ori = Quaternion::rotation_x(footverttaltfslow * -0.2)
                * Quaternion::rotation_z(tilt * -1.5);
            next.leg_br.scale = Vec3::one() * 1.02;

            next.foot_fl.offset = Vec3::new(
                -skeleton_attr.feet_f.0,
                skeleton_attr.feet_f.1,
                skeleton_attr.feet_f.2 + ((footvertfslow * -1.0 * skeleton_attr.maximize).max(0.0)),
            );
            next.foot_fl.ori =
                Quaternion::rotation_x((1.0 - skeleton_attr.dampen) * -1.0 + footverttfslow * 0.5);
            next.foot_fl.scale = Vec3::one() * 0.96;

            next.foot_fr.offset = Vec3::new(
                skeleton_attr.feet_f.0,
                skeleton_attr.feet_f.1,
                skeleton_attr.feet_f.2 + ((footvert * -1.0 * skeleton_attr.maximize).max(0.0)),
            );
            next.foot_fr.ori =
                Quaternion::rotation_x((1.0 - skeleton_attr.dampen) * -1.0 + footvertt * 0.5);
            next.foot_fr.scale = Vec3::one() * 0.96;

            next.foot_bl.offset = Vec3::new(
                -skeleton_attr.feet_b.0,
                skeleton_attr.feet_b.1,
                skeleton_attr.feet_b.2 + ((footvert * -1.8).max(0.0)),
            );
            next.foot_bl.ori = Quaternion::rotation_x(footvertt * 0.5 - 0.2);
            next.foot_bl.scale = Vec3::one() * 0.96;

            next.foot_br.offset = Vec3::new(
                skeleton_attr.feet_b.0,
                skeleton_attr.feet_b.1,
                skeleton_attr.feet_b.2 + ((footvertfslow * -0.8).max(-0.0)),
            );
            next.foot_br.ori = Quaternion::rotation_x(footverttfslow * 0.5 - 0.2);
            next.foot_br.scale = Vec3::one() * 0.96;
        } else {
            let x_tilt = avg_vel.z.atan2(avg_vel.xy().magnitude());

            //Gallop
            next.head_upper.offset =
                Vec3::new(0.0, skeleton_attr.head_upper.0, skeleton_attr.head_upper.1);
            next.head_upper.ori = Quaternion::rotation_x(short * -0.03 - 0.1)
                * Quaternion::rotation_z(tilt * -1.2)
                * Quaternion::rotation_y(tilt * 0.8);
            next.head_upper.scale = Vec3::one();

            next.head_lower.offset =
                Vec3::new(0.0, skeleton_attr.head_lower.0, skeleton_attr.head_lower.1);
            next.head_lower.ori = Quaternion::rotation_z(tilt * -0.8)
                * Quaternion::rotation_x(short * -0.05)
                * Quaternion::rotation_y(tilt * 0.3);
            next.head_lower.scale = Vec3::one() * 1.02;

            next.jaw.offset = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
            next.jaw.ori = Quaternion::rotation_x(0.0);
            next.jaw.scale = Vec3::one() * 1.02;

            next.tail.offset = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
            next.tail.ori =
                Quaternion::rotation_x(shortalt * 0.3) * Quaternion::rotation_z(tilt * 1.5);
            next.tail.scale = Vec3::one();

            next.torso_front.offset = Vec3::new(
                0.0,
                skeleton_attr.torso_front.0,
                skeleton_attr.torso_front.1 + shortalt * 2.5 + x_tilt * 10.0,
            ) * skeleton_attr.scaler
                / 11.0;
            next.torso_front.ori = Quaternion::rotation_x(short * 0.13 + x_tilt)
                * Quaternion::rotation_y(tilt * 0.8)
                * Quaternion::rotation_z(tilt * -1.5);
            next.torso_front.scale = Vec3::one() * skeleton_attr.scaler / 11.0;

            next.torso_back.offset = Vec3::new(
                0.0,
                skeleton_attr.torso_back.0,
                skeleton_attr.torso_back.1 + shortalt * 0.2 - 0.2,
            );
            next.torso_back.ori = Quaternion::rotation_x(short * 0.1)
                * Quaternion::rotation_z(tilt * 1.8)
                * Quaternion::rotation_y(tilt * 0.6);
            next.torso_back.scale = Vec3::one();

            next.ears.offset = Vec3::new(0.0, skeleton_attr.ears.0, skeleton_attr.ears.1);
            next.ears.ori = Quaternion::rotation_x(shortalt * 0.2 + 0.2);
            next.ears.scale = Vec3::one() * 1.02;

            next.leg_fl.offset = Vec3::new(
                -skeleton_attr.leg_f.0,
                skeleton_attr.leg_f.1 + footvertaltf * -1.3,
                skeleton_attr.leg_f.2 + 1.0 + footverttaltf * -1.9,
            );
            next.leg_fl.ori = Quaternion::rotation_x(footverttaltf * -0.65)
                * Quaternion::rotation_z(tilt * -0.5)
                * Quaternion::rotation_y(tilt * 1.5);
            next.leg_fl.scale = Vec3::one() * 1.02;

            next.leg_fr.offset = Vec3::new(
                skeleton_attr.leg_f.0,
                skeleton_attr.leg_f.1 + footvertalt * -1.3,
                skeleton_attr.leg_f.2 + 1.0 + footverttalt * -1.9,
            );
            next.leg_fr.ori = Quaternion::rotation_x(footverttalt * -0.65)
                * Quaternion::rotation_z(tilt * -0.5)
                * Quaternion::rotation_y(tilt * 1.5);
            next.leg_fr.scale = Vec3::one() * 1.02;

            next.leg_bl.offset = Vec3::new(
                -skeleton_attr.leg_b.0,
                skeleton_attr.leg_b.1 + footvert * -1.7,
                skeleton_attr.leg_b.2 + 1.0 + footvertt * -1.5,
            );
            next.leg_bl.ori = Quaternion::rotation_x(footvertt * -0.45 - 0.2)
                * Quaternion::rotation_y(tilt * 1.5)
                * Quaternion::rotation_z(tilt * -1.5);
            next.leg_bl.scale = Vec3::one() * 1.02;

            next.leg_br.offset = Vec3::new(
                skeleton_attr.leg_b.0,
                skeleton_attr.leg_b.1 + footvertf * -1.7,
                skeleton_attr.leg_b.2 + 1.0 + footverttf * -1.5,
            );
            next.leg_br.ori = Quaternion::rotation_x(footverttf * -0.45 - 0.2)
                * Quaternion::rotation_y(tilt * 1.5)
                * Quaternion::rotation_z(tilt * -1.5);
            next.leg_br.scale = Vec3::one() * 1.02;

            next.foot_fl.offset = Vec3::new(
                -skeleton_attr.feet_f.0,
                skeleton_attr.feet_f.1,
                skeleton_attr.feet_f.2 + ((footvertf * -2.7 * skeleton_attr.maximize).max(0.0)),
            );
            next.foot_fl.ori =
                Quaternion::rotation_x((1.0 - skeleton_attr.dampen) * -1.0 + footverttf * 0.9)
                    * Quaternion::rotation_y(tilt * -1.0);
            next.foot_fl.scale = Vec3::one() * 0.96;

            next.foot_fr.offset = Vec3::new(
                skeleton_attr.feet_f.0,
                skeleton_attr.feet_f.1,
                skeleton_attr.feet_f.2 + ((footvert * -2.7 * skeleton_attr.maximize).max(0.0)),
            );
            next.foot_fr.ori =
                Quaternion::rotation_x((1.0 - skeleton_attr.dampen) * -1.0 + footvertt * 0.9)
                    * Quaternion::rotation_y(tilt * -1.0);
            next.foot_fr.scale = Vec3::one() * 0.96;

            next.foot_bl.offset = Vec3::new(
                -skeleton_attr.feet_b.0,
                skeleton_attr.feet_b.1,
                skeleton_attr.feet_b.2 + ((footvert * 1.3).max(0.0)),
            );
            next.foot_bl.ori = Quaternion::rotation_x(footvertt * -0.9 - 0.2)
                * Quaternion::rotation_y(tilt * -1.0);
            next.foot_bl.scale = Vec3::one() * 0.96;

            next.foot_br.offset = Vec3::new(
                skeleton_attr.feet_b.0,
                skeleton_attr.feet_b.1,
                skeleton_attr.feet_b.2 + ((footvertf * 1.3).max(-0.0)),
            );
            next.foot_br.ori = Quaternion::rotation_x(footverttf * -0.9 - 0.2)
                * Quaternion::rotation_y(tilt * -1.0);
            next.foot_br.scale = Vec3::one() * 0.96;
        }
        next
    }
}
