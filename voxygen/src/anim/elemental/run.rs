use super::{
    super::{Animation, SkeletonAttr},
    ElementalSkeleton,
};
use std::f32::consts::PI;
use std::ops::Mul;
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Skeleton = ElementalSkeleton;
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

        next.upper_torso.offset = Vec3::new(0.0, 0.0, 7.0 + wave_cos * 1.1);
        next.upper_torso.ori = Quaternion::rotation_z(wave * 0.1);
        next.upper_torso.scale = Vec3::one();

        next.lower_torso.offset = Vec3::new(0.0, 0.0, 2.0 + wave_cos * 1.1);
        next.lower_torso.ori = Quaternion::rotation_z(wave * 0.6);
        next.lower_torso.scale = Vec3::one();

        next.hand_l.offset = Vec3::new(
            -7.5 + wave_cos_dub * 1.0,
            2.0 + wave_cos * 5.0,
            0.0 - wave * 1.5,
        );
        next.hand_l.ori = Quaternion::rotation_x(wave_cos * 0.8);
        next.hand_l.scale = Vec3::one();

        next.hand_r.offset = Vec3::new(
            7.5 - wave_cos_dub * 1.0,
            2.0 - wave_cos * 5.0,
            0.0 + wave * 1.5,
        );
        next.hand_r.ori = Quaternion::rotation_x(wave_cos * -0.8);
        next.hand_r.scale = Vec3::one();

        next.feet.offset = Vec3::new(3.4, 0.0 - wave_cos * 1.0, 6.0 - wave_cos_dub * 0.7);
        next.feet.ori = Quaternion::rotation_x(-0.0 + wave_cos * 1.5);
        next.feet.scale = Vec3::one();

        next.shoulder_l.offset = Vec3::new(-10.0, -3.2, 2.5);
        next.shoulder_l.ori = Quaternion::rotation_x(0.0);
        next.shoulder_l.scale = Vec3::one() * 1.04;

        next.shoulder_r.offset = Vec3::new(0.0, -3.2, 2.5);
        next.shoulder_r.ori = Quaternion::rotation_x(0.0);
        next.shoulder_r.scale = Vec3::one() * 1.04;

        next
    }
}
