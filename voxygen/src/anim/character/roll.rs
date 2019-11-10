use super::{
    super::{Animation, SkeletonAttr},
    CharacterSkeleton,
};
use common::comp::item::Tool;
use std::f32::consts::PI;
use vek::*;

pub struct RollAnimation;

impl Animation for RollAnimation {
    type Skeleton = CharacterSkeleton;
    type Dependency = (Option<Tool>, f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, _global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 5.5).sin();
        let wave_quick = (anim_time as f32 * 9.5).sin();
        let wave_quick_cos = (anim_time as f32 * 9.5).cos();
        let wave_slow = (anim_time as f32 * 2.8 + PI).sin();
        let wave_dub = (anim_time as f32 * 5.5).sin();

        next.head.offset = Vec3::new(
            0.0 + skeleton_attr.neck_right,
            -2.0 + skeleton_attr.neck_forward,
            skeleton_attr.neck_height + 21.0 + wave_dub * -8.0,
        );
        next.head.ori = Quaternion::rotation_x(wave_dub * 0.4);
        next.head.scale = Vec3::one();

        next.chest.offset = Vec3::new(0.0, 0.0, 7.0 + wave_dub * -5.0);
        next.chest.ori = Quaternion::rotation_x(wave_dub * 0.4);
        next.chest.scale = Vec3::one() * 1.01;

        next.belt.offset = Vec3::new(0.0, 0.0, 5.0 + wave_dub * -3.0);
        next.belt.ori = Quaternion::rotation_x(0.0 + wave_dub * 0.4);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(0.0, 0.0, 2.0 + wave_dub * -2.0);
        next.shorts.ori = Quaternion::rotation_x(0.0 + wave_dub * 0.4);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(
            -5.5 + wave * -0.5,
            -2.0 + wave_quick_cos * -5.5,
            1.0 + wave_quick * 0.5,
        );

        next.l_hand.ori =
            Quaternion::rotation_x(wave_slow * 6.5) * Quaternion::rotation_y(wave * 0.3);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(
            5.5 + wave * 0.5,
            -2.0 + wave_quick_cos * 2.5,
            1.0 + wave_quick * 3.0,
        );
        next.r_hand.ori =
            Quaternion::rotation_x(wave_slow * 6.5) * Quaternion::rotation_y(wave * 0.3);
        next.r_hand.scale = Vec3::one();

        next.l_foot.offset = Vec3::new(-3.4, -0.1, 9.0 - 0.0 + wave_dub * -1.2 + wave_slow * 4.0);
        next.l_foot.ori = Quaternion::rotation_x(wave * 0.6);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(3.4, -0.1, 9.0 - 0.0 + wave_dub * -1.0 + wave_slow * 4.0);
        next.r_foot.ori = Quaternion::rotation_x(wave * -0.4);
        next.r_foot.scale = Vec3::one();

        next.weapon.offset = Vec3::new(
            -7.0 + skeleton_attr.weapon_x,
            -5.0 + skeleton_attr.weapon_y,
            15.0,
        );
        next.weapon.ori = Quaternion::rotation_y(2.5)
            * Quaternion::rotation_z(1.57)
            * Quaternion::rotation_x(0.0);
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

        next.torso.offset =
            Vec3::new(0.0, 11.0, 0.1 + wave_dub * 16.0) / 11.0 * skeleton_attr.scaler;
        next.torso.ori = Quaternion::rotation_x(wave_slow * 6.5);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
        next
    }
}
