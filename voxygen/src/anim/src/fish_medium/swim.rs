use super::{
    super::{vek::*, Animation},
    FishMediumSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct SwimAnimation;

type SwimAnimationDependency = (Vec3<f32>, Vec3<f32>, Vec3<f32>, f64, Vec3<f32>);

impl Animation for SwimAnimation {
    type Dependency = SwimAnimationDependency;
    type Skeleton = FishMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"fish_medium_swim\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "fish_medium_swim")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, orientation, last_ori, global_time, avg_vel): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let slower = (anim_time as f32 * 1.0 + PI).sin();
        let slow = (anim_time as f32 * 3.5 + PI).sin();
        let slowalt = (anim_time as f32 * 3.5 + PI + 0.2).sin();
        let fast = (anim_time as f32 * 5.5 + PI).sin();
        let fastalt = (anim_time as f32 * 5.5 + PI + 0.2).sin();

        let ori: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if ::vek::Vec2::new(ori, last_ori)
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
        let abstilt = tilt.abs();
        let x_tilt = avg_vel.z.atan2(avg_vel.xy().magnitude());

        let squash = if abstilt > 0.2 { 0.35 } else { 1.0 }; //condenses the body at strong turns

        next.chest_front.scale = Vec3::one() / 11.0;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_z(slowalt * -0.1 + tilt * -2.0);

        next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
        next.jaw.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);

        next.chest_front.position = Vec3::new(0.0, s_a.chest_front.0, s_a.chest_front.1) / 11.0;
        next.chest_front.orientation =
            Quaternion::rotation_x(velocity.z.abs() * -0.005 + abstilt * 1.0 + x_tilt);

        next.chest_back.position = Vec3::new(0.0, s_a.chest_back.0, s_a.chest_back.1);
        next.chest_back.orientation = Quaternion::rotation_z(fastalt * 0.3 + tilt * 2.0);

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.tail.orientation = Quaternion::rotation_z(fast * 0.3 + tilt * 2.0);

        next.fin_l.position = Vec3::new(-s_a.fin.0, s_a.fin.1, s_a.fin.2);
        next.fin_l.orientation = Quaternion::rotation_z(fast * 0.3 - 0.1 + tilt * -0.5);

        next.fin_r.position = Vec3::new(s_a.fin.0, s_a.fin.1, s_a.fin.2);
        next.fin_r.orientation = Quaternion::rotation_z(-fast * 0.3 + 0.1 + tilt * -0.5);
        next
    }
}
