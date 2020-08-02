use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::{Hands, ToolKind};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct Input {
    pub attack: bool,
}
pub struct BlockAnimation;

impl Animation for BlockAnimation {
    type Dependency = (Option<ToolKind>, Option<ToolKind>, f64);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_block\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_block")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave_ultra_slow = (anim_time as f32 * 3.0 + PI).sin();
        let wave_ultra_slow_cos = (anim_time as f32 * 3.0 + PI).cos();
        let wave_slow_cos = (anim_time as f32 * 6.0 + PI).cos();
        let _wave_slow = (anim_time as f32 * 6.0 + PI).sin();

        let _head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 1.5)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.3,
            ((global_time + anim_time) as f32 / 1.5)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.15,
        );
        next.head.offset = Vec3::new(
            0.0 + wave_slow_cos * 0.2,
            -1.0 + skeleton_attr.head.0,
            skeleton_attr.head.1 + 19.5 + wave_ultra_slow * 0.2,
        );
        next.head.ori = Quaternion::rotation_x(-0.25);
        next.head.scale = Vec3::one() * 1.01 * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(0.0 + wave_slow_cos * 0.2, 0.0, 5.0 + wave_ultra_slow * 0.2);
        next.chest.ori =
            Quaternion::rotation_x(-0.15) * Quaternion::rotation_y(wave_ultra_slow_cos * 0.01);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0 + wave_slow_cos * 0.2, 0.0, 3.0 + wave_ultra_slow * 0.2);
        next.belt.ori =
            Quaternion::rotation_x(0.0) * Quaternion::rotation_y(wave_ultra_slow_cos * 0.008);
        next.belt.scale = Vec3::one() * 1.01;

        next.shorts.offset = Vec3::new(0.0 + wave_slow_cos * 0.2, 0.0, 1.0 + wave_ultra_slow * 0.2);
        next.shorts.ori = Quaternion::rotation_x(0.1);
        next.shorts.scale = Vec3::one();

        match active_tool_kind {
            //TODO: Inventory
            Some(ToolKind::Sword(_)) => {
                next.l_hand.offset = Vec3::new(0.0, -5.0, -5.0);
                next.l_hand.ori = Quaternion::rotation_x(1.27);
                next.l_hand.scale = Vec3::one() * 1.04;
                next.r_hand.offset = Vec3::new(0.0, -6.0, -8.0);
                next.r_hand.ori = Quaternion::rotation_x(1.27);
                next.r_hand.scale = Vec3::one() * 1.05;
                next.main.offset = Vec3::new(0.0, 0.0, -6.0);
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.main.scale = Vec3::one();

                next.control.offset = Vec3::new(-8.0, 13.0, 8.0);
                next.control.ori = Quaternion::rotation_x(0.2)
                    * Quaternion::rotation_y(0.4)
                    * Quaternion::rotation_z(-1.57);
                next.control.scale = Vec3::one();
            },
            Some(ToolKind::Axe(_)) => {
                next.l_hand.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    3.5 + wave_ultra_slow_cos * 0.5,
                    0.0 + wave_ultra_slow * 1.0,
                );
                next.l_hand.ori = Quaternion::rotation_x(-0.3);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    3.0 + wave_ultra_slow_cos * 0.5,
                    -2.0 + wave_ultra_slow * 1.0,
                );
                next.r_hand.ori = Quaternion::rotation_x(-0.3);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.main.offset = Vec3::new(-6.0, 4.5, 0.0 + wave_ultra_slow * 1.0);
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.main.scale = Vec3::one();
            },
            Some(ToolKind::Hammer(_)) => {
                next.l_hand.offset = Vec3::new(-7.0, 3.5, 6.5);
                next.l_hand.ori = Quaternion::rotation_x(2.07)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(-0.2);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(7.0, 2.5, 3.75);
                next.r_hand.ori = Quaternion::rotation_x(2.07)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(-0.2);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.main.offset = Vec3::new(5.0, 8.75, 5.5);
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(-1.35)
                    * Quaternion::rotation_z(-0.85);
                next.main.scale = Vec3::one();
            },
            Some(ToolKind::Staff(_)) => {
                next.l_hand.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    3.5 + wave_ultra_slow_cos * 0.5,
                    0.0 + wave_ultra_slow * 1.0,
                );
                next.l_hand.ori = Quaternion::rotation_x(-0.3);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    3.0 + wave_ultra_slow_cos * 0.5,
                    -2.0 + wave_ultra_slow * 1.0,
                );
                next.r_hand.ori = Quaternion::rotation_x(-0.3);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.main.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    4.5 + wave_ultra_slow_cos * 0.5,
                    0.0 + wave_ultra_slow * 1.0,
                );
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.main.scale = Vec3::one();
            },
            Some(ToolKind::Shield(_)) => {
                next.l_hand.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    3.5 + wave_ultra_slow_cos * 0.5,
                    0.0 + wave_ultra_slow * 1.0,
                );
                next.l_hand.ori = Quaternion::rotation_x(-0.3);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    3.0 + wave_ultra_slow_cos * 0.5,
                    -2.0 + wave_ultra_slow * 1.0,
                );
                next.r_hand.ori = Quaternion::rotation_x(-0.3);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.main.offset = Vec3::new(
                    -6.0 + wave_ultra_slow_cos * 1.0,
                    4.5 + wave_ultra_slow_cos * 0.5,
                    0.0 + wave_ultra_slow * 1.0,
                );
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.main.scale = Vec3::one();
            },
            _ => {},
        }

        next.l_shoulder.offset = Vec3::new(
            -skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.glider.offset = Vec3::new(0.0, 5.0, 0.0);
        next.glider.ori = Quaternion::rotation_y(0.0);
        next.glider.scale = Vec3::one() * 0.0;

        next.lantern.offset = Vec3::new(
            skeleton_attr.lantern.0,
            skeleton_attr.lantern.1,
            skeleton_attr.lantern.2,
        );
        next.lantern.ori = Quaternion::rotation_x(0.0);
        next.lantern.scale = Vec3::one() * 0.0;

        next.torso.offset = Vec3::new(0.0, -0.2, 0.1) * skeleton_attr.scaler;
        next.torso.ori = Quaternion::rotation_x(0.0);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

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
