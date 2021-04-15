use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::{AbilityInfo, StageSection},
};
pub struct ChargeswingAnimation;

type ChargeswingAnimationDependency = (
    (Option<Hands>, Option<Hands>),
    Vec3<f32>,
    f32,
    Option<StageSection>,
    Option<AbilityInfo>,
);

impl Animation for ChargeswingAnimation {
    type Dependency = ChargeswingAnimationDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_chargeswing\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_chargeswing")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (hands, _velocity, _global_time, stage_section, ability_info): Self::Dependency,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let lab: f32 = 1.0;

        let short = ((5.0 / (1.5 + 3.5 * ((anim_time * lab * 8.0).sin()).powi(2))).sqrt())
            * ((anim_time * lab * 8.0).sin());
        // end spin stuff

        let (move1base, move2base, movement3, tension, test) = match stage_section {
            Some(StageSection::Charge) => (
                (anim_time.powf(0.25)).min(1.0),
                0.0,
                0.0,
                (anim_time * 18.0 * lab).sin(),
                0.0,
            ),
            Some(StageSection::Swing) => (1.0, anim_time.powf(0.25), 0.0, 0.0, anim_time.powi(4)),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4), 0.0, 1.0),
            _ => (0.0, 0.0, 0.0, 0.0, 0.0),
        };

        let pullback = 1.0 - movement3;
        let move1 = move1base * pullback;
        let move2 = move2base * pullback;
        let slowrise = test * pullback;
        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation = Quaternion::rotation_z(0.0);

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

        if let Some(ToolKind::Hammer) = ability_info.and_then(|a| a.tool) {
            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_x(0.0);

            next.chest.orientation =
                Quaternion::rotation_z(short * 0.04 + (move1 * 2.0 + move2 * -3.5));
            next.belt.orientation = Quaternion::rotation_z(short * 0.08 + (move1 * -1.0));
            next.shorts.orientation = Quaternion::rotation_z(short * 0.15 + (move1 * -1.5));
            next.head.position = Vec3::new(
                0.0 + (move1 * -1.0 + move2 * 2.0),
                s_a.head.0 + (move1 * 1.0),
                s_a.head.1,
            );
            next.head.orientation = Quaternion::rotation_z(move1 * -1.5 + move2 * 3.2);
            next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);
        }

        match hands {
            (Some(Hands::Two), _) | (None, Some(Hands::Two)) => match ability_info
                .and_then(|a| a.tool)
            {
                Some(ToolKind::Hammer) | Some(ToolKind::HammerSimple) => {
                    next.hand_l.position =
                        Vec3::new(s_a.hhl.0, s_a.hhl.1, s_a.hhl.2 + (move2 * -8.0));
                    next.hand_l.orientation =
                        Quaternion::rotation_x(s_a.hhl.3) * Quaternion::rotation_y(s_a.hhl.4);
                    next.hand_r.position = Vec3::new(s_a.hhr.0, s_a.hhr.1, s_a.hhr.2);
                    next.hand_r.orientation =
                        Quaternion::rotation_x(s_a.hhr.3) * Quaternion::rotation_y(s_a.hhr.4);

                    next.control.position = Vec3::new(
                        s_a.hc.0 + (move1 * -2.0 + move2 * -8.0),
                        s_a.hc.1 + (move1 * 2.0 + move2 * 6.0),
                        s_a.hc.2 + (move1 * -2.0 + slowrise * 8.0),
                    );
                    next.control.orientation = Quaternion::rotation_x(s_a.hc.3 + (move2 * 0.0))
                        * Quaternion::rotation_y(
                            s_a.hc.4
                                + (tension * 0.08 + move1 * 0.7 + move2 * -1.0 + slowrise * 2.0),
                        )
                        * Quaternion::rotation_z(s_a.hc.5 + (move1 * 0.2 + move2 * -1.0));
                },
                _ => {},
            },
            (_, _) => {},
        };

        match hands {
            (Some(Hands::One), _) => match ability_info.and_then(|a| a.tool) {
                Some(ToolKind::Hammer) | Some(ToolKind::HammerSimple) => {
                    next.control_l.position = Vec3::new(
                        -7.0 + move1 * 4.0,
                        8.0 + move1 * 2.0 + move2 * 4.0,
                        2.0 + move1 * -1.0 + slowrise * 20.0,
                    );
                    next.control_l.orientation = Quaternion::rotation_x(-0.3 + move2 * -1.0)
                        * Quaternion::rotation_y(tension * 0.07 + move1 * -1.2 + slowrise * 0.5)
                        * Quaternion::rotation_z(move2 * 1.0);
                    next.hand_l.position = Vec3::new(0.0, -0.5, 0.0);
                    next.hand_l.orientation = Quaternion::rotation_x(1.57)
                },

                _ => {},
            },
            (_, _) => {},
        };

        match hands {
            (None | Some(Hands::One), Some(Hands::One)) => {
                match ability_info.and_then(|a| a.tool) {
                    Some(ToolKind::Hammer) | Some(ToolKind::HammerSimple) => {
                        next.control_r.position = Vec3::new(
                            7.0 + move1 * 1.0 + move2 * -20.0,
                            8.0 + move1 * 1.0 + move2 * 4.0,
                            2.0 + move1 * -3.0 + slowrise * 20.0,
                        );
                        next.control_r.orientation = Quaternion::rotation_x(-0.3 + move2 * -1.0)
                            * Quaternion::rotation_y(
                                tension * -0.07 + move1 * -2.0 + slowrise * 1.5,
                            )
                            * Quaternion::rotation_z(move2 * 1.0);
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

        if let (None, Some(Hands::Two)) = hands {
            next.second = next.main;
        }

        next
    }
}
