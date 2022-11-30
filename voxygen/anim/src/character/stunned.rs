use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::StageSection,
};
use core::f32::consts::PI;

pub struct StunnedAnimation;

type StunnedAnimationDependency = (
    Option<ToolKind>,
    Option<ToolKind>,
    (Option<Hands>, Option<Hands>),
    f32,
    f32,
    Option<StageSection>,
    f32,
    bool,
);
impl Animation for StunnedAnimation {
    type Dependency<'a> = StunnedAnimationDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_stunned\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_stunned")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            active_tool_kind,
            _second_tool_kind,
            hands,
            _velocity,
            global_time,
            stage_section,
            timer,
            wield_status,
        ): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (movement1base, movement2) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0),
            Some(StageSection::Recover) => (1.0, anim_time.powf(4.0)),
            _ => (0.0, 0.0),
        };
        let pullback = 1.0 - movement2;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let movement1 = movement1base * pullback * mirror;
        let movement1abs = movement1base * pullback;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_z(movement1 * -0.3);
        next.shorts.orientation =
            Quaternion::rotation_x(movement1abs * -0.2) * Quaternion::rotation_z(movement1 * -0.3);
        next.belt.orientation =
            Quaternion::rotation_x(movement1abs * -0.1) * Quaternion::rotation_z(movement1 * -0.2);

        next.chest.orientation =
            Quaternion::rotation_x(movement1abs * 0.3) * Quaternion::rotation_z(movement1 * 1.0);
        if wield_status {
            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_x(0.0);
            next.second.position = Vec3::new(0.0, 0.0, 0.0);
            next.second.orientation = Quaternion::rotation_z(0.0);
            match hands {
                (Some(Hands::Two), _) | (None, Some(Hands::Two)) => match active_tool_kind {
                    Some(ToolKind::Sword) => {
                        next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                        next.hand_l.orientation =
                            Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                        next.hand_r.position = Vec3::new(s_a.shr.0, s_a.shr.1, s_a.shr.2);
                        next.hand_r.orientation =
                            Quaternion::rotation_x(s_a.shr.3) * Quaternion::rotation_y(s_a.shr.4);

                        next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                        next.control.orientation = Quaternion::rotation_x(s_a.sc.3);
                    },
                    Some(ToolKind::Axe) => {
                        next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                        next.hand_l.orientation =
                            Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                        next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                        next.hand_r.orientation =
                            Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                        next.control.position = Vec3::new(s_a.ac.0, s_a.ac.1, s_a.ac.2);
                        next.control.orientation = Quaternion::rotation_x(s_a.ac.3)
                            * Quaternion::rotation_y(s_a.ac.4)
                            * Quaternion::rotation_z(s_a.ac.5);
                    },
                    Some(ToolKind::Hammer | ToolKind::Pick) => {
                        next.hand_l.position = Vec3::new(s_a.hhl.0, s_a.hhl.1, s_a.hhl.2);
                        next.hand_l.orientation = Quaternion::rotation_x(s_a.hhl.3)
                            * Quaternion::rotation_y(s_a.hhl.4)
                            * Quaternion::rotation_z(s_a.hhl.5);
                        next.hand_r.position = Vec3::new(s_a.hhr.0, s_a.hhr.1, s_a.hhr.2);
                        next.hand_r.orientation = Quaternion::rotation_x(s_a.hhr.3)
                            * Quaternion::rotation_y(s_a.hhr.4)
                            * Quaternion::rotation_z(s_a.hhr.5);

                        next.control.position = Vec3::new(s_a.hc.0, s_a.hc.1, s_a.hc.2);
                        next.control.orientation = Quaternion::rotation_x(s_a.hc.3)
                            * Quaternion::rotation_y(s_a.hc.4)
                            * Quaternion::rotation_z(s_a.hc.5);
                    },
                    Some(ToolKind::Staff) | Some(ToolKind::Sceptre) => {
                        next.hand_r.position = Vec3::new(s_a.sthr.0, s_a.sthr.1, s_a.sthr.2);
                        next.hand_r.orientation =
                            Quaternion::rotation_x(s_a.sthr.3) * Quaternion::rotation_y(s_a.sthr.4);

                        next.control.position = Vec3::new(s_a.stc.0, s_a.stc.1, s_a.stc.2);

                        next.hand_l.position = Vec3::new(s_a.sthl.0, s_a.sthl.1, s_a.sthl.2);
                        next.hand_l.orientation = Quaternion::rotation_x(s_a.sthl.3);

                        next.control.orientation = Quaternion::rotation_x(s_a.stc.3)
                            * Quaternion::rotation_y(s_a.stc.4)
                            * Quaternion::rotation_z(s_a.stc.5);
                    },
                    Some(ToolKind::Bow) => {
                        next.hand_l.position = Vec3::new(s_a.bhl.0, s_a.bhl.1, s_a.bhl.2);
                        next.hand_l.orientation = Quaternion::rotation_x(s_a.bhl.3);
                        next.hand_r.position = Vec3::new(s_a.bhr.0, s_a.bhr.1, s_a.bhr.2);
                        next.hand_r.orientation = Quaternion::rotation_x(s_a.bhr.3);

                        next.hold.position = Vec3::new(0.0, -1.0, -5.2);
                        next.hold.orientation = Quaternion::rotation_x(-PI / 2.0);
                        next.hold.scale = Vec3::one() * 1.0;

                        next.control.position = Vec3::new(s_a.bc.0, s_a.bc.1, s_a.bc.2);
                        next.control.orientation =
                            Quaternion::rotation_y(s_a.bc.4) * Quaternion::rotation_z(s_a.bc.5);
                    },
                    Some(ToolKind::Debug) => {
                        next.hand_l.position = Vec3::new(-7.0, 4.0, 3.0);
                        next.hand_l.orientation = Quaternion::rotation_x(1.27);
                        next.main.position = Vec3::new(-5.0, 5.0, 23.0);
                        next.main.orientation = Quaternion::rotation_x(PI);
                    },
                    Some(ToolKind::Farming) => {
                        next.hand_l.position = Vec3::new(9.0, 1.0, 1.0);
                        next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0);
                        next.hand_r.position = Vec3::new(9.0, 1.0, 11.0);
                        next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0);
                        next.main.position = Vec3::new(7.5, 7.5, 13.2);
                        next.main.orientation = Quaternion::rotation_y(PI);
                        next.control.position = Vec3::new(-11.0, 1.8, 4.0);
                    },
                    Some(ToolKind::Instrument) => {
                        next.hand_l.position = Vec3::new(-7.0, 4.0, 3.0);
                        next.hand_l.orientation = Quaternion::rotation_x(1.27);
                        next.main.position = Vec3::new(-5.0, 5.0, 23.0);
                        next.main.orientation = Quaternion::rotation_x(PI);
                    },
                    _ => {},
                },
                (_, _) => {},
            };
            match hands {
                (Some(Hands::One), _) => {
                    next.control_l.position = Vec3::new(-7.0, 8.0, 2.0);
                    next.control_l.orientation = Quaternion::rotation_x(-0.3);
                    next.hand_l.position = Vec3::new(0.0, -0.5, 0.0);
                    next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
                },
                (_, _) => {},
            };
            match hands {
                (None | Some(Hands::One), Some(Hands::One)) => {
                    next.control_r.position = Vec3::new(7.0, 8.0, 2.0);
                    next.control_r.orientation = Quaternion::rotation_x(-0.3);
                    next.hand_r.position = Vec3::new(0.0, -0.5, 0.0);
                    next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
                },
                (_, _) => {},
            };
            match hands {
                (None, None) | (None, Some(Hands::One)) => {
                    next.hand_l.position = Vec3::new(-4.5, 8.0, 5.0);
                    next.hand_l.orientation =
                        Quaternion::rotation_x(1.9) * Quaternion::rotation_y(-0.5)
                },
                (_, _) => {},
            };
            match hands {
                (None, None) | (Some(Hands::One), None) => {
                    next.hand_r.position = Vec3::new(4.5, 8.0, 5.0);
                    next.hand_r.orientation =
                        Quaternion::rotation_x(1.9) * Quaternion::rotation_y(0.5)
                },
                (_, _) => {},
            }
        } else if mirror > 0.0 {
            next.hand_r.position = Vec3::new(
                s_a.hand.0 + movement1abs * -4.0,
                s_a.hand.1 + movement1 * 7.0,
                s_a.hand.2 + movement1 * 6.0,
            );
            next.hand_r.orientation =
                Quaternion::rotation_x(movement1 * 1.2) * Quaternion::rotation_y(movement1 * 1.2);
        } else {
            next.hand_l.position = Vec3::new(
                -s_a.hand.0 + movement1abs * 4.0,
                s_a.hand.1 + movement1abs * 7.0,
                s_a.hand.2 + movement1abs * 6.0,
            );
            next.hand_l.orientation = Quaternion::rotation_x(movement1abs * 1.2)
                * Quaternion::rotation_y(movement1 * 1.2);
        };
        next.torso.position = Vec3::new(0.0, 0.0, 0.0);
        next.torso.orientation = Quaternion::rotation_z(0.0);

        if let (None, Some(Hands::Two)) = hands {
            next.second = next.main;
        }

        next
    }
}
