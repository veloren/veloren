use super::gliding::GlidingAnimation;
use super::{
    super::{Animation, SkeletonAttr},
    CharacterSkeleton,
};
use std::f32::consts::PI;
use vek::*;

pub struct BarrelRollAnimation;

impl Animation for BarrelRollAnimation {
    type Skeleton = CharacterSkeleton;
    type Dependency = (f32, f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (velocity, global_time): Self::Dependency,
        anim_time: f64,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        // Perform Glide animation
        let mut next = GlidingAnimation::update_skeleton(
            skeleton,
            (velocity, global_time),
            anim_time,
            skeleton_attr,
        );

        // Rotate - roll component is removed after .55s, see phys.rs, which
        // is roughly 1 radian
        let roll = (anim_time as f32) * 2.0 * PI * 0.95;
        next.head.ori = Quaternion::rotation_y(roll);
        next.chest.ori = Quaternion::rotation_y(roll);
        next.belt.ori = Quaternion::rotation_y(roll);
        next.shorts.ori = Quaternion::rotation_y(roll);
        next.l_hand.ori = Quaternion::rotation_y(roll);
        next.r_hand.ori = Quaternion::rotation_y(roll);
        next.l_foot.ori = Quaternion::rotation_y(roll);
        next.r_foot.ori = Quaternion::rotation_y(roll);
        next.l_shoulder.ori = Quaternion::rotation_y(roll);
        next.r_shoulder.ori = Quaternion::rotation_y(roll);
        next.draw.ori = Quaternion::rotation_y(roll);
        next.torso.ori = Quaternion::rotation_y(roll);

        next
    }
}
