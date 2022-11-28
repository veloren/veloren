use super::{
    super::{vek::*, Animation},
    FishMediumSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct SwimAnimation;

type SwimAnimationDependency = (Vec3<f32>, Vec3<f32>, Vec3<f32>, f32, Vec3<f32>, f32);

impl Animation for SwimAnimation {
    type Dependency<'a> = SwimAnimationDependency;
    type Skeleton = FishMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"fish_medium_swim\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "fish_medium_swim")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, orientation, last_ori, _global_time, avg_vel, acc_vel): Self::Dependency<'_>,
        _anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let slowalt = (acc_vel * s_a.tempo / 1.5 + PI + 0.2).sin();
        let fast = (acc_vel * s_a.tempo + PI).sin();
        let fastalt = (acc_vel * s_a.tempo + PI + 0.2).sin();

        let ori: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if vek::Vec2::new(ori, last_ori)
            .map(|o| o.magnitude_squared())
            .map(|m| m > 0.001 && m.is_finite())
            .reduce_and()
            && ori.angle_between(last_ori).is_finite()
        {
            ori.angle_between(last_ori).min(0.8)
                * last_ori.determine_side(Vec2::zero(), ori).signum()
        } else {
            0.0
        } * 1.3;
        let x_tilt = avg_vel.z.atan2(avg_vel.xy().magnitude());
        let vel = (velocity.magnitude()).min(s_a.amplitude);
        let slowvel = vel * 0.1;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_z(slowalt * -0.1 + tilt * -2.0);

        next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);

        next.chest_front.position = Vec3::new(0.0, s_a.chest_front.0, s_a.chest_front.1);
        next.chest_front.orientation = Quaternion::rotation_x(velocity.z.abs() * -0.005 + x_tilt)
            * Quaternion::rotation_z(fast * vel * -0.02);

        next.chest_back.position = Vec3::new(0.0, s_a.chest_back.0, s_a.chest_back.1);
        next.chest_back.orientation = Quaternion::rotation_z(fastalt * slowvel + tilt * 2.0);

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.tail.orientation = Quaternion::rotation_z(fast * slowvel + tilt * 2.0);

        next.fin_l.position = Vec3::new(-s_a.fin.0, s_a.fin.1, s_a.fin.2);
        next.fin_l.orientation = Quaternion::rotation_z(fast * slowvel - 0.1 + tilt * -0.5);

        next.fin_r.position = Vec3::new(s_a.fin.0, s_a.fin.1, s_a.fin.2);
        next.fin_r.orientation = Quaternion::rotation_z(fast * -slowvel + 0.1 + tilt * -0.5);
        next
    }
}
