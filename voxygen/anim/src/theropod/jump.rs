use super::{super::Animation, SkeletonAttr, TheropodSkeleton};
//use std::f32::consts::PI;
use super::super::vek::*;

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Dependency<'a> = (f32, Vec3<f32>, Vec3<f32>, f32, Vec3<f32>);
    type Skeleton = TheropodSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"theropod_jump\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "theropod_jump")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, _orientation, _last_ori, _global_time, _avg_vel): Self::Dependency<'_>,
        _anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        next.head.scale = Vec3::one() * 1.02;
        next.neck.scale = Vec3::one() * 0.98;
        next.jaw.scale = Vec3::one() * 0.98;
        next.foot_l.scale = Vec3::one() * 0.96;
        next.foot_r.scale = Vec3::one() * 0.96;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_x(-0.1);

        next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);

        next.neck.position = Vec3::new(0.0, s_a.neck.0, s_a.neck.1);
        next.neck.orientation = Quaternion::rotation_x(-0.1);

        next.chest_front.position = Vec3::new(0.0, s_a.chest_front.0, s_a.chest_front.1);

        next.chest_back.position = Vec3::new(0.0, s_a.chest_back.0, s_a.chest_back.1);

        next.tail_front.position = Vec3::new(0.0, s_a.tail_front.0, s_a.tail_front.1);
        next.tail_front.orientation = Quaternion::rotation_x(0.1);

        next.tail_back.position = Vec3::new(0.0, s_a.tail_back.0, s_a.tail_back.1);
        next.tail_back.orientation = Quaternion::rotation_x(0.1);

        next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2);

        next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2);

        next.leg_l.position = Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2);

        next.leg_r.position = Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2);

        next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2);

        next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2);

        next
    }
}
