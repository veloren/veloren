use super::{
    super::{vek::*, Animation},
    BipedSmallSkeleton, SkeletonAttr,
};
use core::f32::consts::PI;

pub struct IdleAnimation;

type IdleAnimationDependency = (Vec3<f32>, Vec3<f32>, Vec3<f32>, f32, Vec3<f32>);

impl Animation for IdleAnimation {
    type Dependency<'a> = IdleAnimationDependency;
    type Skeleton = BipedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_small_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_small_idle")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, _orientation, _last_ori, _global_time, _avg_vel): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let slow = (anim_time * 4.0).sin();

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1 + slow * -0.1);

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + slow * 0.3);
        next.pants.position = Vec3::new(0.0, s_a.pants.0, s_a.pants.1);
        next.main.position = Vec3::new(2.0, -3.0, -3.0);
        next.main.orientation = Quaternion::rotation_y(-0.5) * Quaternion::rotation_z(PI / 2.0);

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2 + slow * -0.1);
        next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2 + slow * -0.1);
        next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2);
        next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2);

        next
    }
}
