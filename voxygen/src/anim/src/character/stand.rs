use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use std::{f32::consts::PI, ops::Mul};

pub struct StandAnimation;

impl Animation for StandAnimation {
    type Dependency = (Option<ToolKind>, Option<ToolKind>, f64, Vec3<f32>);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_stand\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_stand")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, global_time, avg_vel): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let slow = (anim_time as f32 * 1.0).sin();
        let breathe = ((anim_time as f32 * 0.5).sin()).abs();
        let impact = (avg_vel.z).max(-15.0);
        let head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 12.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.3,
            ((global_time + anim_time) as f32 / 12.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.15,
        );
        next.head.offset = Vec3::new(
            0.0,
            -3.0 + skeleton_attr.head.0,
            skeleton_attr.head.1 + slow * 0.3 + breathe * -0.05,
        );
        next.head.ori = Quaternion::rotation_z(head_look.x)
            * Quaternion::rotation_x(impact * -0.02 + head_look.y.abs());
        next.head.scale = Vec3::one() * skeleton_attr.head_scale + breathe * -0.05;

        next.chest.offset = Vec3::new(
            0.0,
            skeleton_attr.chest.0,
            skeleton_attr.chest.1 + slow * 0.3 + impact * 0.2,
        );
        next.chest.ori =
            Quaternion::rotation_z(head_look.x * 0.6) * Quaternion::rotation_x(impact * 0.04);
        next.chest.scale = Vec3::one() * 1.01 + breathe * 0.03;

        next.belt.offset = Vec3::new(
            0.0,
            skeleton_attr.belt.0 + impact * 0.005,
            skeleton_attr.belt.1,
        );
        next.belt.ori =
            Quaternion::rotation_z(head_look.x * -0.1) * Quaternion::rotation_x(impact * -0.03);
        next.belt.scale = Vec3::one() + breathe * -0.03;

        next.back.offset = Vec3::new(0.0, skeleton_attr.back.0, skeleton_attr.back.1);
        next.back.scale = Vec3::one() * 1.02;

        next.shorts.offset = Vec3::new(
            0.0,
            skeleton_attr.shorts.0 + impact * -0.2,
            skeleton_attr.shorts.1,
        );
        next.shorts.ori =
            Quaternion::rotation_z(head_look.x * -0.2) * Quaternion::rotation_x(impact * -0.04);
        next.shorts.scale = Vec3::one() + breathe * -0.03;

        next.l_hand.offset = Vec3::new(
            -skeleton_attr.hand.0,
            skeleton_attr.hand.1 + slow * 0.15 - impact * 0.2,
            skeleton_attr.hand.2 + slow * 0.5 + impact * -0.1,
        );

        next.l_hand.ori = Quaternion::rotation_x(slow * -0.06 + impact * -0.1);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(
            skeleton_attr.hand.0,
            skeleton_attr.hand.1 + slow * 0.15 - impact * 0.2,
            skeleton_attr.hand.2 + slow * 0.5 + impact * -0.1,
        );
        next.r_hand.ori = Quaternion::rotation_x(slow * -0.06 + impact * -0.1);
        next.r_hand.scale = Vec3::one();

        next.l_foot.offset = Vec3::new(
            -skeleton_attr.foot.0,
            skeleton_attr.foot.1 - impact * 0.15,
            skeleton_attr.foot.2,
        );
        next.l_foot.ori = Quaternion::rotation_x(impact * 0.02);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(
            skeleton_attr.foot.0,
            skeleton_attr.foot.1 + impact * 0.15,
            skeleton_attr.foot.2,
        );
        next.r_foot.ori = Quaternion::rotation_x(impact * -0.02);
        next.r_foot.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(
            -skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.l_shoulder.scale = (Vec3::one() + breathe * -0.05) * 1.15;

        next.r_shoulder.offset = Vec3::new(
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.r_shoulder.scale = (Vec3::one() + breathe * -0.05) * 1.15;

        next.glider.offset = Vec3::new(0.0, 0.0, 10.0);
        next.glider.scale = Vec3::one() * 0.0;

        match active_tool_kind {
            Some(ToolKind::Dagger(_)) => {
                next.main.offset = Vec3::new(-4.0, -5.0, 7.0);
                next.main.ori =
                    Quaternion::rotation_y(0.25 * PI) * Quaternion::rotation_z(1.5 * PI);
            },
            Some(ToolKind::Shield(_)) => {
                next.main.offset = Vec3::new(-0.0, -5.0, 3.0);
                next.main.ori =
                    Quaternion::rotation_y(0.25 * PI) * Quaternion::rotation_z(-1.5 * PI);
            },
            _ => {
                next.main.offset = Vec3::new(-7.0, -5.0, 15.0);
                next.main.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
            },
        }
        next.main.scale = Vec3::one();

        match second_tool_kind {
            Some(ToolKind::Dagger(_)) => {
                next.second.offset = Vec3::new(4.0, -6.0, 7.0);
                next.second.ori =
                    Quaternion::rotation_y(-0.25 * PI) * Quaternion::rotation_z(-1.5 * PI);
            },
            Some(ToolKind::Shield(_)) => {
                next.second.offset = Vec3::new(0.0, -4.0, 3.0);
                next.second.ori =
                    Quaternion::rotation_y(-0.25 * PI) * Quaternion::rotation_z(1.5 * PI);
            },
            _ => {
                next.second.offset = Vec3::new(-7.0, -5.0, 15.0);
                next.second.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
            },
        }
        next.second.scale = Vec3::one();

        next.lantern.offset = Vec3::new(
            skeleton_attr.lantern.0,
            skeleton_attr.lantern.1,
            skeleton_attr.lantern.2,
        );
        next.lantern.ori = Quaternion::rotation_x(0.1) * Quaternion::rotation_y(0.1);
        next.lantern.scale = Vec3::one() * 0.65;

        next.torso.offset = Vec3::new(0.0, 0.0, 0.) * skeleton_attr.scaler;
        next.torso.ori = Quaternion::rotation_x(0.0);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next.control.scale = Vec3::one();

        next.l_control.scale = Vec3::one();

        next.r_control.scale = Vec3::one();

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
