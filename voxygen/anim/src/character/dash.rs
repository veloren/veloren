use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::{AbilityInfo, StageSection},
};
use std::f32::consts::PI;

pub struct DashAnimation;

type DashAnimationDependency = (
    (Option<Hands>, Option<Hands>),
    f32,
    Option<StageSection>,
    Option<AbilityInfo>,
);
impl Animation for DashAnimation {
    type Dependency = DashAnimationDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_dash\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_dash")]
    #[allow(clippy::single_match)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (hands, _global_time, stage_section, ability_info): Self::Dependency,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (movement1, movement2, movement3, move4) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0, 0.0),
            Some(StageSection::Charge) => (1.0, anim_time, 0.0, 0.0),
            Some(StageSection::Swing) => (1.0, 1.0, anim_time.powf(0.01), 0.0),
            Some(StageSection::Recover) => (1.1, 1.0, 1.0, anim_time.powi(4)),
            _ => (0.0, 0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - move4;
        let move1 = movement1 * pullback;
        let move2 = movement2 * pullback;
        let move3 = movement3 * pullback;

        fn slow(x: f32) -> f32 {
            ((5.0 / (1.1 + 3.9 * ((x * 12.4).sin()).powi(2))).sqrt()) * ((x * 12.4).sin())
        }

        fn short(x: f32) -> f32 {
            ((5.0 / (1.5 + 3.5 * ((x * 5.0).sin()).powi(2))).sqrt()) * ((x * 5.0).sin())
        }

        fn shortalt(x: f32) -> f32 { (x * 5.0 + PI / 2.0).sin() }

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation = Quaternion::rotation_z(0.0);
        match ability_info.and_then(|a| a.tool) {
            Some(ToolKind::Sword) => {
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);

                next.head.position =
                    Vec3::new(0.0, 0.0 + s_a.head.0, s_a.head.1 + move2.min(1.0) * 1.0);
                next.head.orientation = Quaternion::rotation_y(move2.min(1.0) * -0.3 + move3 * 0.3)
                    * Quaternion::rotation_z(move1 * -0.9 + move3 * 1.6);

                next.chest.position = Vec3::new(
                    0.0,
                    s_a.chest.0,
                    s_a.chest.1 + (2.0 + shortalt(move2) * -2.5) + move3 * -3.0,
                );
                next.chest.orientation =
                    Quaternion::rotation_x(move2.min(1.0) * -0.4 + move3 * 0.4)
                        * Quaternion::rotation_y(move2.min(1.0) * -0.2 + move3 * 0.3)
                        * Quaternion::rotation_z(move1 * 1.1 + move3 * -2.2);

                next.shorts.orientation = Quaternion::rotation_z(short(move2).min(1.0) * 0.25);

                next.belt.orientation = Quaternion::rotation_z(short(move2).min(1.0) * 0.1);
            },
            _ => {},
        }

        next.lantern.orientation = Quaternion::rotation_x(slow(anim_time) * -0.7 + 0.4)
            * Quaternion::rotation_y(slow(anim_time) * 0.4);

        match hands {
            (Some(Hands::Two), _) => match ability_info.and_then(|a| a.tool) {
                Some(ToolKind::Sword) | Some(ToolKind::SwordSimple) => {
                    next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                    next.hand_l.orientation =
                        Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                    next.hand_r.position = Vec3::new(s_a.shr.0, s_a.shr.1, s_a.shr.2);
                    next.hand_r.orientation =
                        Quaternion::rotation_x(s_a.shr.3) * Quaternion::rotation_y(s_a.shr.4);

                    next.control.position = Vec3::new(
                        s_a.sc.0 + (move1 * -5.0 + move3 * -2.0),
                        s_a.sc.1 + (move2.min(1.0) * -2.0),
                        s_a.sc.2 + (move2.min(1.0) * 2.0),
                    );
                    next.control.orientation =
                        Quaternion::rotation_x(s_a.sc.3 + (move1 * -1.0 + move3 * -0.5))
                            * Quaternion::rotation_y(s_a.sc.4 + (move1 * 1.5 + move3 * -2.5));
                },
                _ => {},
            },
            (_, _) => {},
        };

        match hands {
            (Some(Hands::One), Some(Hands::One)) | (Some(Hands::One), None) => {
                match ability_info.and_then(|a| a.tool) {
                    Some(ToolKind::Sword) | Some(ToolKind::SwordSimple) => {
                        next.control_l.position =
                            Vec3::new(-7.0, 8.0 + move3 * 5.0, 2.0 + move1 * 4.0);
                        next.control_l.orientation =
                            Quaternion::rotation_x(-0.3 + move2 * 1.0 + move3 * 1.0)
                                * Quaternion::rotation_y(move1 * -1.2 + move3 * -1.5)
                                * Quaternion::rotation_z(move2 * 1.0 + move3 * 1.5);
                        next.hand_l.position = Vec3::new(0.0, -0.5, 0.0);
                        next.hand_l.orientation = Quaternion::rotation_x(1.57)
                    },

                    _ => {},
                }
            },
            (_, _) => {},
        };
        match hands {
            (Some(Hands::One), Some(Hands::One)) | (None, Some(Hands::One)) => {
                match ability_info.and_then(|a| a.tool) {
                    Some(ToolKind::Sword) | Some(ToolKind::SwordSimple) => {
                        next.control_r.position = Vec3::new(
                            7.0 + move1 * 5.0 + move3 * -30.0,
                            8.0 + move3 * -5.0,
                            2.0 + move1 * 1.0,
                        );
                        next.control_r.orientation =
                            Quaternion::rotation_x(-0.3 + move1 * -3.0 + move3 * -0.5)
                                * Quaternion::rotation_y(
                                    move1 * 1.5 + (move2 * 1.0).min(0.8) + move3 * 1.5,
                                )
                                * Quaternion::rotation_z(move3 * 1.5);
                        next.hand_r.position = Vec3::new(0.0, -0.5, 0.0);
                        next.hand_r.orientation = Quaternion::rotation_x(1.57)
                    },
                    _ => {},
                }
            },
            (_, _) => {},
        };

        match hands {
            (None, None) | (None, Some(Hands::One)) => {
                next.hand_l.position = Vec3::new(-4.5, 8.0, 5.0);
                next.hand_l.orientation = Quaternion::rotation_x(1.9) * Quaternion::rotation_y(-0.5)
            },
            (_, _) => {},
        };
        match hands {
            (None, None) | (Some(Hands::One), None) => {
                next.hand_r.position = Vec3::new(4.5, 8.0, 5.0);
                next.hand_r.orientation = Quaternion::rotation_x(1.9) * Quaternion::rotation_y(0.5)
            },
            (_, _) => {},
        };

        next
    }
}
