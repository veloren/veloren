use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use std::f32::consts::PI;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency = (Option<ToolKind>, Option<ToolKind>, f64);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_idle")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, _global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave_ultra_slow = (anim_time as f32 * 1.0).sin();
        let wave_ultra_slow_cos = (anim_time as f32 * 1.0 + PI / 2.0).sin();
        let head_abs = ((anim_time as f32 * 0.5 + PI).sin()) + 1.0;

        next.head.position = Vec3::new(
            0.0,
            -2.0 + skeleton_attr.head.0,
            skeleton_attr.head.1 + wave_ultra_slow * 0.1 + head_abs * -0.5,
        );

        next.head.scale = Vec3::one() * skeleton_attr.head_scale - head_abs * 0.05;

        next.chest.position = Vec3::new(
            0.0,
            skeleton_attr.chest.0,
            skeleton_attr.chest.1 + wave_ultra_slow * 0.1,
        );
        next.chest.scale = Vec3::one() + head_abs * 0.05;

        next.belt.position = Vec3::new(
            0.0,
            skeleton_attr.belt.0,
            skeleton_attr.belt.1 + wave_ultra_slow * 0.1,
        );
        next.belt.orientation = Quaternion::rotation_x(0.0);
        next.belt.scale = Vec3::one() - head_abs * 0.05;

        next.shorts.position = Vec3::new(
            0.0,
            skeleton_attr.shorts.0,
            skeleton_attr.shorts.1 + wave_ultra_slow * 0.1,
        );
        next.shorts.orientation = Quaternion::rotation_x(0.0);
        next.shorts.scale = Vec3::one();

        next.back.position = Vec3::new(0.0, skeleton_attr.back.0, skeleton_attr.back.1);
        next.back.scale = Vec3::one() * 1.02;

        next.hand_l.position = Vec3::new(
            -skeleton_attr.hand.0,
            skeleton_attr.hand.1 + wave_ultra_slow_cos * 0.15,
            skeleton_attr.hand.2 + wave_ultra_slow * 0.5,
        );

        next.hand_l.orientation = Quaternion::rotation_x(0.0 + wave_ultra_slow * -0.06);
        next.hand_l.scale = Vec3::one();

        next.hand_r.position = Vec3::new(
            skeleton_attr.hand.0,
            skeleton_attr.hand.1 + wave_ultra_slow_cos * 0.15,
            skeleton_attr.hand.2 + wave_ultra_slow * 0.5 + head_abs * -0.05,
        );
        next.hand_r.orientation = Quaternion::rotation_x(0.0 + wave_ultra_slow * -0.06);
        next.hand_r.scale = Vec3::one() + head_abs * -0.05;

        next.foot_l.position = Vec3::new(
            -skeleton_attr.foot.0,
            skeleton_attr.foot.1,
            skeleton_attr.foot.2,
        );
        next.foot_l.scale = Vec3::one();

        next.foot_r.position = Vec3::new(
            skeleton_attr.foot.0,
            skeleton_attr.foot.1,
            skeleton_attr.foot.2,
        );
        next.foot_r.scale = Vec3::one();

        next.shoulder_l.position = Vec3::new(
            -skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.shoulder_l.orientation = Quaternion::rotation_x(0.0);
        next.shoulder_l.scale = (Vec3::one() + head_abs * -0.05) * 1.15;

        next.shoulder_r.position = Vec3::new(
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.shoulder_r.orientation = Quaternion::rotation_x(0.0);
        next.shoulder_r.scale = (Vec3::one() + head_abs * -0.05) * 1.15;

        next.glider.scale = Vec3::one() * 0.0;

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
        next.second.scale = Vec3::one();

        next.lantern.position = Vec3::new(
            skeleton_attr.lantern.0,
            skeleton_attr.lantern.1,
            skeleton_attr.lantern.2,
        );
        next.lantern.orientation = Quaternion::rotation_x(0.1) * Quaternion::rotation_y(0.1);
        next.lantern.scale = Vec3::one() * 0.65;
        next.hold.scale = Vec3::one() * 0.0;

        next.torso.position = Vec3::new(0.0, -0.2, 0.1) * skeleton_attr.scaler;
        next.torso.orientation = Quaternion::rotation_x(0.0);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next.control.position = Vec3::new(0.0, 0.0, 0.0);
        next.control.orientation = Quaternion::rotation_x(0.0);
        next.control.scale = Vec3::one();

        next.control_l.position = Vec3::new(0.0, 0.0, 0.0);
        next.control_l.orientation = Quaternion::rotation_x(0.0);
        next.control_l.scale = Vec3::one();

        next.control_r.position = Vec3::new(0.0, 0.0, 0.0);
        next.control_r.orientation = Quaternion::rotation_x(0.0);
        next.control_r.scale = Vec3::one();

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
