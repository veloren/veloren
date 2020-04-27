use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::ToolKind;
use std::f32::consts::PI;
use vek::*;

pub struct RollAnimation;

impl Animation for RollAnimation {
    type Dependency = (Option<ToolKind>, Vec3<f32>, Vec3<f32>, f64);
    type Skeleton = CharacterSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, orientation, last_ori, _global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 4.5).sin();
        let wave_quick = (anim_time as f32 * 7.5).sin();
        let wave_quick_cos = (anim_time as f32 * 7.5).cos();
        let wave_slow = (anim_time as f32 * 2.3 + PI).sin();
        let wave_dub = (anim_time as f32 * 4.5).sin();

        let ori = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if Vec2::new(ori, last_ori)
            .map(|o| Vec2::<f32>::from(o).magnitude_squared())
            .map(|m| m > 0.001 && m.is_finite())
            .reduce_and()
            && ori.angle_between(last_ori).is_finite()
        {
            ori.angle_between(last_ori).min(0.5)
                * last_ori.determine_side(Vec2::zero(), ori).signum()
        } else {
            0.0
        } * 1.3;

        next.head.offset = Vec3::new(
            0.0,
            -2.0 + skeleton_attr.head.0,
            skeleton_attr.head.1 + wave_dub * -8.0,
        );
        next.head.ori = Quaternion::rotation_x(wave_dub * 0.4);
        next.head.scale = Vec3::one();

        next.chest.offset = Vec3::new(
            0.0,
            skeleton_attr.chest.0,
            skeleton_attr.chest.1 + wave_dub * -5.0,
        );
        next.chest.ori = Quaternion::rotation_x(wave_dub * 0.4);
        next.chest.scale = Vec3::one() * 1.01;

        next.belt.offset = Vec3::new(
            0.0,
            skeleton_attr.belt.0,
            skeleton_attr.belt.0 + wave_dub * -3.0,
        );
        next.belt.ori = Quaternion::rotation_x(0.0 + wave_dub * 0.4);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(
            0.0,
            skeleton_attr.shorts.0,
            skeleton_attr.shorts.0 + wave_dub * -2.0,
        );
        next.shorts.ori = Quaternion::rotation_x(0.0 + wave_dub * 0.4);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(
            -skeleton_attr.chest.0 + wave * -0.5,
            skeleton_attr.hand.1 + wave_quick_cos * -5.5,
            skeleton_attr.hand.2 + wave_quick * 0.5,
        );

        next.l_hand.ori =
            Quaternion::rotation_x(wave_slow * 6.5) * Quaternion::rotation_y(wave * 0.3);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(
            skeleton_attr.hand.0 + wave * 0.5,
            skeleton_attr.hand.1 + wave_quick_cos * 2.5,
            skeleton_attr.hand.2 + wave_quick * 3.0,
        );
        next.r_hand.ori =
            Quaternion::rotation_x(wave_slow * 6.5) * Quaternion::rotation_y(wave * 0.3);
        next.r_hand.scale = Vec3::one();

        next.l_foot.offset = Vec3::new(
            -skeleton_attr.foot.0,
            skeleton_attr.foot.1,
            skeleton_attr.foot.2 + wave_dub * -1.2 + wave_slow * 4.0,
        );
        next.l_foot.ori = Quaternion::rotation_x(wave * 0.6);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(
            skeleton_attr.foot.0,
            skeleton_attr.foot.1,
            skeleton_attr.foot.2 + wave_dub * -1.0 + wave_slow * 4.0,
        );
        next.r_foot.ori = Quaternion::rotation_x(wave * -0.4);
        next.r_foot.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(
            -skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.2,
        );
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.2,
        );
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.glider.offset = Vec3::new(0.0, 0.0, 10.0);
        next.glider.scale = Vec3::one() * 0.0;

        next.main.offset = Vec3::new(-7.0, -5.0, 15.0);
        next.main.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
        next.main.scale = Vec3::one();

        next.second.offset = Vec3::new(0.0, 0.0, 0.0);
        next.second.scale = Vec3::one() * 0.0;

        next.lantern.offset = Vec3::new(
            skeleton_attr.lantern.0,
            skeleton_attr.lantern.1,
            skeleton_attr.lantern.2,
        );
        next.lantern.ori = Quaternion::rotation_x(0.1) * Quaternion::rotation_y(0.1);
        next.lantern.scale = Vec3::one() * 0.65;

        next.torso.offset =
            Vec3::new(0.0, 0.0, 0.1 + wave_dub * 16.0) / 11.0 * skeleton_attr.scaler;
        next.torso.ori = Quaternion::rotation_x(wave_slow * 6.5) * Quaternion::rotation_y(tilt);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next.control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.control.ori = Quaternion::rotation_x(0.0);
        next.control.scale = Vec3::one();

        next.l_control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.l_control.ori = Quaternion::rotation_x(0.0);
        next.l_control.scale = Vec3::one();

        next.r_control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.r_control.ori = Quaternion::rotation_x(0.0);
        next.r_control.scale = Vec3::one();
        next
    }
}
