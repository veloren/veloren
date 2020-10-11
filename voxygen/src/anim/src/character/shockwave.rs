use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::StageSection,
};

pub struct Input {
    pub attack: bool,
}
pub struct ShockwaveAnimation;

impl Animation for ShockwaveAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        f64,
        f32,
        Option<StageSection>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_shockwave\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_shockwave")]
    #[allow(clippy::single_match)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, _global_time, velocity, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let movement = (anim_time as f32 * 1.0).min(1.0);

        next.head.position = Vec3::new(0.0, -2.0 + skeleton_attr.head.0, skeleton_attr.head.1);

        next.l_hand.position = Vec3::new(0.0, 0.0, -4.0);
        next.l_hand.orientation = Quaternion::rotation_x(1.27) * Quaternion::rotation_y(0.0);
        next.l_hand.scale = Vec3::one() * 1.05;
        next.r_hand.position = Vec3::new(0.0, 0.0, 2.0);
        next.r_hand.orientation = Quaternion::rotation_x(1.57) * Quaternion::rotation_y(0.2);
        next.r_hand.scale = Vec3::one() * 1.05;
        next.main.position = Vec3::new(0.0, 0.0, 13.2);
        next.main.orientation = Quaternion::rotation_y(3.14);

        next.control.position = Vec3::new(-4.0, 7.0, 4.0);
        next.control.orientation = Quaternion::rotation_x(-0.3)
            * Quaternion::rotation_y(0.15)
            * Quaternion::rotation_z(0.0);
        next.control.scale = Vec3::one();
        let twist = movement * 0.8;

        match active_tool_kind {
            //TODO: Inventory
            Some(ToolKind::Staff(_)) => {
                if let Some(stage_section) = stage_section {
                    match stage_section {
                        StageSection::Buildup => {
                            next.control.position = Vec3::new(
                                -4.0 + movement * 5.0,
                                7.0 + movement * 3.0,
                                4.0 + movement * 10.0,
                            );
                            next.control.orientation =
                                Quaternion::rotation_x(-0.3 + movement * 0.8)
                                    * Quaternion::rotation_y(0.15 + movement * -0.15)
                                    * Quaternion::rotation_z(movement * 0.8);
                            next.head.orientation = Quaternion::rotation_x(movement * 0.4)
                                * Quaternion::rotation_z(twist * 0.2);

                            next.chest.position = Vec3::new(
                                0.0,
                                skeleton_attr.chest.0,
                                skeleton_attr.chest.1 + movement * 2.0,
                            );
                            next.chest.orientation = Quaternion::rotation_z(twist * -0.2);

                            next.belt.orientation = Quaternion::rotation_z(twist * 0.6);

                            next.shorts.orientation = Quaternion::rotation_z(twist);

                            if velocity < 0.5 {
                                next.l_foot.position = Vec3::new(
                                    -skeleton_attr.foot.0,
                                    skeleton_attr.foot.1 + movement * -7.0,
                                    skeleton_attr.foot.2,
                                );
                                next.l_foot.orientation = Quaternion::rotation_x(movement * -0.8)
                                    * Quaternion::rotation_z(movement * 0.3);

                                next.r_foot.position = Vec3::new(
                                    skeleton_attr.foot.0,
                                    skeleton_attr.foot.1 + movement * 5.0,
                                    skeleton_attr.foot.2,
                                );
                                next.r_foot.orientation = Quaternion::rotation_y(movement * -0.3)
                                    * Quaternion::rotation_z(movement * 0.4);
                            } else {
                            };
                        },
                        StageSection::Swing => {
                            next.control.position = Vec3::new(1.0, 10.0, 14.0 + movement * -2.0);
                            next.control.orientation = Quaternion::rotation_x(0.5 + movement * 0.3)
                                * Quaternion::rotation_y(movement * 0.3)
                                * Quaternion::rotation_z(0.8 + movement * -0.8);

                            next.head.orientation = Quaternion::rotation_x(0.4)
                                * Quaternion::rotation_z(0.2 + movement * -0.8);

                            next.chest.position = Vec3::new(
                                0.0,
                                skeleton_attr.chest.0,
                                skeleton_attr.chest.1 + 2.0 + movement * -4.0,
                            );
                            next.chest.orientation = Quaternion::rotation_x(movement * -0.8)
                                * Quaternion::rotation_z(movement * -0.3);

                            next.belt.orientation = Quaternion::rotation_x(movement * 0.2)
                                * Quaternion::rotation_z(0.48 + movement * -0.48);

                            next.shorts.orientation = Quaternion::rotation_x(movement * 0.3)
                                * Quaternion::rotation_z(0.8 + movement * -0.8);
                            if velocity < 0.5 {
                                next.l_foot.position = Vec3::new(
                                    -skeleton_attr.foot.0,
                                    skeleton_attr.foot.1 - 7.0 + movement * 7.0,
                                    skeleton_attr.foot.2,
                                );
                                next.l_foot.orientation =
                                    Quaternion::rotation_x(-0.8 + movement * 0.8)
                                        * Quaternion::rotation_z(0.3 + movement * -0.3);

                                next.r_foot.position = Vec3::new(
                                    skeleton_attr.foot.0,
                                    skeleton_attr.foot.1 + 5.0 + movement * -5.0,
                                    skeleton_attr.foot.2,
                                );
                                next.r_foot.orientation =
                                    Quaternion::rotation_y(-0.3 + movement * 0.3)
                                        * Quaternion::rotation_z(0.4 + movement * -0.4);
                            } else {
                            };
                        },
                        StageSection::Recover => {
                            next.control.position = Vec3::new(
                                1.0 + movement * -5.0,
                                10.0 + movement * -3.0,
                                12.0 + movement * -8.0,
                            );
                            next.control.orientation =
                                Quaternion::rotation_x(0.8 + movement * -1.1)
                                    * Quaternion::rotation_y(movement * -0.15)
                                    * Quaternion::rotation_z(0.0);
                            next.head.orientation = Quaternion::rotation_x(0.4 + movement * -0.4)
                                * Quaternion::rotation_z(-0.6 + movement * 0.6);

                            next.belt.orientation = Quaternion::rotation_x(0.2 + movement * -0.2);

                            next.shorts.orientation = Quaternion::rotation_x(0.3 + movement * -0.3);

                            next.chest.position = Vec3::new(
                                0.0,
                                skeleton_attr.chest.0,
                                skeleton_attr.chest.1 - 2.0 + movement * 2.0,
                            );
                            next.chest.orientation = Quaternion::rotation_x(-0.8 + movement * 0.8)
                                * Quaternion::rotation_z(-0.3 + movement * 0.3);
                        },
                        _ => {},
                    }
                }
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
