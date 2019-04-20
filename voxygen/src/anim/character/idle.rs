// Standard
use std::f32::consts::PI;

// Library
use vek::*;

// Local
use super::{
    CharacterSkeleton,
    super::Animation,
    SCALE,
};

pub struct IdleAnimation;

//TODO: Make it actually good, possibly add the head rotating slightly, add breathing, etc.
impl Animation for IdleAnimation {
    type Skeleton = CharacterSkeleton;
    type Dependency = f64;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        time: f64,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        next.head.offset = Vec3::unit_z() * 13.0 / SCALE;
        next.head.ori = Quaternion::rotation_z(0.0);

        next.chest.offset = Vec3::unit_z() * 9.0 / SCALE;
        next.chest.ori = Quaternion::rotation_z(0.0);

        next.belt.offset = Vec3::unit_z() * 7.0 / SCALE;
        next.belt.ori = Quaternion::rotation_z(0.0);

        next.shorts.offset = Vec3::unit_z() * 4.0 / SCALE;
        next.shorts.ori = Quaternion::rotation_z(0.0);

        next.l_hand.offset = Vec3::new(-8.0, 0.0, 9.0) / SCALE;
        next.r_hand.offset = Vec3::new(8.0, 0.0, 9.0 ) / SCALE;

        next.l_foot.offset = Vec3::new(-3.5, 0.0, 3.0) / SCALE;
        next.l_foot.ori = Quaternion::rotation_x(0.0);
        next.r_foot.offset = Vec3::new(3.5, 0.0, 3.0) / SCALE;
        next.r_foot.ori = Quaternion::rotation_x(0.0);

        next.back.offset = Vec3::new(-9.0, 5.0, 18.0);
        next.back.ori = Quaternion::rotation_y(2.5);
        next.back.scale = Vec3::one();

        next
    }
}
