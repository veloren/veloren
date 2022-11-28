use super::{
    super::{vek::*, Animation},
    FishMediumSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct IdleAnimation;

type IdleAnimationDependency = (Vec3<f32>, Vec3<f32>, Vec3<f32>, f32, Vec3<f32>);

impl Animation for IdleAnimation {
    type Dependency<'a> = IdleAnimationDependency;
    type Skeleton = FishMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"fish_medium_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "fish_medium_idle")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, _orientation, _last_ori, _global_time, _avg_vel): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let slow = (anim_time * 3.5 + PI).sin();
        let slowalt = (anim_time * 3.5 + PI + 0.2).sin();

        next.jaw.scale = Vec3::one() * 0.98;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_z(slowalt * -0.1);

        next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);

        next.chest_front.position = Vec3::new(0.0, s_a.chest_front.0, s_a.chest_front.1);
        next.chest_front.orientation = Quaternion::rotation_x(0.0);

        next.chest_back.position = Vec3::new(0.0, s_a.chest_back.0, s_a.chest_back.1);
        next.chest_back.orientation = Quaternion::rotation_z(slowalt * 0.1);

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.tail.orientation = Quaternion::rotation_z(slow * 0.1);

        next.fin_l.position = Vec3::new(-s_a.fin.0, s_a.fin.1, s_a.fin.2);
        next.fin_l.orientation = Quaternion::rotation_z(slow * 0.1 - 0.1);

        next.fin_r.position = Vec3::new(s_a.fin.0, s_a.fin.1, s_a.fin.2);
        next.fin_r.orientation = Quaternion::rotation_z(-slow * 0.1 + 0.1);
        next
    }
}
