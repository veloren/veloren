use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::StageSection,
};
use std::f32::consts::PI;

pub struct BeamAnimation;

impl Animation for BeamAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        f64,
        f32,
        Option<StageSection>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_beam\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_beam")]
    #[allow(clippy::single_match)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, _global_time, velocity, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (movement1, movement2, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time as f32, 0.0, 0.0),
            Some(StageSection::Cast) => (1.0, anim_time as f32, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time as f32),
            _ => (0.0, 0.0, 0.0),
        };

        next.hand_l.position = Vec3::new(0.0, 0.0, -4.0);
        next.hand_l.orientation = Quaternion::rotation_x(1.27) * Quaternion::rotation_y(0.0);
        next.hand_r.position = Vec3::new(0.0, 0.0, 2.0);
        next.hand_r.orientation = Quaternion::rotation_x(1.57) * Quaternion::rotation_y(0.2);
        next.main.position = Vec3::new(0.0, 0.0, 13.2);
        next.main.orientation = Quaternion::rotation_y(PI);

        next.control.position = Vec3::new(-4.0, 7.0, 4.0);
        next.control.orientation = Quaternion::rotation_x(-0.3)
            * Quaternion::rotation_y(0.15)
            * Quaternion::rotation_z(0.0);

        match active_tool_kind {
            Some(ToolKind::Staff(_)) | Some(ToolKind::Sceptre(_)) => {
                next.control.position = Vec3::new(
                    -4.0 + movement1 * 16.0 + movement3 * -16.0,
                    7.0 + movement1 + (movement2 * 8.0).sin() * 2.0 + movement3 * -1.0,
                    4.0 + movement1 * 4.0 + movement3 * -4.0,
                );
                next.control.orientation =
                    Quaternion::rotation_x(-0.3 + movement1 * -1.2 + movement3 * 1.2)
                        * Quaternion::rotation_y(
                            0.15 + movement1 * -1.4
                                + (movement2 * 16.0).sin() * 0.07
                                + movement3 * 1.4,
                        )
                        * Quaternion::rotation_z(
                            movement1 * -1.7
                                + (movement2 * 8.0 + PI / 4.0).sin() * 0.3
                                + movement3 * 1.7,
                        );
                next.head.orientation = Quaternion::rotation_x(0.0) * Quaternion::rotation_z(0.0);

                next.hand_l.position = Vec3::new(
                    0.0 + movement1 * -1.0 + (movement2 * 8.0).sin() * 3.5 + movement3,
                    0.0 + movement1 * -5.0
                        + (movement2 * 8.0).sin() * -2.0
                        + (movement2 * 16.0).sin() * -1.5
                        + movement3 * 5.0,
                    -4.0 + movement1 * 19.0
                        + (movement2 * 8.0 + PI / 2.0).sin() * 3.5
                        + movement3 * -19.0,
                );
                next.hand_l.orientation = Quaternion::rotation_x(1.57 + movement3 * -0.3)
                    * Quaternion::rotation_y(
                        movement1 * -1.1
                            + (movement2 * 8.0 + PI / 2.0).sin() * -0.3
                            + movement3 * 1.1,
                    )
                    * Quaternion::rotation_z(movement1 * -2.8 + movement3 * 2.8);

                if velocity < 0.5 {
                    next.head.orientation =
                        Quaternion::rotation_z(movement1 * -0.5 + (movement2 * 16.0).sin() * 0.05);

                    next.foot_l.position =
                        Vec3::new(-s_a.foot.0, s_a.foot.1 + movement1 * -3.0, s_a.foot.2);
                    next.foot_l.orientation = Quaternion::rotation_x(movement1 * -0.5)
                        * Quaternion::rotation_z(movement1 * 0.5);

                    next.foot_r.position =
                        Vec3::new(s_a.foot.0, s_a.foot.1 + movement1 * 4.0, s_a.foot.2);
                    next.foot_r.orientation = Quaternion::rotation_z(movement1 * 0.5);
                    next.chest.orientation =
                        Quaternion::rotation_x(movement1 * -0.2 + (movement2 * 8.0).sin() * 0.05)
                            * Quaternion::rotation_z(movement1 * 0.5);
                    next.belt.orientation = Quaternion::rotation_x(movement1 * 0.1)
                        * Quaternion::rotation_z(movement1 * -0.1);
                    next.shorts.orientation = Quaternion::rotation_x(movement1 * 0.2)
                        * Quaternion::rotation_z(movement1 * -0.2);
                } else {
                };
            },
            _ => {},
        }
        next.second.scale = match (
            active_tool_kind.map(|tk| tk.hands()),
            second_tool_kind.map(|tk| tk.hands()),
        ) {
            (Some(Hands::OneHand), Some(Hands::OneHand)) => Vec3::one(),
            (_, _) => Vec3::zero(),
        };

        next
    }
}
