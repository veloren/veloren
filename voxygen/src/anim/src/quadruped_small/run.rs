use super::{super::Animation, QuadrupedSmallSkeleton, SkeletonAttr};
use std::f32::consts::PI;
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (f32, Vec3<f32>, Vec3<f32>, f64, Vec3<f32>);
    type Skeleton = QuadrupedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_small_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_small_run")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, orientation, last_ori, _global_time, avg_vel): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();

        let slow = (anim_time as f32 * 14.0).sin();
        let fast = (anim_time as f32 * 20.0).sin();
        let fast_alt = (anim_time as f32 * 20.0 + PI / 2.0).sin();
        let slow_alt = (anim_time as f32 * 14.0 + PI / 2.0).sin();
        let x_tilt = avg_vel.z.atan2(avg_vel.xy().magnitude()).max(-0.7);


        let lab = 0.6; //6

        let speedmult = if speed > 8.0 {
            1.2 * (1.0)
        } else {
            0.8 * (1.0)
        };
        let abssin = (((anim_time as f32 * 5.0 * speedmult + PI * 1.6).sin().abs()) - 0.2).abs().max(-0.2);
        let abssint = (((anim_time as f32 * 5.0 * speedmult).sin().abs()) - 0.2).abs().max(-0.2);
        let shortalt = (anim_time as f32 * 16.0 * lab as f32 * speedmult + PI * 0.5).sin();

        let footvert = (anim_time as f32 * 20.0 * lab as f32 * speedmult + PI * 0.0).sin();
        let footvertt = (anim_time as f32 * 20.0 * lab as f32 * speedmult + PI * 0.4).sin();
        let footvertalt = (anim_time as f32 * 20.0 * lab as f32 * speedmult + PI * 1.2).sin();
        let footverttalt = (anim_time as f32 * 20.0 * lab as f32 * speedmult + PI * 1.6).sin();

        let footvertf = (anim_time as f32 * 20.0 * lab as f32 * speedmult + PI * 0.3).sin();
        let footverttf = (anim_time as f32 * 20.0 * lab as f32 * speedmult + PI * 0.7).sin();
        let footvertaltf = (anim_time as f32 * 20.0 * lab as f32 * speedmult + PI * 1.5).sin();
        let footverttaltf = (anim_time as f32 * 20.0 * lab as f32 * speedmult + PI * 1.9).sin();

        let footvertfslow = (anim_time as f32 * 20.0 * lab as f32 * speedmult + PI * 0.6).sin();
        let footverttfslow = (anim_time as f32 * 20.0 * lab as f32 * speedmult + PI * 1.0).sin();
        let footvertaltfslow = (anim_time as f32 * 20.0 * lab as f32 * speedmult + PI * 1.8).sin();
        let footverttaltfslow = (anim_time as f32 * 20.0 * lab as f32 * speedmult + PI * 2.2).sin();

        let ori: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if Vec2::new(ori, last_ori)
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

        next.head.offset =
            Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1 + abssint * 2.0);
        next.head.ori = Quaternion::rotation_x(0.2 + slow * 0.05  + x_tilt * -0.5)
            * Quaternion::rotation_y(tilt * 0.8)
            * Quaternion::rotation_z(tilt * -1.2);
        next.head.scale = Vec3::one();

        next.chest.offset = Vec3::new(
            0.0,
            skeleton_attr.chest.0,
            skeleton_attr.chest.1 + abssin * 8.0 + x_tilt * 6.0,
        ) / 11.0;
        next.chest.ori = Quaternion::rotation_x(abssin * 0.5 - 0.2 + x_tilt)
        * Quaternion::rotation_y(tilt * 0.8)
        * Quaternion::rotation_z(tilt * -1.5);
        next.chest.scale = Vec3::one() / 11.0;

        next.leg_fl.offset = Vec3::new(
            -skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1 + footvertaltf * -1.3,
            skeleton_attr.feet_f.2 + 1.0 + footverttaltf * -1.9,
        );
        next.leg_fl.ori = Quaternion::rotation_x(footverttaltf * -0.65)
            * Quaternion::rotation_z(tilt * -0.5)
            * Quaternion::rotation_y(tilt * 1.5);
        next.leg_fl.scale = Vec3::one() * 1.02;

        next.leg_fr.offset = Vec3::new(
            skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1 + footvertaltf * -1.3,
            skeleton_attr.feet_f.2 + 1.0 + footvertaltf * -1.9,
        );
        next.leg_fr.ori = Quaternion::rotation_x(footvertaltf * -0.65)
            * Quaternion::rotation_z(tilt * -0.5)
            * Quaternion::rotation_y(tilt * 1.5);
        next.leg_fr.scale = Vec3::one() * 1.02;

        next.leg_bl.offset = Vec3::new(
            -skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1 + footvert * -1.7,
            skeleton_attr.feet_b.2 + 1.0 + footvertt * -1.5,
        );
        next.leg_bl.ori = Quaternion::rotation_x(footvertt * -0.45 - 0.2)
            * Quaternion::rotation_y(tilt * 1.5)
            * Quaternion::rotation_z(tilt * -1.5);
        next.leg_bl.scale = Vec3::one() * 1.02;

        next.leg_br.offset = Vec3::new(
            skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1 + footvertt * -1.7,
            skeleton_attr.feet_b.2 + 1.0 + footvertt * -1.5,
        );
        next.leg_br.ori = Quaternion::rotation_x(footvertt * -0.45 - 0.2)
            * Quaternion::rotation_y(tilt * 1.5)
            * Quaternion::rotation_z(tilt * -1.5);
        next.leg_br.scale = Vec3::one() * 1.02;

        next.tail.offset = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
        next.tail.ori = Quaternion::rotation_z(0.0);
        next.tail.scale = Vec3::one();
        next
    }
}
