use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::{AbilityInfo, StageSection},
};
use core::f32::consts::PI;

pub struct BlockAnimation;

type BlockAnimationDependency<'a> = (
    (Option<Hands>, Option<Hands>),
    Option<ToolKind>,
    Option<ToolKind>,
    Vec3<f32>,
    Option<&'a str>,
    Option<StageSection>,
    Option<AbilityInfo>,
);
impl Animation for BlockAnimation {
    type Dependency<'a> = BlockAnimationDependency<'a>;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_block\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_block")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            hands,
            active_tool_kind,
            second_tool_kind,
            velocity,
            ability_id,
            stage_section,
            _ability_info,
        ): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_z(0.0);
        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation = Quaternion::rotation_z(0.0);

        match ability_id {
            None => {
                let speed = Vec2::<f32>::from(velocity).magnitude();

                let (movement1base, move2, movement3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, (anim_time * 10.0).sin(), 0.0),

                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powf(4.0)),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - movement3;
                let move1 = movement1base * pullback;

                if speed > 0.5 {
                } else {
                    next.chest.position =
                        Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + move1 * -1.0 + move2 * 0.2);
                    next.chest.orientation = Quaternion::rotation_x(move1 * -0.15);
                    next.head.orientation = Quaternion::rotation_x(move1 * 0.25);

                    next.belt.position =
                        Vec3::new(0.0, s_a.belt.0 + move1 * 0.5, s_a.belt.1 + move1 * 0.5);
                    next.shorts.position =
                        Vec3::new(0.0, s_a.shorts.0 + move1 * 1.3, s_a.shorts.1 + move1 * 1.0);

                    next.belt.orientation = Quaternion::rotation_x(move1 * 0.15);
                    next.shorts.orientation = Quaternion::rotation_x(move1 * 0.25);

                    next.foot_l.position =
                        Vec3::new(-s_a.foot.0, s_a.foot.1 + move1 * 2.0, s_a.foot.2);
                    next.foot_l.orientation = Quaternion::rotation_z(move1 * -0.5);

                    next.foot_r.position =
                        Vec3::new(s_a.foot.0, s_a.foot.1 + move1 * -2.0, s_a.foot.2);
                    next.foot_r.orientation = Quaternion::rotation_x(move1 * -0.5);
                };

                match (hands, active_tool_kind, second_tool_kind) {
                    ((Some(Hands::Two), _), tool, _) | ((None, Some(Hands::Two)), _, tool) => {
                        match tool {
                            Some(ToolKind::Sword) => {
                                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                                next.hand_l.orientation = Quaternion::rotation_x(s_a.shl.3)
                                    * Quaternion::rotation_y(s_a.shl.4);
                                next.hand_r.position = Vec3::new(
                                    s_a.shr.0 + move1 * -2.0,
                                    s_a.shr.1,
                                    s_a.shr.2 + move1 * 20.0,
                                );
                                next.hand_r.orientation = Quaternion::rotation_x(s_a.shr.3)
                                    * Quaternion::rotation_y(s_a.shr.4)
                                    * Quaternion::rotation_z(move1 * 1.5);

                                next.control.position = Vec3::new(
                                    s_a.sc.0 + move1 * -3.0,
                                    s_a.sc.1,
                                    s_a.sc.2 + move1 * 4.0,
                                );
                                next.control.orientation = Quaternion::rotation_x(s_a.sc.3)
                                    * Quaternion::rotation_y(move1 * 1.1)
                                    * Quaternion::rotation_z(move1 * 1.7);
                            },

                            Some(ToolKind::Axe) => {
                                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                                next.main.orientation = Quaternion::rotation_x(0.0);

                                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                                next.hand_l.orientation = Quaternion::rotation_x(s_a.ahl.3)
                                    * Quaternion::rotation_y(s_a.ahl.4);
                                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                                next.hand_r.orientation = Quaternion::rotation_x(s_a.ahr.3)
                                    * Quaternion::rotation_z(s_a.ahr.5);

                                next.control.position = Vec3::new(
                                    s_a.ac.0 + move1 * 13.0,
                                    s_a.ac.1 + move1 * -3.0,
                                    s_a.ac.2 + move1 * 8.0,
                                );
                                next.control.orientation =
                                    Quaternion::rotation_x(s_a.ac.3 + move1 * -2.0)
                                        * Quaternion::rotation_y(s_a.ac.4 + move1 * -1.8)
                                        * Quaternion::rotation_z(s_a.ac.5 + move1 * 4.0);
                            },
                            Some(ToolKind::Hammer) | Some(ToolKind::Pick) => {
                                next.hand_l.position = Vec3::new(
                                    s_a.hhl.0,
                                    s_a.hhl.1 + move1 * 6.0,
                                    s_a.hhl.2 + move1 * 6.0,
                                );
                                next.hand_l.orientation =
                                    Quaternion::rotation_x(s_a.hhl.3 + move1 * -0.5)
                                        * Quaternion::rotation_y(s_a.hhl.4 + move1 * 1.5)
                                        * Quaternion::rotation_z(s_a.hhl.5 + move1 * PI);
                                next.hand_r.position = Vec3::new(s_a.hhr.0, s_a.hhr.1, s_a.hhr.2);
                                next.hand_r.orientation = Quaternion::rotation_x(s_a.hhr.3)
                                    * Quaternion::rotation_y(s_a.hhr.4)
                                    * Quaternion::rotation_z(s_a.hhr.5);

                                next.control.position = Vec3::new(
                                    s_a.hc.0 + move1 * 3.0,
                                    s_a.hc.1 + move1 * 3.0,
                                    s_a.hc.2 + move1 * 10.0,
                                );
                                next.control.orientation = Quaternion::rotation_x(s_a.hc.3)
                                    * Quaternion::rotation_y(s_a.hc.4)
                                    * Quaternion::rotation_z(s_a.hc.5 + move1 * -1.0);
                            },
                            Some(ToolKind::Staff) | Some(ToolKind::Sceptre) => {
                                next.hand_r.position =
                                    Vec3::new(s_a.sthr.0, s_a.sthr.1, s_a.sthr.2);
                                next.hand_r.orientation = Quaternion::rotation_x(s_a.sthr.3)
                                    * Quaternion::rotation_y(s_a.sthr.4);

                                next.control.position = Vec3::new(s_a.stc.0, s_a.stc.1, s_a.stc.2);

                                next.hand_l.position =
                                    Vec3::new(s_a.sthl.0, s_a.sthl.1, s_a.sthl.2);
                                next.hand_l.orientation = Quaternion::rotation_x(s_a.sthl.3);

                                next.control.orientation = Quaternion::rotation_x(s_a.stc.3)
                                    * Quaternion::rotation_y(s_a.stc.4)
                                    * Quaternion::rotation_z(s_a.stc.5);
                            },
                            Some(ToolKind::Bow) => {
                                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                                next.main.orientation = Quaternion::rotation_x(0.0);
                                next.hand_l.position = Vec3::new(s_a.bhl.0, s_a.bhl.1, s_a.bhl.2);
                                next.hand_l.orientation = Quaternion::rotation_x(s_a.bhl.3);
                                next.hand_r.position = Vec3::new(s_a.bhr.0, s_a.bhr.1, s_a.bhr.2);
                                next.hand_r.orientation = Quaternion::rotation_x(s_a.bhr.3);

                                next.hold.position = Vec3::new(0.0, -1.0, -5.2);
                                next.hold.orientation = Quaternion::rotation_x(-PI / 2.0);
                                next.hold.scale = Vec3::one() * 1.0;

                                next.control.position = Vec3::new(s_a.bc.0, s_a.bc.1, s_a.bc.2);
                                next.control.orientation = Quaternion::rotation_x(0.0)
                                    * Quaternion::rotation_y(s_a.bc.4)
                                    * Quaternion::rotation_z(s_a.bc.5);
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
                                next.control.orientation = Quaternion::rotation_x(0.0)
                                    * Quaternion::rotation_y(0.6)
                                    * Quaternion::rotation_z(0.0);
                            },
                            _ => {},
                        }
                    },
                    ((_, _), _, _) => {},
                };
                match hands {
                    (Some(Hands::One), _) => {
                        next.control_l.position =
                            Vec3::new(-7.0, 8.0 + move1 * 3.0, 2.0 + move1 * 3.0);
                        next.control_l.orientation =
                            Quaternion::rotation_x(-0.3) * Quaternion::rotation_y(move1 * 1.0);
                        next.hand_l.position = Vec3::new(0.0, -0.5, 0.0);
                        next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
                    },
                    (_, _) => {},
                };
                match hands {
                    (None | Some(Hands::One), Some(Hands::One)) => {
                        next.control_r.position =
                            Vec3::new(7.0, 8.0 + move1 * 3.0, 2.0 + move1 * 3.0);
                        next.control_r.orientation =
                            Quaternion::rotation_x(-0.3) * Quaternion::rotation_y(move1 * -1.0);
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
                };

                if let (None, Some(Hands::Two)) = hands {
                    next.second = next.main;
                }
            },
            Some("common.abilities.sword.defensive_deflect") => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powi(2), 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, (anim_time * 20.0).sin(), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powf(0.5)),
                    _ => (0.0, 0.0, 0.0),
                };

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation =
                    Quaternion::rotation_x(s_a.sc.3) * Quaternion::rotation_z(move1 * -0.9);

                next.chest.orientation = Quaternion::rotation_z(move1 * -0.6);
                next.head.orientation = Quaternion::rotation_z(move1 * 0.2);
                next.belt.orientation = Quaternion::rotation_z(move1 * -0.1);
                next.shorts.orientation = Quaternion::rotation_z(move1 * 0.1);
                next.control.orientation.rotate_y(move1 * -1.7);
                next.control.orientation.rotate_z(move1 * 0.6);
                next.control.position += Vec3::new(move1 * 11.0, move1 * 2.0, move1 * 5.0);

                next.control.orientation.rotate_y(move2 / 50.0);

                next.chest.orientation.rotate_z(move3 * -0.6);
                next.head.orientation.rotate_z(move3 * 0.4);
                next.belt.orientation.rotate_z(move3 * 0.2);
                next.shorts.orientation.rotate_z(move3 * 0.6);
                next.control.position += Vec3::new(move3 * 6.0, 0.0, move3 * 9.0);
                next.control.orientation.rotate_z(move3 * -0.5);
                next.control.orientation.rotate_y(move3 * 0.6);
            },
            _ => {},
        }

        next
    }
}
