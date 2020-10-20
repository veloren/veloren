use super::{super::Animation, GolemSkeleton, SkeletonAttr};
//use std::f32::consts::PI;
use super::super::vek::*;

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Dependency = (f32, f64);
    type Skeleton = GolemSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"golem_jump\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "golem_jump")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        _anim_time: f64,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1) * 1.02;
        next.head.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.head.scale = Vec3::one() * 1.02;

        next.upper_torso.position = Vec3::new(0.0, s_a.upper_torso.0, s_a.upper_torso.1) / 8.0;
        next.upper_torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.upper_torso.scale = Vec3::one() / 8.0;

        next.shoulder_l.position = Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.shoulder_l.scale = Vec3::one();

        next.shoulder_r.position = Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.shoulder_r.scale = Vec3::one();

        next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2);
        next.hand_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.hand_l.scale = Vec3::one() * 1.02;

        next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2);
        next.hand_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.hand_r.scale = Vec3::one() * 1.02;

        next.leg_l.position = Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2) * 1.02;
        next.leg_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.leg_l.scale = Vec3::one() * 1.02;

        next.leg_r.position = Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2) * 1.02;
        next.leg_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.leg_r.scale = Vec3::one() * 1.02;

        next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2) / 8.0;
        next.foot_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.foot_l.scale = Vec3::one() / 8.0;

        next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2) / 8.0;
        next.foot_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.foot_r.scale = Vec3::one() / 8.0;

        next.torso.position = Vec3::new(0.0, 0.0, 0.0);
        next.torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.torso.scale = Vec3::one();
        next
    }
}
