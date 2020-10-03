use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::StageSection,
};
pub struct RepeaterAnimation;

impl Animation for RepeaterAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        Vec3<f32>,
        f64,
        Option<StageSection>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_repeater\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_repeater")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, _velocity, _global_time, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let lab = 1.0;

        // end spin stuff

        let movement = (anim_time as f32 * 1.0).min(1.0);
        let fire = (anim_time as f32 * 18.0 * lab as f32).sin();

        if let Some(ToolKind::Bow(_)) = active_tool_kind {
            next.l_hand.position = Vec3::new(2.0, 1.5, 0.0);
            next.l_hand.orientation = Quaternion::rotation_x(1.20)
                * Quaternion::rotation_y(-0.6)
                * Quaternion::rotation_z(-0.3);
            next.l_hand.scale = Vec3::one() * 1.05;
            next.r_hand.position = Vec3::new(5.9, 4.5, -5.0);
            next.r_hand.orientation = Quaternion::rotation_x(1.20)
                * Quaternion::rotation_y(-0.6)
                * Quaternion::rotation_z(-0.3);
            next.r_hand.scale = Vec3::one() * 1.05;
            next.main.position = Vec3::new(3.0, 2.0, -13.0);
            next.main.orientation = Quaternion::rotation_x(-0.3)
                * Quaternion::rotation_y(0.3)
                * Quaternion::rotation_z(-0.6);

            next.hold.position = Vec3::new(1.2, -1.0, -5.2);
            next.hold.orientation = Quaternion::rotation_x(-1.7)
                * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(-0.1);
            next.hold.scale = Vec3::one() * 1.0;

            next.control.position = Vec3::new(-7.0, 6.0, 6.0);
            next.control.orientation = Quaternion::rotation_x(0.0) * Quaternion::rotation_z(0.0);
            next.control.scale = Vec3::one();
            if let Some(stage_section) = stage_section {
                match stage_section {
                    StageSection::Movement => {
                        next.l_foot.position = Vec3::new(
                            -skeleton_attr.foot.0 + movement * -0.75 - 0.75,
                            skeleton_attr.foot.1 + movement * 4.0 + 4.0,
                            skeleton_attr.foot.2 + movement * 2.5 + 2.5,
                        );
                        next.l_foot.orientation = Quaternion::rotation_x(movement * 0.6 + 0.6)
                            * Quaternion::rotation_z(movement * 0.3 + 0.3);

                        next.r_foot.position = Vec3::new(
                            skeleton_attr.foot.0 + movement * 0.75 + 0.75,
                            skeleton_attr.foot.1 + movement * 4.0 + 4.0,
                            skeleton_attr.foot.2 + movement * 2.5 + 2.5,
                        );
                        next.r_foot.orientation = Quaternion::rotation_x(movement * 0.6 + 0.6)
                            * Quaternion::rotation_z(movement * -0.3 - 0.3);
                        next.shorts.position = Vec3::new(
                            0.0,
                            skeleton_attr.shorts.0 + movement * 4.0,
                            skeleton_attr.shorts.1 + movement * 1.0,
                        );
                        next.shorts.orientation = Quaternion::rotation_x(movement * 0.6);
                        next.belt.position = Vec3::new(
                            0.0,
                            skeleton_attr.belt.0 + movement * 2.0,
                            skeleton_attr.belt.1,
                        );
                        next.belt.orientation = Quaternion::rotation_x(movement * 0.2);
                        next.control.position = Vec3::new(
                            -7.0 + movement * 5.0,
                            6.0 + movement * 3.0,
                            6.0 + movement * 1.0,
                        );
                        next.control.orientation = Quaternion::rotation_x(movement * 0.4)
                            * Quaternion::rotation_y(movement * 0.8);
                        next.head.orientation = Quaternion::rotation_y(movement * 0.15);
                        next.torso.orientation = Quaternion::rotation_x(movement * 0.1)
                    },

                    StageSection::Buildup => {
                        next.l_foot.position = Vec3::new(
                            -skeleton_attr.foot.0 - 1.5,
                            skeleton_attr.foot.1 + 8.0,
                            skeleton_attr.foot.2 + 5.0,
                        );
                        next.l_foot.orientation = Quaternion::rotation_x(1.2 + movement * -0.2)
                            * Quaternion::rotation_z(0.6);

                        next.r_foot.position = Vec3::new(
                            skeleton_attr.foot.0 + 1.5,
                            skeleton_attr.foot.1 + 8.0,
                            skeleton_attr.foot.2 + 5.0,
                        );
                        next.r_foot.orientation = Quaternion::rotation_x(1.2 + movement * -0.2)
                            * Quaternion::rotation_z(-0.6);
                        next.shorts.position = Vec3::new(
                            0.0,
                            skeleton_attr.shorts.0 + 4.0,
                            skeleton_attr.shorts.1 + 1.0,
                        );
                        next.shorts.orientation = Quaternion::rotation_x(0.6);
                        next.belt.position =
                            Vec3::new(0.0, skeleton_attr.belt.0 + 2.0, skeleton_attr.belt.1);
                        next.belt.orientation = Quaternion::rotation_x(0.2);
                        next.control.position = Vec3::new(-2.0, 9.0, 7.0);
                        next.control.orientation =
                            Quaternion::rotation_x(0.4) * Quaternion::rotation_y(0.8);
                        next.head.orientation = Quaternion::rotation_y(0.15 + movement * 0.05);
                        next.torso.orientation = Quaternion::rotation_x(0.1 + movement * 0.1);
                    },

                    StageSection::Shoot => {
                        next.l_foot.position = Vec3::new(
                            -skeleton_attr.foot.0 - 1.5,
                            skeleton_attr.foot.1 + 8.0,
                            skeleton_attr.foot.2 + 5.0,
                        );
                        next.l_foot.orientation =
                            Quaternion::rotation_x(1.0) * Quaternion::rotation_z(0.6);

                        next.r_foot.position = Vec3::new(
                            skeleton_attr.foot.0 + 1.5,
                            skeleton_attr.foot.1 + 8.0,
                            skeleton_attr.foot.2 + 5.0,
                        );
                        next.r_foot.orientation =
                            Quaternion::rotation_x(1.0) * Quaternion::rotation_z(-0.6);
                        next.shorts.position = Vec3::new(
                            0.0,
                            skeleton_attr.shorts.0 + 4.0,
                            skeleton_attr.shorts.1 + 1.0,
                        );
                        next.shorts.orientation = Quaternion::rotation_x(0.6);
                        next.belt.position =
                            Vec3::new(0.0, skeleton_attr.belt.0 + 2.0, skeleton_attr.belt.1);
                        next.belt.orientation = Quaternion::rotation_x(0.2);
                        next.control.position = Vec3::new(-2.0, 9.0, 7.0);
                        next.control.orientation =
                            Quaternion::rotation_x(0.4) * Quaternion::rotation_y(0.8);
                        next.head.orientation = Quaternion::rotation_y(0.2);
                        next.torso.orientation = Quaternion::rotation_x(0.2 + movement * 0.15);

                        next.l_hand.position =
                            Vec3::new(2.0 + fire * -6.0 - 3.0, 1.5 + fire * -6.0 - 3.0, 0.0);
                        next.l_hand.orientation = Quaternion::rotation_x(1.20)
                            * Quaternion::rotation_y(-0.6)
                            * Quaternion::rotation_z(-0.3);
                        next.hold.scale = Vec3::one() * 0.0;
                    },
                    StageSection::Recover => {},
                    _ => {},
                }
            }
        }

        next.second.scale = match (
            active_tool_kind.map(|tk| tk.hands()),
            second_tool_kind.map(|tk| tk.hands()),
        ) {
            (Some(Hands::OneHand), Some(Hands::OneHand)) => Vec3::one(),
            (_, _) => Vec3::zero(),
        };

        //next.torso.position = Vec3::new(0.0, 0.0, 0.0) * skeleton_attr.scaler;
        //next.torso.orientation = Quaternion::rotation_z(0.0);
        //next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
        next
    }
}
