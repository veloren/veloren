use super::{super::Animation, CharacterSkeleton};
use std::ops::Mul;
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Skeleton = CharacterSkeleton;
    type Dependency = (f32, f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (velocity, global_time): Self::Dependency,
        anim_time: f64,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 14.0).sin();
        let wave_double = (anim_time as f32 * 28.0).sin();
        let wave_cos_double = (anim_time as f32 * 28.0).cos();
        let wave_cos = (anim_time as f32 * 14.0).cos();

        let head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 2.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.2,
            ((global_time + anim_time) as f32 / 2.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.1,
        );

        next.head.offset = Vec3::new(0.0, -1.0, 16.0 + wave_cos * 1.3);
        next.head.ori = Quaternion::rotation_z(head_look.x + wave * 0.1)
            * Quaternion::rotation_x(head_look.y + 0.35);
        next.head.scale = Vec3::one();

        next.chest.offset = Vec3::new(0.0, 0.0, 7.0 + wave_cos * 1.1);
        next.chest.ori = Quaternion::rotation_z(wave * 0.1);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0, 0.0, 5.0 + wave_cos * 1.1);
        next.belt.ori = Quaternion::rotation_z(wave * 0.25);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(0.0, 0.0, 2.0 + wave_cos * 1.1);
        next.shorts.ori = Quaternion::rotation_z(wave * 0.6);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(
            -7.5 + wave_double * 1.5,
            3.0 + wave_cos * 8.0,
            12.0 - wave * 1.0,
        ) / 11.0;
        next.l_hand.ori = Quaternion::rotation_x(wave_cos * 1.1);
        next.l_hand.scale = Vec3::one() / 11.0;

        next.r_hand.offset = Vec3::new(
            7.5 + wave_double * -1.5,
            3.0 - wave_cos * 8.0,
            12.0 + wave * 1.0,
        ) / 11.0;
        next.r_hand.ori = Quaternion::rotation_x(wave_cos * -1.1);
        next.r_hand.scale = Vec3::one() / 11.0;

        next.l_foot.offset = Vec3::new(-3.4, 0.0 + wave_cos * 1.0, 6.0);
        next.l_foot.ori = Quaternion::rotation_x(-0.0 - wave_cos * 1.5);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(3.4, 0.0 - wave_cos * 1.0, 6.0);
        next.r_foot.ori = Quaternion::rotation_x(-0.0 + wave_cos * 1.5);
        next.r_foot.scale = Vec3::one();

        next.weapon.offset = Vec3::new(-7.0, -7.0, 18.0);
        next.weapon.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57 + wave * 0.5);
        next.weapon.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(-10.0, -3.2, 2.5);
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = Vec3::one() * 1.04;

        next.r_shoulder.offset = Vec3::new(0.0, -3.2, 2.5);
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = Vec3::one() * 1.04;

        next.draw.offset = Vec3::new(0.0, 5.0, 0.0);
        next.draw.ori = Quaternion::rotation_y(0.0);
        next.draw.scale = Vec3::one() * 0.0;

        next.left_equip.offset = Vec3::new(0.0, 0.0, 5.0) / 11.0;
        next.left_equip.ori = Quaternion::rotation_x(0.0);;
        next.left_equip.scale = Vec3::one() * 0.0;

        next.right_equip.offset = Vec3::new(0.0, 0.0, 5.0) / 11.0;
        next.right_equip.ori = Quaternion::rotation_x(0.0);;
        next.right_equip.scale = Vec3::one() * 0.0;

        next.torso.offset = Vec3::new(0.0, -0.2, 0.4);
        next.torso.ori = Quaternion::rotation_x(-velocity * 0.04 - wave_cos * 0.008 * velocity);
        next.torso.scale = Vec3::one() / 11.0;

        next
    }
}
