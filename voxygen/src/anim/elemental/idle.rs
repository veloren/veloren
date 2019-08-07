use super::{
    super::{Animation, SkeletonAttr},
    ElementalSkeleton,
};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct Input {
    pub attack: bool,
}
pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Skeleton = ElementalSkeleton;
    type Dependency = f64;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        global_time: f64,
        anim_time: f64,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave_ultra_slow = (anim_time as f32 * 1.0 + PI).sin();
        let wave_ultra_slow_cos = (anim_time as f32 * 1.0 + PI).cos();

        let head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 12.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.5,
            ((global_time + anim_time) as f32 / 12.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.25,
        );
        next.head.offset = Vec3::new(
            0.0 + skeleton_attr.neck_right,
            0.0 + skeleton_attr.neck_forward,
            skeleton_attr.neck_height + 15.0 + wave_ultra_slow * 0.3,
        );
        next.head.ori =
            Quaternion::rotation_z(head_look.x) * Quaternion::rotation_x(head_look.y.abs());
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.upper_torso.offset = Vec3::new(0.0, 0.0, 7.0 + wave_ultra_slow * 0.3);
        next.upper_torso.ori = Quaternion::rotation_x(0.0);
        next.upper_torso.scale = Vec3::one();

        next.lower_torso.offset = Vec3::new(0.0, 0.0, 2.0 + wave_ultra_slow * 0.3);
        next.lower_torso.ori = Quaternion::rotation_x(0.0);
        next.lower_torso.scale = Vec3::one();

        next.hand_l.offset = Vec3::new(
            -7.5,
            0.0 + wave_ultra_slow_cos * 0.15,
            0.0 + wave_ultra_slow * 0.5,
        );

        next.hand_l.ori = Quaternion::rotation_x(0.0 + wave_ultra_slow * -0.06);
        next.hand_l.scale = Vec3::one();

        next.hand_r.offset = Vec3::new(
            7.5,
            0.0 + wave_ultra_slow_cos * 0.15,
            0.0 + wave_ultra_slow * 0.5,
        );
        next.hand_r.ori = Quaternion::rotation_x(0.0 + wave_ultra_slow * -0.06);
        next.hand_r.scale = Vec3::one();

        next.feet.offset = Vec3::new(3.4, -0.1, 8.0);
        next.feet.ori = Quaternion::identity();
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
