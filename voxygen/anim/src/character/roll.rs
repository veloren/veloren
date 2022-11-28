use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::StageSection,
};
use std::f32::consts::PI;

pub struct RollAnimation;

type RollAnimationDependency = (
    Option<ToolKind>,
    Option<ToolKind>,
    (Option<Hands>, Option<Hands>),
    bool,
    Vec3<f32>,
    Vec3<f32>,
    f32,
    Option<StageSection>,
);

impl Animation for RollAnimation {
    type Dependency<'a> = RollAnimationDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_roll\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_roll")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            active_tool_kind,
            _second_tool_kind,
            hands,
            wield_status,
            orientation,
            last_ori,
            _global_time,
            stage_section,
        ): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let ori: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if vek::Vec2::new(ori, last_ori)
            .map(|o| o.magnitude_squared())
            .map(|m| m > 0.0001 && m.is_finite())
            .reduce_and()
            && ori.angle_between(last_ori).is_finite()
        {
            ori.angle_between(last_ori).min(0.05)
                * last_ori.determine_side(Vec2::zero(), ori).signum()
        } else {
            0.0
        };

        let (movement1base, movement2, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
            Some(StageSection::Movement) => (1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - movement3;
        let movement1 = movement1base * pullback;

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
                    Some(ToolKind::Hammer | ToolKind::Pick | ToolKind::Instrument) => {
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
        } else {
            next.hand_l.position = Vec3::new(
                -s_a.hand.0,
                s_a.hand.1 + 1.0 * movement1,
                s_a.hand.2 + 2.0 * movement1,
            );

            next.hand_l.orientation = Quaternion::rotation_x(0.6 * movement1);

            next.hand_r.position = Vec3::new(
                -1.0 * movement1 + s_a.hand.0,
                s_a.hand.1 + 1.0 * movement1,
                s_a.hand.2 + 2.0 * movement1,
            );
            next.hand_r.orientation = Quaternion::rotation_x(0.6 * movement1);
        };
        next.head.position = Vec3::new(
            0.0,
            s_a.head.0 + 1.5 * movement1,
            s_a.head.1 - 1.0 * movement1,
        );
        next.head.orientation = Quaternion::rotation_x(-0.3 * movement1);

        next.chest.position = Vec3::new(0.0, s_a.chest.0, -9.5 * movement1 + s_a.chest.1);
        next.chest.orientation = Quaternion::rotation_x(-0.2 * movement1);

        next.belt.position = Vec3::new(
            0.0,
            s_a.belt.0 + 1.0 * movement1,
            s_a.belt.1 + 1.0 * movement1,
        );
        next.belt.orientation = Quaternion::rotation_x(0.55 * movement1);

        next.shorts.position = Vec3::new(
            0.0,
            s_a.shorts.0 + 4.5 * movement1,
            s_a.shorts.1 + 2.5 * movement1,
        );
        next.shorts.orientation = Quaternion::rotation_x(0.8 * movement1);

        next.foot_l.position = Vec3::new(
            1.0 * movement1 - s_a.foot.0,
            s_a.foot.1 + 5.5 * movement1,
            s_a.foot.2 - 5.0 * movement1,
        );
        next.foot_l.orientation = Quaternion::rotation_x(0.9 * movement1);

        next.foot_r.position = Vec3::new(
            1.0 * movement1 + s_a.foot.0,
            s_a.foot.1 + 5.5 * movement1,
            s_a.foot.2 - 5.0 * movement1,
        );
        next.foot_r.orientation = Quaternion::rotation_x(0.9 * movement1);

        next.torso.position = Vec3::new(0.0, 0.0, 7.0 * movement1);
        next.torso.orientation =
            Quaternion::rotation_x(-0.3 + movement1 * -0.4 + movement2 * -2.0 * PI)
                * Quaternion::rotation_z(tilt * -10.0);

        next
    }
}
