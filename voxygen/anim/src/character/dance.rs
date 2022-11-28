use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::ToolKind;
use std::{f32::consts::PI, ops::Mul};

pub struct DanceAnimation;

impl Animation for DanceAnimation {
    type Dependency<'a> = (Option<ToolKind>, Option<ToolKind>, f32);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_dance\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_dance")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, _second_tool_kind, global_time): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        *rate = 1.0;

        let lab: f32 = 1.0;
        let short = ((5.0 / (3.0 + 2.0 * ((anim_time * lab * 6.0).sin()).powi(2))).sqrt())
            * ((anim_time * lab * 6.0).sin());
        let noisea = (anim_time * 11.0 + PI / 6.0).sin();
        let noiseb = (anim_time * 19.0 + PI / 4.0).sin();

        let shorte = (anim_time * lab * 6.0).sin();

        let shortealt = (anim_time * lab * 6.0 + PI / 2.0).sin();

        let foot = ((5.0 / (1.0 + (4.0) * ((anim_time * lab * 8.0).sin()).powi(2))).sqrt())
            * ((anim_time * lab * 8.0).sin());

        let head_look = Vec2::new(
            (global_time + anim_time / 6.0).floor().mul(7331.0).sin() * 0.3,
            (global_time + anim_time / 6.0).floor().mul(1337.0).sin() * 0.15,
        );

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_z(short * -0.6)
            * Quaternion::rotation_x(0.2 + head_look.y.max(0.0) + shorte.abs() * -0.2);

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + shortealt * 1.5);
        next.chest.orientation = Quaternion::rotation_z(short * 0.35)
            * Quaternion::rotation_y(shorte * 0.08)
            * Quaternion::rotation_x(foot * 0.07);

        next.belt.position = Vec3::new(0.0, s_a.belt.0, s_a.belt.1);
        next.belt.orientation = Quaternion::rotation_z(shorte * 0.25);

        next.back.position = Vec3::new(0.0, s_a.back.0, s_a.back.1);
        next.back.orientation =
            Quaternion::rotation_x(-0.25 + shorte * 0.1 + noisea * 0.1 + noiseb * 0.1);

        next.shorts.position = Vec3::new(0.0, s_a.shorts.0, s_a.shorts.1);
        next.shorts.orientation = Quaternion::rotation_z(foot * 0.35);

        next.hand_l.position = Vec3::new(
            1.0 - s_a.hand.0,
            2.0 + s_a.hand.1 + shortealt * -3.0,
            s_a.hand.2 + shortealt * -0.75,
        );
        next.hand_l.orientation =
            Quaternion::rotation_x(1.4 + foot * 0.15) * Quaternion::rotation_y(0.2);

        next.hand_r.position = Vec3::new(
            -1.0 + s_a.hand.0,
            2.0 + s_a.hand.1 + shortealt * 3.0,
            s_a.hand.2 + shortealt * 0.75,
        );
        next.hand_r.orientation =
            Quaternion::rotation_x(1.4 + foot * -0.15) * Quaternion::rotation_y(-0.2);

        next.foot_l.position = Vec3::new(
            -s_a.foot.0 + foot * 0.8,
            1.5 + -s_a.foot.1 + foot * -4.0,
            s_a.foot.2 + 2.0,
        );
        next.foot_l.orientation =
            Quaternion::rotation_x(foot * -0.3) * Quaternion::rotation_z(short * -0.15);

        next.foot_r.position = Vec3::new(
            s_a.foot.0 + foot * 0.8,
            1.5 + -s_a.foot.1 + foot * 4.0,
            s_a.foot.2 + 2.0,
        );
        next.foot_r.orientation =
            Quaternion::rotation_x(foot * 0.3) * Quaternion::rotation_z(short * 0.15);

        next.shoulder_l.position = Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_l.orientation = Quaternion::rotation_x(shorte * 0.15);

        next.shoulder_r.position = Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_r.orientation = Quaternion::rotation_x(shorte * -0.15);

        next.lantern.orientation =
            Quaternion::rotation_x(shorte * 0.7 + 0.4) * Quaternion::rotation_y(shorte * 0.4);

        next.torso.position = Vec3::new(0.0, -3.3, 0.0);
        next.torso.orientation = Quaternion::rotation_z(short * -0.2);

        next
    }
}
