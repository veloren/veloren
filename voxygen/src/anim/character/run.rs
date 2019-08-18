use super::{
    super::{Animation, SkeletonAttr},
    CharacterSkeleton,
};
use std::f32::consts::PI;
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
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 12.0).sin();
        let wave_cos = (anim_time as f32 * 12.0).cos();
        let wave_diff = (anim_time as f32 * 12.0 + PI / 2.0).sin();
        let wave_cos_dub = (anim_time as f32 * 24.0).cos();
        let wave_stop = (anim_time as f32 * 2.6).min(PI / 2.0).sin();

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

        next.head.offset = Vec3::new(
            0.0,
            -1.0 + skeleton_attr.neck_forward,
            skeleton_attr.neck_height + 15.0 + wave_cos * 1.3,
        );
        next.head.ori = Quaternion::rotation_z(head_look.x + wave * 0.1)
            * Quaternion::rotation_x(head_look.y + 0.35);
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(0.0, 0.0, 7.0 + wave_cos * 1.1);
        next.chest.ori = Quaternion::rotation_z(wave * 0.2);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0, 0.0, 5.0 + wave_cos * 1.1);
        next.belt.ori = Quaternion::rotation_z(wave * 0.35);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(0.0, 0.0, 2.0 + wave_cos * 1.1);
        next.shorts.ori = Quaternion::rotation_z(wave * 0.6);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(
            -7.5 + wave_cos_dub * 1.0,
            2.0 + wave_cos * 5.0,
            0.0 - wave * 1.5,
        );
        next.l_hand.ori = Quaternion::rotation_x(wave_cos * 0.8);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(
            7.5 - wave_cos_dub * 1.0,
            2.0 - wave_cos * 5.0,
            0.0 + wave * 1.5,
        );
        next.r_hand.ori = Quaternion::rotation_x(wave_cos * -0.8);
        next.r_hand.scale = Vec3::one();

        next.l_foot.offset = Vec3::new(-3.4, 0.0 + wave_cos * 1.0, 6.0 - wave_cos_dub * 0.7);
        next.l_foot.ori = Quaternion::rotation_x(-0.0 - wave_cos * 1.5);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(3.4, 0.0 - wave_cos * 1.0, 6.0 - wave_cos_dub * 0.7);
        next.r_foot.ori = Quaternion::rotation_x(-0.0 + wave_cos * 1.5);
        next.r_foot.scale = Vec3::one();

        next.weapon.offset = Vec3::new(
            -7.0 + skeleton_attr.weapon_x,
            -5.0 + skeleton_attr.weapon_y,
            15.0,
        );
        next.weapon.ori =
            Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57 + wave_cos * 0.25);
        next.weapon.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(-5.0, 0.0, 4.7);
        next.l_shoulder.ori = Quaternion::rotation_x(wave_cos * 0.15);
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(5.0, 0.0, 4.7);
        next.r_shoulder.ori = Quaternion::rotation_x(wave * 0.15);
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.draw.offset = Vec3::new(0.0, 5.0, 0.0);
        next.draw.ori = Quaternion::rotation_y(0.0);
        next.draw.scale = Vec3::one() * 0.0;

        next.torso.offset = Vec3::new(0.0, -0.2 + wave * -0.08, 0.4) * skeleton_attr.scaler;
        next.torso.ori =
            Quaternion::rotation_x(wave_stop * velocity * -0.06 + wave_diff * velocity * -0.005);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next
    }
}
