use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::{AbilityInfo, StageSection},
};
use core::f32::consts::PI;

pub struct LeapAnimation;

type LeapAnimationDependency = (
    (Option<Hands>, Option<Hands>),
    Vec3<f32>,
    f32,
    Option<StageSection>,
    Option<AbilityInfo>,
);
impl Animation for LeapAnimation {
    type Dependency<'a> = LeapAnimationDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_leapmelee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_leapmelee")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (hands, _velocity, _global_time, stage_section, ability_info): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (movement1, movement2, movement3, move4) = match stage_section {
            Some(StageSection::Buildup) => (anim_time, 0.0, 0.0, 0.0),
            Some(StageSection::Movement) => (1.0, anim_time.powi(2), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, 1.0, anim_time.powf(0.75), 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time.powf(0.75)),
            _ => (0.0, 0.0, 0.0, 0.0),
        };
        if matches!(
            stage_section,
            Some(StageSection::Movement | StageSection::Action | StageSection::Recover)
        ) {
            next.main_weapon_trail = true;
            next.off_weapon_trail = true;
        }
        let pullback = 1.0 - move4;
        let move1 = movement1 * pullback;
        let move2 = movement2 * pullback;
        let move3 = movement3 * pullback;

        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation = Quaternion::rotation_z(0.0);
        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_z(0.0);
        next.torso.position = Vec3::new(0.0, 0.0, 1.1);
        next.torso.orientation = Quaternion::rotation_z(0.0);

        match ability_info.and_then(|a| a.tool) {
            Some(ToolKind::Hammer) => {
                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

                next.chest.orientation = Quaternion::rotation_x(move2 * 0.4 + move3 * -1.5)
                    * Quaternion::rotation_z(move1 * 0.5 + move2 * 0.2 + move3 * -0.7);

                next.head.orientation = Quaternion::rotation_x(move3 * 0.2)
                    * Quaternion::rotation_y(move2 * -0.1)
                    * Quaternion::rotation_z(move1 * -0.4 + move2 * -0.2 + move3 * 0.6);

                next.foot_l.position = Vec3::new(
                    -s_a.foot.0,
                    s_a.foot.1 + move3 * 13.0,
                    s_a.foot.2 + move3 * -2.0,
                );
                next.foot_l.orientation = Quaternion::rotation_x(-0.8 + move3 * 1.7);

                next.foot_r.position = Vec3::new(
                    s_a.foot.0,
                    s_a.foot.1 + move2 * 8.0 + move3 * -13.0,
                    s_a.foot.2 + move2 * 5.0 + move3 * -5.0,
                );
                next.foot_r.orientation = Quaternion::rotation_x(0.9 + move3 * -1.7);
                next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + move3 * -5.0);
            },
            Some(ToolKind::Axe) => {
                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

                next.torso.orientation =
                    Quaternion::rotation_x(move1 * 0.3 + move2 * 0.5 + move3 * -1.0)
                        * Quaternion::rotation_z(move1 * 0.4 + move2 * 0.4);

                next.head.orientation = Quaternion::rotation_x(move2 * -0.6 + move3 * 0.8)
                    * Quaternion::rotation_z(move2 * -0.4);

                next.foot_l.position = Vec3::new(
                    -s_a.foot.0,
                    s_a.foot.1 + move2 * -4.0 + move3 * 4.0,
                    s_a.foot.2 + move2 * 5.0 + move3 * -5.0,
                );

                next.foot_r.position = Vec3::new(
                    s_a.foot.0,
                    s_a.foot.1 + move2 * 4.0,
                    s_a.foot.2 + move3 * -3.0,
                );

                next.foot_l.orientation =
                    Quaternion::rotation_x(move1 * 0.9 - move2 * 1.9 + move3 * 1.8);

                next.foot_r.orientation = Quaternion::rotation_x(move1 * 0.9 - move3 * 1.8);

                next.belt.orientation = Quaternion::rotation_x(move1 * 0.22 + move2 * 0.1);
                next.shorts.orientation = Quaternion::rotation_x(move1 * 0.3 + move2 * 0.1);

                next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);
            },
            _ => {},
        }

        match hands {
            (Some(Hands::Two), _) | (None, Some(Hands::Two)) => {
                match ability_info.and_then(|a| a.tool) {
                    Some(ToolKind::Hammer) => {
                        next.hand_l.position = Vec3::new(s_a.hhl.0, s_a.hhl.1, s_a.hhl.2);
                        next.hand_l.orientation =
                            Quaternion::rotation_x(s_a.hhl.3) * Quaternion::rotation_z(s_a.hhl.5);
                        next.hand_r.position = Vec3::new(s_a.hhr.0, s_a.hhr.1, s_a.hhr.2);
                        next.hand_r.orientation =
                            Quaternion::rotation_x(s_a.hhr.3) * Quaternion::rotation_z(s_a.hhr.5);
                        next.main.position = Vec3::new(0.0, 0.0, 0.0);
                        next.main.orientation =
                            Quaternion::rotation_y(0.0) * Quaternion::rotation_z(0.0);
                        next.control.position = Vec3::new(
                            s_a.hc.0 + move2 * -10.0 + move3 * 10.0,
                            s_a.hc.1 + move2 * 5.0 + move3 * 7.0,
                            s_a.hc.2 + move2 * 5.0 + move3 * -10.0,
                        );
                        next.control.orientation =
                            Quaternion::rotation_x(s_a.hc.3 + move2 * PI / 2.0 + move3 * -2.3)
                                * Quaternion::rotation_y(s_a.hc.4 + move2 * 1.3)
                                * Quaternion::rotation_z(s_a.hc.5 + move2 * -1.0 + move3 * 0.5);
                    },
                    Some(ToolKind::Axe) => {
                        next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                        next.hand_l.orientation = Quaternion::rotation_x(s_a.ahl.3);
                        next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                        next.hand_r.orientation = Quaternion::rotation_x(s_a.ahr.3);
                        next.main.position = Vec3::new(0.0, 0.0, 0.0);
                        next.main.orientation =
                            Quaternion::rotation_y(0.0) * Quaternion::rotation_z(0.0);

                        next.control.position = Vec3::new(
                            s_a.ac.0 + move2 * 8.0 + move3 * 15.0,
                            s_a.ac.1 + move3 * -10.0,
                            s_a.ac.2 + move3 * 4.0,
                        );
                        next.control.orientation = Quaternion::rotation_x(s_a.ac.3)
                            * Quaternion::rotation_y(s_a.ac.4 + move2 * -0.8 + move3 * -4.0)
                            * Quaternion::rotation_z(s_a.ac.5 + move2 * -0.6 + move3 * -1.6);
                    },
                    _ => {},
                }
            },
            (_, _) => {},
        };

        match hands {
            (Some(Hands::One), _) => match ability_info.and_then(|a| a.tool) {
                Some(ToolKind::Axe) => {
                    next.control_l.position =
                        Vec3::new(-7.0 + move3 * 4.0, 8.0 + move3 * 8.0, 2.0 + move3 * -4.0);
                    next.control_l.orientation =
                        Quaternion::rotation_x(-0.3 + move2 * 1.0 + move3 * -2.0)
                            * Quaternion::rotation_y(move2 * -0.5 + move3 * 1.9);
                    next.hand_l.position = Vec3::new(0.0, -0.5, 0.0);
                    next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
                },
                Some(ToolKind::Hammer) | Some(ToolKind::Pick) => {
                    next.control_l.position = Vec3::new(
                        -7.0,
                        8.0 + move2 * -5.0 + move3 * 9.0,
                        2.0 + move2 * 8.0 + move3 * -12.0,
                    );
                    next.control_l.orientation =
                        Quaternion::rotation_x(-0.3 + move2 * 1.5 + move3 * -2.5);
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
                    Some(ToolKind::Axe) => {
                        next.control_r.position = Vec3::new(
                            7.0 + move2 * 2.0,
                            8.0 + move2 * -8.0 + move3 * 13.0,
                            2.0 + move2 * 7.0 + move3 * -10.0,
                        );
                        next.control_r.orientation = Quaternion::rotation_x(-0.3 + move3 * -2.2)
                            * Quaternion::rotation_y(move2 * -0.5 + move3 * 1.2);
                        next.hand_r.position = Vec3::new(0.0, -0.5, 0.0);
                        next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
                    },
                    Some(ToolKind::Hammer) | Some(ToolKind::Pick) => {
                        next.control_r.position = Vec3::new(
                            7.0 + move2 * 3.0 + move3 * -3.0,
                            8.0 + move2 * -9.0 + move3 * 15.0,
                            2.0 + move2 * 11.0 + move3 * -18.0,
                        );
                        next.control_r.orientation =
                            Quaternion::rotation_x(-0.3 + move2 * 1.5 + move3 * -2.5)
                                * Quaternion::rotation_y(move2 * -0.75 + move3 * 0.75);
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
