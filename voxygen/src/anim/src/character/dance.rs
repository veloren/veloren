use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::ToolKind;
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct DanceAnimation;

impl Animation for DanceAnimation {
    type Dependency = (Option<ToolKind>, f64);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_dance\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_dance")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        *rate = 1.0;

        let lab = 1.0;
        let short = (((5.0)
            / (3.0 + 2.0 * ((anim_time as f32 * lab as f32 * 6.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 6.0).sin());
        let noisea = (anim_time as f32 * 11.0 + PI / 6.0).sin();
        let noiseb = (anim_time as f32 * 19.0 + PI / 4.0).sin();

        let shorte = (anim_time as f32 * lab as f32 * 6.0).sin();

        let shortealt = (anim_time as f32 * lab as f32 * 6.0 + PI / 2.0).sin();

        let foot = (((5.0)
            / (1.0 + (4.0) * ((anim_time as f32 * lab as f32 * 8.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 8.0).sin());

        let head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 6.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.3,
            ((global_time + anim_time) as f32 / 6.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.15,
        );

        next.head.offset = Vec3::new(0.0, -2.0 + skeleton_attr.head.0, skeleton_attr.head.1);
        next.head.ori = Quaternion::rotation_z(short * -0.6)
            * Quaternion::rotation_x(0.2 + head_look.y.max(0.0) + shorte.abs() * -0.2);
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(
            0.0,
            skeleton_attr.chest.0,
            skeleton_attr.chest.1 + shortealt * 1.5,
        );
        next.chest.ori = Quaternion::rotation_z(short * 0.35)
            * Quaternion::rotation_y(shorte * 0.08)
            * Quaternion::rotation_x(foot * 0.07);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0, skeleton_attr.belt.0, skeleton_attr.belt.1);
        next.belt.ori = Quaternion::rotation_z(shorte * 0.25);
        next.belt.scale = Vec3::one();

        next.back.offset = Vec3::new(0.0, skeleton_attr.back.0, skeleton_attr.back.1);
        next.back.ori = Quaternion::rotation_x(-0.25 + shorte * 0.1 + noisea * 0.1 + noiseb * 0.1);
        next.back.scale = Vec3::one() * 1.02;

        next.shorts.offset = Vec3::new(0.0, skeleton_attr.shorts.0, skeleton_attr.shorts.1);
        next.shorts.ori = Quaternion::rotation_z(foot * 0.35);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(
            1.0 - skeleton_attr.hand.0,
            2.0 + skeleton_attr.hand.1 + shortealt * -3.0,
            skeleton_attr.hand.2 + shortealt * -0.75,
        );
        next.l_hand.ori = Quaternion::rotation_x(1.4 + foot * 0.15) * Quaternion::rotation_y(0.2);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(
            -1.0 + skeleton_attr.hand.0,
            2.0 + skeleton_attr.hand.1 + shortealt * 3.0,
            skeleton_attr.hand.2 + shortealt * 0.75,
        );
        next.r_hand.ori = Quaternion::rotation_x(1.4 + foot * -0.15) * Quaternion::rotation_y(-0.2);
        next.r_hand.scale = Vec3::one();

        next.l_foot.offset = Vec3::new(
            -skeleton_attr.foot.0 + foot * 0.8,
            1.5 + -skeleton_attr.foot.1 + foot * -4.0,
            skeleton_attr.foot.2 + 2.0,
        );
        next.l_foot.ori =
            Quaternion::rotation_x(foot * -0.3) * Quaternion::rotation_z(short * -0.15);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(
            skeleton_attr.foot.0 + foot * 0.8,
            1.5 + -skeleton_attr.foot.1 + foot * 4.0,
            skeleton_attr.foot.2 + 2.0,
        );
        next.r_foot.ori = Quaternion::rotation_x(foot * 0.3) * Quaternion::rotation_z(short * 0.15);
        next.r_foot.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(
            -skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.l_shoulder.ori = Quaternion::rotation_x(shorte * 0.15);
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.r_shoulder.ori = Quaternion::rotation_x(shorte * -0.15);
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.glider.offset = Vec3::new(0.0, 0.0, 10.0);
        next.glider.scale = Vec3::one() * 0.0;

        next.main.offset = Vec3::new(-7.0, -6.5, 15.0);
        next.main.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57 + shorte * 0.25);
        next.main.scale = Vec3::one();

        next.second.scale = Vec3::one() * 0.0;

        next.lantern.offset = Vec3::new(
            skeleton_attr.lantern.0,
            skeleton_attr.lantern.1,
            skeleton_attr.lantern.2,
        );
        next.lantern.ori =
            Quaternion::rotation_x(shorte * 0.7 + 0.4) * Quaternion::rotation_y(shorte * 0.4);
        next.lantern.scale = Vec3::one() * 0.65;

        next.torso.offset = Vec3::new(0.0, -0.3, 0.0) * skeleton_attr.scaler;
        next.torso.ori = Quaternion::rotation_z(short * -0.2);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next.control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.control.ori = Quaternion::rotation_x(0.0);
        next.control.scale = Vec3::one();

        next.l_control.scale = Vec3::one();

        next.r_control.scale = Vec3::one();

        next
    }
}
