use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::{AbilityInfo, StageSection},
};
use core::f32::consts::PI;

pub struct SpinAnimation;

type SpinAnimationDependency = (
    (Option<Hands>, Option<Hands>),
    Vec3<f32>,
    f32,
    Option<StageSection>,
    Option<AbilityInfo>,
);
impl Animation for SpinAnimation {
    type Dependency<'a> = SpinAnimationDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_spin\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_spin")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (hands, _velocity, _global_time, stage_section, ability_info): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (movement1base, movement2base, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - movement3;
        let move1 = movement1base * pullback;
        let move2 = movement2base * pullback;
        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation = Quaternion::rotation_z(0.0);
        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_x(0.0);
        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

        if matches!(
            stage_section,
            Some(StageSection::Action | StageSection::Recover)
        ) {
            next.main_weapon_trail = true;
            next.off_weapon_trail = true;
        }
        match ability_info.and_then(|a| a.tool) {
            Some(ToolKind::Sword) => {
                next.head.position = Vec3::new(
                    0.0 + 2.0 + move2 * -2.0,
                    2.0 + move2 * -2.0 + s_a.head.0,
                    s_a.head.1,
                );

                next.chest.orientation = Quaternion::rotation_x(move2 * -0.15)
                    * Quaternion::rotation_y(move1 * -0.1 + move2 * 0.15)
                    * Quaternion::rotation_z(-1.0 + move1 * -0.6 + move2 * 1.6);

                next.belt.orientation =
                    Quaternion::rotation_x(move1 * 0.1) * Quaternion::rotation_z(move2.sin() * 0.5);

                next.shorts.orientation =
                    Quaternion::rotation_x(move1 * 0.1) * Quaternion::rotation_z(move2.sin() * 1.5);

                next.head.orientation = Quaternion::rotation_x(move2 * 0.15)
                    * Quaternion::rotation_z(1.07 + move1 * 0.4 + move2 * -1.5);
            },

            Some(ToolKind::Axe) => {
                let (movement1, movement2, movement3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                    _ => (0.0, 0.0, 0.0),
                };

                next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);

                next.chest.orientation = Quaternion::rotation_x(0.4 + movement2 * -0.5)
                    * Quaternion::rotation_y(movement1 * -0.1 + movement2 * 0.0)
                    * Quaternion::rotation_z(0.5 + movement1 * -0.6 + movement2 * 0.6);

                next.belt.orientation = Quaternion::rotation_x(movement1 * -0.2 + movement2 * 0.2);

                next.shorts.orientation =
                    Quaternion::rotation_x(movement1 * -0.2 + movement2 * 0.2);

                next.head.orientation = Quaternion::rotation_y(movement1 * 0.0 + movement3 * -0.0)
                    * Quaternion::rotation_z(1.0 + movement1 * -0.5 + movement2 * 0.0);
                next.torso.position = Vec3::new(
                    0.0,
                    0.0,
                    -11.0
                        + 11.0 * (movement1 * 0.5 * PI).sin()
                        + 11.0 * (movement2 * 0.5 * PI + 0.5 * PI).sin(),
                );
                next.torso.orientation =
                    Quaternion::rotation_z(movement1.powi(2) * -6.0 + movement2 * -1.7);

                next.foot_l.position = Vec3::new(
                    -s_a.foot.0 + (movement1 * -1.0 + movement2 * -3.0),
                    s_a.foot.1,
                    s_a.foot.2 + (movement2 * 6.0),
                );
                next.foot_l.orientation = Quaternion::rotation_x(movement1 * 0.2 + movement2 * 0.5)
                    * Quaternion::rotation_y(movement2 * 0.5);

                next.foot_r.position = Vec3::new(
                    s_a.foot.0,
                    s_a.foot.1 + (movement1 * -2.0 + movement2 * -3.0),
                    s_a.foot.2,
                );
                next.foot_r.orientation =
                    Quaternion::rotation_x(movement1 * -0.5 + movement2 * -0.5);
                next.head.orientation = Quaternion::rotation_x(movement2 * 0.25)
                    * Quaternion::rotation_z(movement2 * 0.8);
            },
            _ => {},
        }
        match hands {
            (Some(Hands::Two), _) | (None, Some(Hands::Two)) => {
                match ability_info.and_then(|a| a.tool) {
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
                            s_a.sc.0 + movement1base * 2.0 + movement2base * -7.0,
                            s_a.sc.1 + 8.0 + movement1base * 0.6 + movement2base * -15.0,
                            s_a.sc.2 + 1.0 + movement1base * 0.6 + movement2base * 1.5,
                        );
                        next.control.orientation =
                            Quaternion::rotation_x(-0.5 + s_a.sc.3 + movement1base * -1.2)
                                * Quaternion::rotation_y(s_a.sc.4 - 0.6 + movement2base * -0.2)
                                * Quaternion::rotation_z(s_a.sc.5 - PI / 2.0 + movement1base * PI);
                    },
                    Some(ToolKind::Axe) => {
                        next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                        next.hand_l.orientation =
                            Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                        next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                        next.hand_r.orientation =
                            Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);
                        let (move1, move2, _move3) = match stage_section {
                            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                            Some(StageSection::Action) => (1.0, anim_time, 0.0),
                            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                            _ => (0.0, 0.0, 0.0),
                        };
                        next.control.position = Vec3::new(
                            s_a.ac.0 + move1 * -1.0 + move2 * -2.0,
                            s_a.ac.1 + move1 * -3.0 + move2 * 3.0,
                            s_a.ac.2 + move1 * 6.0 + move2 * -15.0,
                        );
                        next.control.orientation =
                            Quaternion::rotation_x(s_a.ac.3 + move1 * 0.0 + move2 * -3.0)
                                * Quaternion::rotation_y(s_a.ac.4 + move1 * -0.0 + move2 * -0.4)
                                * Quaternion::rotation_z(s_a.ac.5 + move1 * -2.0 + move2 * -1.0)
                    },
                    _ => {},
                }
            },
            (_, _) => {},
        };

        match hands {
            (Some(Hands::One), _) => match ability_info.and_then(|a| a.tool) {
                Some(ToolKind::Sword) => {
                    next.control_l.position = Vec3::new(-7.0 + movement2base * -5.0, 8.0, 2.0);
                    next.control_l.orientation = Quaternion::rotation_x(1.7)
                        * Quaternion::rotation_y(-2.7 + movement1base * -1.0 + movement2base * 2.0)
                        * Quaternion::rotation_z(1.5 + movement1base * PI);
                    next.hand_l.position = Vec3::new(0.0, -0.5, 0.0);
                    next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
                },
                Some(ToolKind::Axe) => {
                    next.control_l.position = Vec3::new(
                        -2.0 + movement1base * -5.0,
                        18.0 + movement1base * -10.0,
                        6.0 + movement1base * -10.0,
                    );
                    next.control_l.orientation = Quaternion::rotation_x(1.7 + movement2base * 1.5)
                        * Quaternion::rotation_y(-3.7)
                        * Quaternion::rotation_z(1.5 + movement2base * PI / 2.0);
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
                        next.control_r.position =
                            Vec3::new(15.0 + move2 * -15.0, 8.0 + move2 * 5.0, 2.0);
                        next.control_r.orientation = Quaternion::rotation_x(1.7)
                            * Quaternion::rotation_y(-3.3 + move1 * -1.0 + move2 * 2.0)
                            * Quaternion::rotation_z(1.5 + move1 * PI);
                        next.hand_r.position = Vec3::new(0.0, -0.5, 0.0);
                        next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
                    },
                    Some(ToolKind::Axe) => {
                        next.control_r.position = Vec3::new(
                            12.0 + move1 * -10.0,
                            18.0 + move1 * -10.0,
                            4.0 + move1 * -2.0,
                        );
                        next.control_r.orientation = Quaternion::rotation_x(1.7 + move2 * 1.5)
                            * Quaternion::rotation_y(-3.3)
                            * Quaternion::rotation_z(1.5 + move2 * PI / 2.0);
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
