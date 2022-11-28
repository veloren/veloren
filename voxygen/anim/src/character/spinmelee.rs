use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::{AbilityInfo, StageSection},
};
use core::f32::consts::PI;

pub struct SpinMeleeAnimation;

type SpinMeleeAnimationDependency = (
    (Option<Hands>, Option<Hands>),
    Vec3<f32>,
    f32,
    Option<StageSection>,
    Option<AbilityInfo>,
);
impl Animation for SpinMeleeAnimation {
    type Dependency<'a> = SpinMeleeAnimationDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_spinmelee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_spinmelee")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (hands, _velocity, _global_time, stage_section, ability_info): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let (movement1, movement2, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powf(2.0)),
            _ => (0.0, 0.0, 0.0),
        };

        let pullback = 1.0 - movement3;
        let move1 = movement1 * pullback;
        let move2 = movement2 * pullback;
        let mut next = (*skeleton).clone();
        if matches!(
            stage_section,
            Some(StageSection::Action | StageSection::Recover)
        ) {
            next.main_weapon_trail = true;
            next.off_weapon_trail = true;
        }
        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_z(0.0);
        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation = Quaternion::rotation_z(0.0);
        match ability_info.and_then(|a| a.tool) {
            Some(ToolKind::Sword) => {
                next.torso.orientation = Quaternion::rotation_z(movement2 * PI * 2.0);

                next.chest.position = Vec3::new(
                    0.0,
                    s_a.chest.0 + move1 * -2.0,
                    s_a.chest.1 + move1 * -2.0 + move2 * 2.0,
                );
                next.chest.orientation = Quaternion::rotation_x(move1 * -0.3);
                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
                next.head.orientation =
                    Quaternion::rotation_x(move1 * 0.2) * Quaternion::rotation_z(move2 * 0.8);
                next.belt.orientation = Quaternion::rotation_x(move1 * 0.5);
                next.shorts.orientation = Quaternion::rotation_x(move1 * 0.4);
                next.belt.position = Vec3::new(0.0, s_a.belt.0, s_a.belt.1 + move1 * 0.0);
                next.belt.orientation = Quaternion::rotation_x(0.15);
                next.shorts.position =
                    Vec3::new(0.0, s_a.shorts.0 + move1 * 2.0, s_a.shorts.1 + move1 * 1.0);

                next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1 + move1 * -6.0, s_a.foot.2);
                next.foot_r.orientation = Quaternion::rotation_x(move1 * -1.0);
            },

            Some(ToolKind::Axe) => {
                next.head.orientation =
                    Quaternion::rotation_x(move1 * -0.2) * Quaternion::rotation_z(move1 * 0.4);
                next.head.position = Vec3::new(0.0, s_a.head.0 + move1 * 2.0, s_a.head.1);

                next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + move1 * -1.0);
                next.chest.orientation =
                    Quaternion::rotation_x(move1 * 0.3) * Quaternion::rotation_y(move1 * 0.3);

                next.belt.position = Vec3::new(0.0, 1.0 + s_a.belt.0, s_a.belt.1 + move1 * 0.5);
                next.belt.orientation = Quaternion::rotation_x(0.15);
                next.shorts.position = Vec3::new(
                    0.0,
                    1.0 + s_a.shorts.0 + move1 * 1.0,
                    s_a.shorts.1 + move1 * 1.0,
                );
                next.shorts.orientation = Quaternion::rotation_x(0.15 + 0.15 * move1);

                next.torso.orientation =
                    Quaternion::rotation_z(movement1 * -0.5 + movement2 * -6.78);

                next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1 + move1 * 7.0, s_a.foot.2);
                next.foot_l.orientation = Quaternion::rotation_x(move1 * 0.8);

                next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1 + move1 * -3.0, s_a.foot.2);
                next.foot_r.orientation = Quaternion::rotation_x(move1 * -0.5);
            },
            _ => {},
        }
        match hands {
            (Some(Hands::Two), _) | (None, Some(Hands::Two)) => match ability_info
                .and_then(|a| a.tool)
            {
                Some(ToolKind::Sword) => {
                    next.main.position = Vec3::new(0.0, 0.0, 0.0);
                    next.main.orientation = Quaternion::rotation_x(0.0);

                    next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                    next.hand_l.orientation =
                        Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                    next.hand_r.position = Vec3::new(s_a.shr.0, s_a.shr.1, s_a.shr.2);
                    next.hand_r.orientation =
                        Quaternion::rotation_x(s_a.shr.3) * Quaternion::rotation_y(s_a.shr.4);

                    next.control.position = Vec3::new(
                        s_a.sc.0,
                        s_a.sc.1 + move1 * 4.0,
                        s_a.sc.2 + move1 * 2.0 + move2 * 10.0,
                    );
                    next.control.orientation = Quaternion::rotation_x(s_a.sc.3 + move1 * -PI / 2.5)
                        * Quaternion::rotation_z(s_a.sc.5 + move1 * -PI / 2.0);
                },
                Some(ToolKind::Axe) => {
                    next.main.position = Vec3::new(0.0, 0.0, 0.0);
                    next.main.orientation = Quaternion::rotation_x(0.0);
                    next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                    next.hand_l.orientation =
                        Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                    next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                    next.hand_r.orientation =
                        Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                    next.control.position =
                        Vec3::new(s_a.ac.0 + move1 * 8.0, s_a.ac.1, s_a.ac.2 + move1 * -4.0);
                    next.control.orientation =
                        Quaternion::rotation_x(s_a.ac.3 + move1 * -1.0 + move2 * 0.4)
                            * Quaternion::rotation_y(s_a.ac.4 + move1 * -PI)
                            * Quaternion::rotation_z(s_a.ac.5 + move1 * 1.4);
                },
                _ => {},
            },
            (_, _) => {},
        };
        match hands {
            (Some(Hands::One), _) => match ability_info.and_then(|a| a.tool) {
                Some(ToolKind::Sword) => {
                    next.control_l.position = Vec3::new(-7.0, 8.0, 2.0);
                    next.control_l.orientation = Quaternion::rotation_x(-0.3 + move1 * -0.5)
                        * Quaternion::rotation_z(move1 * PI / 2.0);
                    next.hand_l.position = Vec3::new(0.0, -0.5, 0.0);
                    next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
                },
                Some(ToolKind::Axe) => {
                    next.control_l.position = Vec3::new(-7.0, 8.0, 2.0);
                    next.control_l.orientation = Quaternion::rotation_x(-0.3 + move1 * -1.3)
                        * Quaternion::rotation_z(move1 * -PI / 2.0);
                    next.hand_l.position = Vec3::new(0.0, -0.5, 0.0);
                    next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
                },

                _ => {},
            },
            (_, _) => {},
        };
        match hands {
            (None | Some(Hands::One), Some(Hands::One)) => {
                match ability_info.and_then(|a| a.tool) {
                    Some(ToolKind::Sword) => {
                        next.control_r.position = Vec3::new(7.0, 8.0, 2.0 + move1 * 10.0);
                        next.control_r.orientation = Quaternion::rotation_x(-0.3 + move1 * -1.2)
                            * Quaternion::rotation_y(move1 * 0.8)
                            * Quaternion::rotation_z(move1 * PI / 2.0);
                        next.hand_r.position = Vec3::new(0.0, -0.5, 0.0);
                        next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
                    },
                    Some(ToolKind::Axe) => {
                        next.control_r.position = Vec3::new(7.0, 8.0, 2.0);
                        next.control_r.orientation = Quaternion::rotation_x(-0.3 + move1 * -1.6)
                            * Quaternion::rotation_z(move1 * -PI / 2.0);
                        next.hand_r.position = Vec3::new(0.0, -0.5, 0.0);
                        next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
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

        if let (None, Some(Hands::Two)) = hands {
            next.second = next.main;
        }

        next
    }
}
