use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use std::{f32::consts::PI, ops::Mul};

pub struct SitAnimation;

impl Animation for SitAnimation {
    type Dependency = (Option<ToolKind>, Option<ToolKind>, f64);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_sit\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_sit")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let slow = (anim_time as f32 * 1.0).sin();
        let slowa = (anim_time as f32 * 1.0 + PI / 2.0).sin();
        let stop = (anim_time as f32 * 3.0).min(PI / 2.0).sin();
        let slow_abs = ((anim_time as f32 * 0.3).sin()) + 1.0;

        let head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 18.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.25,
            ((global_time + anim_time) as f32 / 18.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.125,
        );
        next.head.position = Vec3::new(
            0.0,
            skeleton_attr.head.0,
            skeleton_attr.head.1 + slow * 0.1 + stop * -0.8,
        );
        next.head.orientation = Quaternion::rotation_z(head_look.x + slow * 0.2 - slow * 0.1)
            * Quaternion::rotation_x((slowa * -0.1 + slow * 0.1 + head_look.y).abs());
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.position = Vec3::new(
            0.0,
            skeleton_attr.chest.0 + stop * -0.4,
            skeleton_attr.chest.1 + slow * 0.1 + stop * -0.8,
        );
        next.chest.orientation = Quaternion::rotation_x(stop * 0.15);
        next.chest.scale = Vec3::one() + slow_abs * 0.05;

        next.belt.position =
            Vec3::new(0.0, skeleton_attr.belt.0 + stop * 1.2, skeleton_attr.belt.1);
        next.belt.orientation = Quaternion::rotation_x(stop * 0.3);
        next.belt.scale = (Vec3::one() + slow_abs * 0.05) * 1.02;

        next.back.position = Vec3::new(0.0, skeleton_attr.back.0, skeleton_attr.back.1);

        next.shorts.position = Vec3::new(
            0.0,
            skeleton_attr.shorts.0 + stop * 2.5,
            skeleton_attr.shorts.1 + stop * 0.6,
        );
        next.shorts.orientation = Quaternion::rotation_x(stop * 0.6);

        next.hand_l.position = Vec3::new(
            -skeleton_attr.hand.0,
            skeleton_attr.hand.1 + slowa * 0.15,
            skeleton_attr.hand.2 + slow * 0.7 + stop * -2.0,
        );
        next.hand_l.orientation = Quaternion::rotation_x(slowa * -0.1 + slow * 0.1);
        next.hand_l.scale = Vec3::one() + slow_abs * -0.05;

        next.hand_r.position = Vec3::new(
            skeleton_attr.hand.0,
            skeleton_attr.hand.1 + slowa * 0.15,
            skeleton_attr.hand.2 + slow * 0.7 + stop * -2.0,
        );
        next.hand_r.orientation = Quaternion::rotation_x(slow * -0.1 + slowa * 0.1);
        next.hand_r.scale = Vec3::one() + slow_abs * -0.05;

        next.foot_l.position = Vec3::new(
            -skeleton_attr.foot.0,
            4.0 + skeleton_attr.foot.1,
            3.0 + skeleton_attr.foot.2,
        );
        next.foot_l.orientation = Quaternion::rotation_x(slow * 0.1 + stop * 1.2 + slow * 0.1);

        next.foot_r.position = Vec3::new(
            skeleton_attr.foot.0,
            4.0 + skeleton_attr.foot.1,
            3.0 + skeleton_attr.foot.2,
        );
        next.foot_r.orientation = Quaternion::rotation_x(slowa * 0.1 + stop * 1.2 + slowa * 0.1);

        next.shoulder_l.position = Vec3::new(
            -skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.shoulder_l.orientation = Quaternion::rotation_x(0.0);
        next.shoulder_l.scale = (Vec3::one() + slow_abs * -0.05) * 1.15;

        next.shoulder_r.position = Vec3::new(
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.shoulder_r.orientation = Quaternion::rotation_x(0.0);
        next.shoulder_r.scale = (Vec3::one() + slow_abs * -0.05) * 1.15;

        match active_tool_kind {
            Some(ToolKind::Dagger(_)) => {
                next.main.position = Vec3::new(-4.0, -5.0, 7.0);
                next.main.orientation =
                    Quaternion::rotation_y(0.25 * PI) * Quaternion::rotation_z(1.5 * PI);
            },
            Some(ToolKind::Shield(_)) => {
                next.main.position = Vec3::new(-0.0, -5.0, 3.0);
                next.main.orientation =
                    Quaternion::rotation_y(0.25 * PI) * Quaternion::rotation_z(-1.5 * PI);
            },
            _ => {
                next.main.position = Vec3::new(-7.0, -5.0, 15.0);
                next.main.orientation = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
            },
        }
        next.main.scale = Vec3::one();

        match second_tool_kind {
            Some(ToolKind::Dagger(_)) => {
                next.second.position = Vec3::new(4.0, -6.0, 7.0);
                next.second.orientation =
                    Quaternion::rotation_y(-0.25 * PI) * Quaternion::rotation_z(-1.5 * PI);
            },
            Some(ToolKind::Shield(_)) => {
                next.second.position = Vec3::new(0.0, -4.0, 3.0);
                next.second.orientation =
                    Quaternion::rotation_y(-0.25 * PI) * Quaternion::rotation_z(1.5 * PI);
            },
            _ => {
                next.second.position = Vec3::new(-7.0, -5.0, 15.0);
                next.second.orientation =
                    Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
            },
        }

        next.torso.position = Vec3::new(0.0, -0.2, stop * -0.16) * skeleton_attr.scaler;

        next.second.scale = match (
            active_tool_kind.map(|tk| tk.hands()),
            second_tool_kind.map(|tk| tk.hands()),
        ) {
            (Some(Hands::OneHand), Some(Hands::OneHand)) => Vec3::one(),
            (_, _) => Vec3::zero(),
        };

        next
    }
}
