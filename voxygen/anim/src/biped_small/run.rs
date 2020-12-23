use super::{
    super::{vek::*, Animation},
    BipedSmallSkeleton, SkeletonAttr,
};

pub struct RunAnimation;

type RunAnimationDependency = (Vec3<f32>, Vec3<f32>, Vec3<f32>, f64, Vec3<f32>);

impl Animation for RunAnimation {
    type Dependency = RunAnimationDependency;
    type Skeleton = BipedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_small_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "Biped_small_run")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, _orientation, _last_ori, _global_time, _avg_vel): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        next.head.scale = Vec3::one();

        next.chest.scale = Vec3::one() / 13.0;

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1) / 13.0;
        next.shorts.position = Vec3::new(0.0, s_a.shorts.0, s_a.shorts.1);
        next.main.position = Vec3::new(0.0, 0.0, 0.0);

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.hand_l.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2);
        next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2);
        next.foot_l.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2);
        next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2);

        next
    }
}
