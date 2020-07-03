use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::{Hands, ToolKind};
/* use std::f32::consts::PI; */
use vek::*;

pub struct LeapAnimation;

impl Animation for LeapAnimation {
    type Dependency = (Option<ToolKind>, Option<ToolKind>, Vec3<f32>, f64);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_leapmelee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_leapmelee")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, _velocity, _global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let lab = 1.0;
        let slowersmooth = (anim_time as f32 * lab as f32 * 4.0).sin();
        let slower = (((1.0)
            / (0.0001 + 0.999 * ((anim_time as f32 * lab as f32 * 4.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 4.0).sin());

        if let Some(ToolKind::Hammer(_)) = active_tool_kind {
            next.l_hand.offset = Vec3::new(-12.0, 0.0, 0.0);
            next.l_hand.ori = Quaternion::rotation_x(-0.0) * Quaternion::rotation_y(0.0);
            next.l_hand.scale = Vec3::one() * 1.08;
            next.r_hand.offset = Vec3::new(3.0, 0.0, 0.0);
            next.r_hand.ori = Quaternion::rotation_x(0.0) * Quaternion::rotation_y(0.0);
            next.r_hand.scale = Vec3::one() * 1.06;
            next.main.offset = Vec3::new(0.0, 0.0, 0.0);
            next.main.ori = Quaternion::rotation_x(0.0)
                * Quaternion::rotation_y(-1.57)
                * Quaternion::rotation_z(1.57);

            next.head.offset = Vec3::new(
                0.0,
                -2.0 + skeleton_attr.head.0 + slower * -1.0,
                skeleton_attr.head.1,
            );
            next.head.ori = Quaternion::rotation_z(slower * 0.05)
                * Quaternion::rotation_x((slowersmooth * -0.25 + slower * 0.55).max(-0.2))
                * Quaternion::rotation_y(slower * 0.05);
            next.head.scale = Vec3::one() * skeleton_attr.head_scale;

            next.chest.offset = Vec3::new(0.0, 0.0, 7.0);
            next.chest.ori = Quaternion::rotation_z(slower * 0.08 + slowersmooth * 0.15)
                * Quaternion::rotation_x(-0.3 + slower * 0.45 + slowersmooth * 0.26)
                * Quaternion::rotation_y(slower * 0.18 + slowersmooth * 0.15);

            next.belt.offset = Vec3::new(0.0, 0.0, -2.0 + slower * -0.7);
            next.belt.ori = Quaternion::rotation_z(slower * -0.16 + slowersmooth * -0.12)
                * Quaternion::rotation_x(0.0 + slower * -0.06)
                * Quaternion::rotation_y(slower * -0.05);

            next.shorts.offset = Vec3::new(0.0, 0.0, -5.0 + slower * -0.7);
            next.shorts.ori = Quaternion::rotation_z(slower * -0.08 + slowersmooth * -0.08)
                * Quaternion::rotation_x(0.0 + slower * -0.08 + slowersmooth * -0.08)
                * Quaternion::rotation_y(slower * -0.07);

            next.lantern.ori =
                Quaternion::rotation_x(slower * -0.7 + 0.4) * Quaternion::rotation_y(slower * 0.4);

            next.l_foot.offset = Vec3::new(
                -skeleton_attr.foot.0,
                slower * 3.0 + slowersmooth * -6.0 - 2.0,
                skeleton_attr.foot.2,
            );
            next.l_foot.ori = Quaternion::rotation_x(slower * -0.2 + slowersmooth * -0.3 - 0.2);

            next.r_foot.offset = Vec3::new(
                skeleton_attr.foot.0,
                slower * 2.0 + slowersmooth * -4.0 - 1.0,
                -2.0 + skeleton_attr.foot.2,
            );
            next.r_foot.ori = Quaternion::rotation_x(slower * -0.4 + slowersmooth * -0.6 - 1.0);

            next.control.scale = Vec3::one();
            next.control.offset = Vec3::new(-7.0, 7.0, 1.0);
            next.control.ori = Quaternion::rotation_x(-0.7 + slower * 1.5)
                * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(1.4 + slowersmooth * -0.4 + slower * 0.2);
            next.control.scale = Vec3::one();
        }
        next.lantern.offset = Vec3::new(
            skeleton_attr.lantern.0,
            skeleton_attr.lantern.1,
            skeleton_attr.lantern.2,
        );
        next.glider.offset = Vec3::new(0.0, 0.0, 10.0);
        next.glider.scale = Vec3::one() * 0.0;
        next.l_control.scale = Vec3::one();
        next.r_control.scale = Vec3::one();

        next.second.scale = match (
            active_tool_kind.map(|tk| tk.into_hands()),
            second_tool_kind.map(|tk| tk.into_hands()),
        ) {
            (Some(Hands::OneHand), Some(Hands::OneHand)) => Vec3::one(),
            (_, _) => Vec3::zero(),
        };

        next.torso.offset = Vec3::new(0.0, 0.0, 0.0) * skeleton_attr.scaler;
        next.torso.ori = Quaternion::rotation_z(0.0);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
        next
    }
}
