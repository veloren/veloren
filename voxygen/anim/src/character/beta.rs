use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::{AbilityInfo, StageSection},
};
use core::f32::consts::PI;

pub struct BetaAnimation;
impl Animation for BetaAnimation {
    type Dependency<'a> = (
        (Option<Hands>, Option<Hands>),
        Option<&'a str>,
        f32,
        f32,
        Option<StageSection>,
        Option<AbilityInfo>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_beta\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_beta")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (hands, _ability_id, _velocity, _global_time, stage_section, ability_info): Self::Dependency<
            '_,
        >,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (move1, move2, move3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
            _ => (0.0, 0.0, 0.0),
        };
        if matches!(
            stage_section,
            Some(StageSection::Action | StageSection::Recover)
        ) {
            next.main_weapon_trail = true;
            next.off_weapon_trail = true;
        }
        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation = Quaternion::rotation_z(0.0);
        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_x(0.0);
        let pullback = 1.0 - move3;
        if let Some(ToolKind::Sword) = ability_info.and_then(|a| a.tool) {
            next.chest.orientation = Quaternion::rotation_x(0.15)
                * Quaternion::rotation_y((-0.1) * pullback)
                * Quaternion::rotation_z((0.4 + move1 * 1.5 + move2 * -2.5) * pullback);
            next.head.orientation = Quaternion::rotation_z(-0.4 + move1 * -1.0 + move2 * 1.5);
        }
        #[allow(clippy::single_match)]
        match hands {
            (Some(Hands::Two), _) | (None, Some(Hands::Two)) => match ability_info
                .and_then(|a| a.tool)
            {
                Some(ToolKind::Sword) => {
                    next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                    next.hand_l.orientation =
                        Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                    next.hand_r.position = Vec3::new(s_a.shr.0, s_a.shr.1, s_a.shr.2);
                    next.hand_r.orientation =
                        Quaternion::rotation_x(s_a.shr.3) * Quaternion::rotation_y(s_a.shr.4);

                    next.control.position = Vec3::new(
                        s_a.sc.0 + (-1.4 + move1 * -3.0 + move2 * -2.0) * pullback,
                        s_a.sc.1 + (-1.4 + move1 * 3.0 + move2 * 3.0) * pullback,
                        s_a.sc.2 + (-1.9 + move1 * 2.5 * pullback),
                    );
                    next.control.orientation = Quaternion::rotation_x(s_a.sc.3 + (-1.7) * pullback)
                        * Quaternion::rotation_y(
                            s_a.sc.4 + (0.4 + move1 * 1.5 + move2 * -2.5) * pullback,
                        )
                        * Quaternion::rotation_z(s_a.sc.5 + (1.67 + move2 * PI / 2.0) * pullback);
                },
                _ => {},
            },
            (_, _) => {},
        };

        match hands {
            #[allow(clippy::single_match)]
            (Some(Hands::One), _) => match ability_info.and_then(|a| a.tool) {
                Some(ToolKind::Sword) => {
                    next.control_l.position = Vec3::new(-12.0, 8.0, 2.0);
                    next.control_l.orientation = Quaternion::rotation_x(1.7)
                        * Quaternion::rotation_y(-3.7 + move1 * -0.75 + move2 * 1.0)
                        * Quaternion::rotation_z(3.69);
                    next.hand_l.position = Vec3::new(0.0, -0.5, 0.0);
                    next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
                },

                _ => {},
            },
            (_, _) => {},
        };
        match hands {
            #[allow(clippy::single_match)]
            (None | Some(Hands::One), Some(Hands::One)) => {
                match ability_info.and_then(|a| a.tool) {
                    Some(ToolKind::Sword) => {
                        next.control_r.position = Vec3::new(0.0 + move1 * -8.0, 13.0, 2.0);
                        next.control_r.orientation = Quaternion::rotation_x(1.7)
                            * Quaternion::rotation_y(-2.3 + move1 * -2.3 + move2 * 1.0)
                            * Quaternion::rotation_z(3.69);
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
