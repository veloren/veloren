// Standard
use std::f32::consts::PI;

// Library
use vek::*;

// Local
use super::{
    CharacterSkeleton,
    super::Animation,
};

pub struct IdleAnimation;

//TODO: Make it actually good, possibly add the head rotating slightly, add breathing, etc.
impl Animation for IdleAnimation {
    type Skeleton = CharacterSkeleton;
    type Dependency = f64;

    fn update_skeleton(
        skeleton: &mut Self::Skeleton,
        time: f64,
    ) {
        skeleton.head.offset = Vec3::unit_z() * 13.0 / 11.0;
        skeleton.head.ori = Quaternion::rotation_z(0.0);

        skeleton.chest.offset = Vec3::unit_z() * 9.0 / 11.0;
        skeleton.chest.ori = Quaternion::rotation_z(0.0);

        skeleton.belt.offset = Vec3::unit_z() * 7.0 / 11.0;
        skeleton.belt.ori = Quaternion::rotation_z(0.0);

        skeleton.shorts.offset = Vec3::unit_z() * 4.0 / 11.0;
        skeleton.shorts.ori = Quaternion::rotation_z(0.0);

        skeleton.l_hand.offset = Vec3::new(-8.0, 0.0, 9.0) / 11.0;
        skeleton.r_hand.offset = Vec3::new(8.0, 0.0, 9.0 ) / 11.0;

        skeleton.l_foot.offset = Vec3::new(-3.5, 0.0, 3.0) / 11.0;
        skeleton.l_foot.ori = Quaternion::rotation_x(0.0);
        skeleton.r_foot.offset = Vec3::new(3.5, 0.0, 3.0) / 11.0;
        skeleton.r_foot.ori = Quaternion::rotation_x(0.0);

        skeleton.back.offset = Vec3::new(-9.0, 5.0, 18.0);
        skeleton.back.ori = Quaternion::rotation_y(2.5);
        skeleton.back.scale = Vec3::one();
    }
}