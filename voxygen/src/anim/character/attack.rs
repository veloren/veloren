use super::{
    super::{Animation, SkeletonAttr},
    CharacterSkeleton,
};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct Input {
    pub attack: bool,
}
pub struct AttackAnimation;

impl Animation for AttackAnimation {
    type Skeleton = CharacterSkeleton;
    type Dependency = f64;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        global_time: f64,
        anim_time: f64,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave_quicken = 1.0 - (anim_time as f32 * 16.0).cos();
        let wave_quicken_slow = 1.0 - (anim_time as f32 * 12.0).cos();
        let wave_quicken_double = 1.0 - (anim_time as f32 * 24.0).cos();
        let wave_quick = (anim_time as f32 * 0.5).sin();
        let wave_stop_quick = (anim_time as f32 * 16.0).min(PI / 2.0).sin();

        next.head.offset = Vec3::new(
            0.0 + skeleton_attr.neck_right,
            0.0 + skeleton_attr.neck_forward,
            skeleton_attr.neck_height + 15.0,
        );
        next.head.ori = Quaternion::rotation_z(wave_stop_quick * -0.25)
            * Quaternion::rotation_x(0.0 + wave_stop_quick * -0.1)
            * Quaternion::rotation_y(wave_stop_quick * 0.1);
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(0.0, 0.0, 7.0);
        next.chest.ori = Quaternion::rotation_x(0.0);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0, 0.0, 5.0);
        next.belt.ori = Quaternion::rotation_x(0.0);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(0.0, 0.0, 2.0);
        next.shorts.ori = Quaternion::rotation_x(0.0);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(
            -8.0 + wave_quicken_slow * 10.0,
            4.0 + wave_quicken_double * 3.0,
            9.0,
        );
        next.l_hand.ori = Quaternion::rotation_z(-0.8)
            * Quaternion::rotation_x(0.0 + wave_quicken * -0.8)
            * Quaternion::rotation_y(0.0 + wave_quicken * -0.4);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(0.0, -2.0, 6.5) / 11.0;
        next.r_hand.ori = Quaternion::rotation_x(0.0);
        next.r_hand.scale = Vec3::one() * 0.0;

        next.l_foot.offset = Vec3::new(
            -3.4,
            -0.1 + wave_stop_quick * 2.0,
            8.0 + wave_stop_quick * -4.0,
        );
        next.l_foot.ori = Quaternion::rotation_x(wave_stop_quick * -1.2);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(
            3.4,
            -0.1 - wave_stop_quick * -2.0,
            8.0 + wave_stop_quick * -2.0,
        );
        next.r_foot.ori = Quaternion::rotation_x(wave_stop_quick * 1.2);
        next.r_foot.scale = Vec3::one();

        next.weapon.offset = Vec3::new(
            -7.0 + skeleton_attr.weapon_x,
            -2.0 + skeleton_attr.weapon_y,
            5.0,
        );
        next.weapon.ori = Quaternion::rotation_y(2.5);
        next.weapon.scale = Vec3::one() * 0.0;

        next.l_shoulder.offset = Vec3::new(-10.0, -3.2, 2.5);
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = Vec3::one() * 1.04;

        next.r_shoulder.offset = Vec3::new(0.0, -3.2, 2.5);
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = Vec3::one() * 1.04;

        next.draw.offset = Vec3::new(0.0, 5.0, 0.0);
        next.draw.ori = Quaternion::rotation_y(0.0);
        next.draw.scale = Vec3::one() * 0.0;

        next.left_equip.offset = Vec3::new(
            -8.0 + wave_quicken_slow * 10.0 + skeleton_attr.weapon_x,
            4.0 + wave_quicken_double * 3.0,
            9.0,
        ) / 11.0
            * skeleton_attr.scaler;
        next.left_equip.ori = Quaternion::rotation_z(-0.8)
            * Quaternion::rotation_x(0.0 + wave_quicken * -0.8)
            * Quaternion::rotation_y(0.0 + wave_quicken * -0.4);
        next.left_equip.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next.right_equip.offset = Vec3::new(
            -8.0 + wave_quicken_slow * 10.0,
            4.0 + wave_quicken_double * 3.0,
            9.0,
        ) / 11.0
            * skeleton_attr.scaler;
        next.right_equip.ori = Quaternion::rotation_z(-0.8)
            * Quaternion::rotation_x(0.0 + wave_quicken * -0.8)
            * Quaternion::rotation_y(0.0 + wave_quicken * -0.4);
        next.right_equip.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next.torso.offset = Vec3::new(0.0, -0.2, 0.1) * skeleton_attr.scaler;
        next.torso.ori = Quaternion::rotation_z(wave_stop_quick * -0.2)
            * Quaternion::rotation_x(0.0 + wave_stop_quick * -0.2)
            * Quaternion::rotation_y(wave_stop_quick * 0.2);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
        next
    }
}
