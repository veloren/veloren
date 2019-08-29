use super::{
    super::{Animation, SkeletonAttr},
    CharacterSkeleton,
};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct Input {
    pub attack: bool,
}
pub struct StandAnimation;

impl Animation for StandAnimation {
    type Skeleton = CharacterSkeleton;
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

        next.chest.offset = Vec3::new(0.0, 0.0, 7.0 + wave_ultra_slow * 0.3);
        next.chest.ori = Quaternion::rotation_x(0.0);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0, 0.0, 5.0 + wave_ultra_slow * 0.3);
        next.belt.ori = Quaternion::rotation_x(0.0);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(0.0, 0.0, 2.0 + wave_ultra_slow * 0.3);
        next.shorts.ori = Quaternion::rotation_x(0.0);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(
            -7.5,
            0.0 + wave_ultra_slow_cos * 0.15,
            0.0 + wave_ultra_slow * 0.5,
        );

        next.l_hand.ori = Quaternion::rotation_x(0.0 + wave_ultra_slow * -0.06);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(
            7.5,
            0.0 + wave_ultra_slow_cos * 0.15,
            0.0 + wave_ultra_slow * 0.5,
        );
        next.r_hand.ori = Quaternion::rotation_x(0.0 + wave_ultra_slow * -0.06);
        next.r_hand.scale = Vec3::one();

        next.l_foot.offset = Vec3::new(-3.4, -0.1, 8.0);
        next.l_foot.ori = Quaternion::identity();
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(3.4, -0.1, 8.0);
        next.r_foot.ori = Quaternion::identity();
        next.r_foot.scale = Vec3::one();

        next.weapon.offset = Vec3::new(
            -7.0 + skeleton_attr.weapon_x,
            -5.0 + skeleton_attr.weapon_y,
            15.0,
        );
        next.weapon.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
        next.weapon.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(-5.0, 0.0, 4.7);
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(5.0, 0.0, 4.7);
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.draw.offset = Vec3::new(0.0, 5.0, 0.0);
        next.draw.ori = Quaternion::rotation_y(0.0);
        next.draw.scale = Vec3::one() * 0.0;

        next.torso.offset = Vec3::new(0.0, -0.2, 0.1) * skeleton_attr.scaler;
        next.torso.ori = Quaternion::rotation_x(0.0);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next
    }
}
