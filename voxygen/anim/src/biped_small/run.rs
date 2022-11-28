use super::{
    super::{vek::*, Animation},
    BipedSmallSkeleton, SkeletonAttr,
};
use core::{f32::consts::PI, ops::Mul};

pub struct RunAnimation;

type RunAnimationDependency = (Vec3<f32>, Vec3<f32>, Vec3<f32>, f32, Vec3<f32>, f32);

impl Animation for RunAnimation {
    type Dependency<'a> = RunAnimationDependency;
    type Skeleton = BipedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_small_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_small_run")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, orientation, last_ori, global_time, _avg_vel, acc_vel): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();
        *rate = 1.0;
        let speednorm = (speed / 9.4).powf(0.4);

        let lab: f32 = 1.0;

        let footrotl = ((5.0 / (0.5 + (5.5) * ((acc_vel * 1.4 * lab + PI * 1.4).sin()).powi(2)))
            .sqrt())
            * ((acc_vel * 1.4 * lab + PI * 1.4).sin());

        let footrotr = ((5.0 / (0.5 + (5.5) * ((acc_vel * 1.4 * lab + PI * 0.4).sin()).powi(2)))
            .sqrt())
            * ((acc_vel * 1.4 * lab + PI * 0.4).sin());

        let shortalter = (acc_vel * lab * 1.4 + PI / -2.0).sin();

        let foothoril = (acc_vel * 1.4 * lab + PI * 1.45).sin();
        let foothorir = (acc_vel * 1.4 * lab + PI * (0.45)).sin();
        let footstrafel = (acc_vel * 1.4 * lab + PI * 1.45).sin();
        let footstrafer = (acc_vel * 1.4 * lab + PI * (0.95)).sin();

        let footvertl = (acc_vel * 1.4 * lab).sin();
        let footvertr = (acc_vel * 1.4 * lab + PI).sin();
        let footvertsl = (acc_vel * 1.4 * lab).sin();
        let footvertsr = (acc_vel * 1.4 * lab + PI * 0.5).sin();

        let shortalt = (acc_vel * lab * 1.4 + PI / 2.0).sin();

        let short = ((5.0 / (1.5 + 3.5 * ((acc_vel * lab * 1.4).sin()).powi(2))).sqrt())
            * ((acc_vel * lab * 1.4).sin());
        let direction = velocity.y * -0.098 * orientation.y + velocity.x * -0.098 * orientation.x;

        let side =
            (velocity.x * -0.098 * orientation.y + velocity.y * 0.098 * orientation.x) * -1.0;
        let sideabs = side.abs();
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

        let head_look = Vec2::new(
            (global_time + anim_time / 18.0).floor().mul(7331.0).sin() * 0.2,
            (global_time + anim_time / 18.0).floor().mul(1137.0).sin() * 0.1,
        );
        next.head.position = Vec3::new(0.0, -1.0 + s_a.head.0, s_a.head.1 + short * 0.1);
        next.head.orientation =
            Quaternion::rotation_z(tilt * -2.5 + head_look.x * 0.2 - short * 0.02)
                * Quaternion::rotation_x(head_look.y + 0.45 * speednorm);

        next.chest.position = Vec3::new(
            0.0,
            s_a.chest.0,
            s_a.chest.1 + 1.0 * speednorm + shortalt * -0.8,
        );
        next.chest.orientation = Quaternion::rotation_z(short * 0.06 + tilt * -0.6)
            * Quaternion::rotation_y(tilt * 1.6)
            * Quaternion::rotation_x(shortalter * 0.035 + speednorm * -0.4 + (tilt.abs()));
        next.main.position = Vec3::new(2.0, -3.0, -3.0);
        next.main.orientation = Quaternion::rotation_y(-0.5) * Quaternion::rotation_z(PI / 2.0);

        next.pants.position = Vec3::new(0.0, s_a.pants.0, s_a.pants.1);
        next.pants.orientation = Quaternion::rotation_x(0.1 * speednorm)
            * Quaternion::rotation_z(short * 0.25 + tilt * -1.5)
            * Quaternion::rotation_y(tilt * 0.7);

        next.hand_l.position = Vec3::new(
            -s_a.hand.0 + footrotr * -1.3 * speednorm,
            1.0 * speednorm + s_a.hand.1 + footrotr * -2.5 * speednorm,
            s_a.hand.2 - footrotr * 1.5 * speednorm,
        );
        next.hand_l.orientation =
            Quaternion::rotation_x(0.4 * speednorm + (footrotr * -1.2) * speednorm)
                * Quaternion::rotation_y(footrotr * 0.4 * speednorm);

        next.hand_r.position = Vec3::new(
            s_a.hand.0 + footrotl * 1.3 * speednorm,
            1.0 * speednorm + s_a.hand.1 + footrotl * -2.5 * speednorm,
            s_a.hand.2 - footrotl * 1.5 * speednorm,
        );
        next.hand_r.orientation =
            Quaternion::rotation_x(0.4 * speednorm + (footrotl * -1.2) * speednorm)
                * Quaternion::rotation_y(footrotl * -0.4 * speednorm);

        next.foot_l.position = Vec3::new(
            -s_a.foot.0 + footstrafel * sideabs * 3.0 + tilt * -2.0,
            s_a.foot.1
                + (1.0 - sideabs) * (-1.0 * speednorm + footrotl * -2.5 * speednorm)
                + (direction * 5.0).max(0.0),
            s_a.foot.2
                + (1.0 - sideabs) * (2.0 * speednorm + ((footvertl * -1.1 * speednorm).max(-1.0)))
                + side * ((footvertsl * 1.5).max(-1.0)),
        );
        next.foot_l.orientation = Quaternion::rotation_x(
            (1.0 - sideabs) * (-0.2 * speednorm + foothoril * -0.9 * speednorm) + sideabs * -0.5,
        ) * Quaternion::rotation_y(
            tilt * 2.0 + side * 0.3 + side * (foothoril * 0.3),
        ) * Quaternion::rotation_z(side * 0.2);

        next.foot_r.position = Vec3::new(
            s_a.foot.0 + footstrafer * sideabs * 3.0 + tilt * -2.0,
            s_a.foot.1
                + (1.0 - sideabs) * (-1.0 * speednorm + footrotr * -2.5 * speednorm)
                + (direction * 5.0).max(0.0),
            s_a.foot.2
                + (1.0 - sideabs) * (2.0 * speednorm + ((footvertr * -1.1 * speednorm).max(-1.0)))
                + side * ((footvertsr * -1.5).max(-1.0)),
        );
        next.foot_r.orientation = Quaternion::rotation_x(
            (1.0 - sideabs) * (-0.2 * speednorm + foothorir * -0.9 * speednorm) + sideabs * -0.5,
        ) * Quaternion::rotation_y(
            tilt * 2.0 + side * 0.3 + side * (foothorir * 0.3),
        ) * Quaternion::rotation_z(side * 0.2);

        next.head.scale = Vec3::one();

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);

        next
    }
}
