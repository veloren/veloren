use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::{AbilityInfo, StageSection},
};
use std::f32::consts::PI;

pub struct BeamAnimation;

impl Animation for BeamAnimation {
    type Dependency<'a> = (
        Option<AbilityInfo>,
        (Option<Hands>, Option<Hands>),
        f32,
        f32,
        Option<StageSection>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_beam\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_beam")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_info, hands, _global_time, velocity, stage_section): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (move1, move2, move3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0),
        };

        next.hand_l.position = Vec3::new(s_a.sthl.0, s_a.sthl.1, s_a.sthl.2);
        next.hand_l.orientation =
            Quaternion::rotation_x(s_a.sthl.3) * Quaternion::rotation_y(s_a.sthl.4);
        next.hand_r.position = Vec3::new(s_a.sthr.0, s_a.sthr.1, s_a.sthl.2);
        next.hand_r.orientation =
            Quaternion::rotation_x(s_a.sthr.3) * Quaternion::rotation_y(s_a.sthr.4);
        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_x(0.0);

        next.control.position = Vec3::new(-4.0, 7.0, 4.0);
        next.control.orientation = Quaternion::rotation_x(-0.3)
            * Quaternion::rotation_y(0.15)
            * Quaternion::rotation_z(0.0);

        match ability_info.and_then(|a| a.tool) {
            Some(ToolKind::Staff) | Some(ToolKind::Sceptre) => {
                next.control.position = Vec3::new(
                    s_a.stc.0 + (move1 * 16.0) * (1.0 - move3),
                    s_a.stc.1 + (move1 + (move2 * 8.0).sin() * 2.0) * (1.0 - move3),
                    s_a.stc.2 + (move1 * 10.0) * (1.0 - move3),
                );
                next.control.orientation =
                    Quaternion::rotation_x(s_a.stc.3 + (move1 * -1.2) * (1.0 - move3))
                        * Quaternion::rotation_y(
                            s_a.stc.4
                                + (move1 * -1.4 + (move2 * 16.0).sin() * 0.07) * (1.0 - move3),
                        )
                        * Quaternion::rotation_z(
                            (move1 * -1.7 + (move2 * 8.0 + PI / 4.0).sin() * 0.3) * (1.0 - move3),
                        );
                next.head.orientation = Quaternion::rotation_x(0.0);

                next.hand_l.position = Vec3::new(
                    0.0 + (move1 * -1.0 + (move2 * 8.0).sin() * 3.5) * (1.0 - move3),
                    0.0 + (move1 * -5.0 + (move2 * 8.0).sin() * -2.0 + (move2 * 16.0).sin() * -1.5)
                        * (1.0 - move3),
                    -4.0 + (move1 * 19.0 + (move2 * 8.0 + PI / 2.0).sin() * 3.5) * (1.0 - move3),
                );
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.sthr.3 + (move1 * -0.3) * (1.0 - move3))
                        * Quaternion::rotation_y(
                            (move1 * -1.1 + (move2 * 8.0 + PI / 2.0).sin() * -0.3) * (1.0 - move3),
                        )
                        * Quaternion::rotation_z((move1 * -2.8) * (1.0 - move3));

                if velocity < 0.5 {
                    next.head.orientation =
                        Quaternion::rotation_z(move1 * -0.5 + (move2 * 16.0).sin() * 0.05);

                    next.foot_l.position =
                        Vec3::new(-s_a.foot.0, s_a.foot.1 + move1 * -3.0, s_a.foot.2);
                    next.foot_l.orientation =
                        Quaternion::rotation_x(move1 * -0.5) * Quaternion::rotation_z(move1 * 0.5);

                    next.foot_r.position =
                        Vec3::new(s_a.foot.0, s_a.foot.1 + move1 * 4.0, s_a.foot.2);
                    next.foot_r.orientation = Quaternion::rotation_z(move1 * 0.5);
                    next.chest.orientation =
                        Quaternion::rotation_x(move1 * -0.2 + (move2 * 8.0).sin() * 0.05)
                            * Quaternion::rotation_z(move1 * 0.5);
                    next.belt.orientation =
                        Quaternion::rotation_x(move1 * 0.1) * Quaternion::rotation_z(move1 * -0.1);
                    next.shorts.orientation =
                        Quaternion::rotation_x(move1 * 0.2) * Quaternion::rotation_z(move1 * -0.2);
                } else {
                };
            },
            _ => {},
        }

        if let (None, Some(Hands::Two)) = hands {
            next.second = next.main;
        }

        next
    }
}
