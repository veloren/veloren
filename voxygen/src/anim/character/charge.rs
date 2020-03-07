use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::ToolKind;
use std::f32::consts::PI;
use vek::*;

pub struct Input {
    pub attack: bool,
}
pub struct ChargeAnimation;

impl Animation for ChargeAnimation {
    type Dependency = (Option<ToolKind>, f64);
    type Skeleton = CharacterSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, _global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave_stop_quick = (anim_time as f32 * 8.0).min(PI / 2.0).sin();
        let constant = 8.0;

        let wave_cos = (((5.0)
            / (1.1 + 3.9 * ((anim_time as f32 * constant as f32 * 2.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * constant as f32 * 1.5).sin());
        let wave_cos_dub = (((5.0)
            / (1.1 + 3.9 * ((anim_time as f32 * constant as f32 * 4.8).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * constant as f32 * 1.5).sin());
        next.head.offset = Vec3::new(
            0.0 + skeleton_attr.neck_right,
            5.0 + skeleton_attr.neck_forward,
            skeleton_attr.neck_height + 19.0 + wave_cos * 2.0,
        );
        next.head.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0) * Quaternion::rotation_y(0.0);
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(0.0, 0.0, 7.0 + wave_cos * 2.0);
        next.chest.ori = Quaternion::rotation_x(-0.7) * Quaternion::rotation_z(-0.9);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0, 0.0, 5.0 + wave_cos * 2.0);
        next.belt.ori = Quaternion::rotation_x(-0.6) * Quaternion::rotation_z(-0.9);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(0.0, 0.0, 2.0 + wave_cos * 2.0);
        next.shorts.ori = Quaternion::rotation_x(-0.5) * Quaternion::rotation_z(-0.9);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(-8.0, -7.5, 5.0);
        next.l_hand.ori = Quaternion::rotation_z(0.5)
            * Quaternion::rotation_x(wave_stop_quick * -1.5 + 0.5 + 1.57)
            * Quaternion::rotation_y(-0.6);
        next.l_hand.scale = Vec3::one() * 1.01;

        next.r_hand.offset = Vec3::new(-8.0, -8.0, 3.0);
        next.r_hand.ori = Quaternion::rotation_z(0.5)
            * Quaternion::rotation_x(wave_stop_quick * -1.5 + 0.5 + 1.57)
            * Quaternion::rotation_y(-0.6);
        next.r_hand.scale = Vec3::one() * 1.01;

        next.l_foot.offset = Vec3::new(-3.4, 0.0 + wave_cos * 1.0, 6.0 - wave_cos_dub * 0.7);
        next.l_foot.ori = Quaternion::rotation_x(-0.0 - wave_cos * 0.8);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(3.4, 0.0 - wave_cos * 1.0, 6.0 - wave_cos_dub * 0.7);
        next.r_foot.ori = Quaternion::rotation_x(-0.0 + wave_cos * 0.8);
        next.r_foot.scale = Vec3::one();

        next.main.offset = Vec3::new(
            -8.0 + skeleton_attr.weapon_x,
            0.0 + skeleton_attr.weapon_y,
            5.0,
        );
        next.main.ori = Quaternion::rotation_z(-0.0)
            * Quaternion::rotation_x(wave_stop_quick * -1.5 + 0.7)
            * Quaternion::rotation_y(-0.5);
        next.main.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(-5.0, 0.0, 4.7);
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(5.0, 0.0, 4.7);
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.glider.offset = Vec3::new(0.0, 5.0, 0.0);
        next.glider.ori = Quaternion::rotation_y(0.0);
        next.glider.scale = Vec3::one() * 0.0;

        next.lantern.offset = Vec3::new(0.0, 0.0, 0.0);
        next.lantern.ori = Quaternion::rotation_x(0.0);
        next.lantern.scale = Vec3::one() * 0.0;

        next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
        next.torso.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0) * Quaternion::rotation_y(0.0);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next.control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.control.ori = Quaternion::rotation_x(0.0);
        next.control.scale = Vec3::one();

        next
    }
}
