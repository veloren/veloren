use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::StageSection,
};
use std::f32::consts::PI;
pub struct LeapAnimation;

impl Animation for LeapAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        Vec3<f32>,
        f64,
        Option<StageSection>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_leapmelee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_leapmelee")]
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

        let movement = (anim_time as f32 * 1.0).min(1.0);

        if let Some(ToolKind::Hammer(_)) = active_tool_kind {
            next.hand_l.position = Vec3::new(-12.0, 0.0, 0.0);
            next.hand_l.orientation = Quaternion::rotation_x(PI) * Quaternion::rotation_y(0.0);
            next.hand_l.scale = Vec3::one() * 1.08;
            next.hand_r.position = Vec3::new(2.0, 0.0, 0.0);
            next.hand_r.orientation = Quaternion::rotation_x(PI) * Quaternion::rotation_y(0.0);
            next.hand_r.scale = Vec3::one() * 1.06;
            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_y(-1.57) * Quaternion::rotation_z(1.57);

            next.control.position = Vec3::new(6.0, 7.0, 1.0);
            next.control.orientation = Quaternion::rotation_x(0.3)
                * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(0.0);
            next.control.scale = Vec3::one();

            next.head.position = Vec3::new(0.0, -2.0 + skeleton_attr.head.0, skeleton_attr.head.1);

            if let Some(stage_section) = stage_section {
                match stage_section {
                    StageSection::Buildup => {
                        next.control.position = Vec3::new(6.0, 7.0, 1.0);
                        next.control.orientation = Quaternion::rotation_x(0.3)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(movement * 0.5);
                        next.chest.orientation = Quaternion::rotation_x(movement * 0.3)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(movement * 0.5);

                        next.head.orientation = Quaternion::rotation_x(0.0)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(movement * -0.4);
                    },

                    StageSection::Movement => {
                        next.control.position = Vec3::new(
                            6.0 + movement * -10.0,
                            7.0 + movement * 5.0,
                            1.0 + movement * 5.0,
                        );
                        next.control.orientation = Quaternion::rotation_x(0.3)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(0.5 + movement * 0.5);
                        next.chest.orientation = Quaternion::rotation_x(0.3 + movement * 0.3)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(0.5 + movement * 0.2);
                        next.head.orientation = Quaternion::rotation_x(0.0)
                            * Quaternion::rotation_y(movement * -0.1)
                            * Quaternion::rotation_z(-0.4 + movement * -0.2);

                        next.foot_l.position = Vec3::new(
                            -skeleton_attr.foot.0,
                            skeleton_attr.foot.1 - 5.0,
                            skeleton_attr.foot.2,
                        );
                        next.foot_l.orientation = Quaternion::rotation_x(-0.8);

                        next.foot_r.position = Vec3::new(
                            skeleton_attr.foot.0,
                            skeleton_attr.foot.1 + 8.0,
                            skeleton_attr.foot.2 + 5.0,
                        );
                        next.foot_r.orientation = Quaternion::rotation_x(0.9);
                    },
                    StageSection::Swing => {
                        next.control.position =
                            Vec3::new(-4.0, 12.0 + movement * 5.0, 6.0 + movement * -7.0);
                        next.control.orientation = Quaternion::rotation_x(0.3 + movement * -3.0)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(1.0 + movement * 0.5);
                        next.chest.orientation = Quaternion::rotation_x(0.6 + movement * -0.9)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(0.7 + movement * -0.7);
                        next.head.orientation = Quaternion::rotation_x(movement * 0.2)
                            * Quaternion::rotation_y(-0.1)
                            * Quaternion::rotation_z(-0.6 + movement * 0.6);

                        next.hand_l.position = Vec3::new(-12.0 + movement * 10.0, 0.0, 0.0);

                        next.foot_l.position = Vec3::new(
                            -skeleton_attr.foot.0,
                            skeleton_attr.foot.1 + 8.0,
                            skeleton_attr.foot.2 - 5.0,
                        );
                        next.foot_l.orientation = Quaternion::rotation_x(0.9);

                        next.foot_r.position = Vec3::new(
                            skeleton_attr.foot.0,
                            skeleton_attr.foot.1 - 5.0,
                            skeleton_attr.foot.2,
                        );
                        next.foot_r.orientation = Quaternion::rotation_x(-0.8);
                    },
                    StageSection::Recover => {
                        next.control.position = Vec3::new(-4.0, 17.0, -1.0);
                        next.control.orientation = Quaternion::rotation_x(-2.7)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(1.5);
                        next.chest.orientation = Quaternion::rotation_x(-0.3 + movement * 0.3)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(0.0);
                        next.head.orientation = Quaternion::rotation_x(0.2)
                            * Quaternion::rotation_y(-0.1)
                            * Quaternion::rotation_z(0.0);

                        next.hand_l.position = Vec3::new(-2.0, 0.0, 0.0);
                    },
                    _ => {},
                }
            }
        } else if let Some(ToolKind::Axe(_)) = active_tool_kind {
            next.hand_l.position = Vec3::new(-0.5, 0.0, 4.0);
            next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
                * Quaternion::rotation_z(0.0)
                * Quaternion::rotation_y(0.0);
            next.hand_l.scale = Vec3::one() * 1.08;
            next.hand_r.position = Vec3::new(0.5, 0.0, -2.5);
            next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
                * Quaternion::rotation_z(0.0)
                * Quaternion::rotation_y(0.0);
            next.hand_r.scale = Vec3::one() * 1.06;
            next.main.position = Vec3::new(-0.0, -2.0, -1.0);
            next.main.orientation = Quaternion::rotation_x(0.0)
                * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(0.0);

            next.control.position = Vec3::new(-3.0, 11.0, 3.0);
            next.control.orientation = Quaternion::rotation_x(1.8)
                * Quaternion::rotation_y(-0.5)
                * Quaternion::rotation_z(PI - 0.2);
            next.control.scale = Vec3::one();

            next.head.position = Vec3::new(0.0, -2.0 + skeleton_attr.head.0, skeleton_attr.head.1);

            if let Some(stage_section) = stage_section {
                match stage_section {
                    StageSection::Buildup => {
                        next.control.position = Vec3::new(
                            -3.0 + movement * 3.0,
                            11.0 + movement * 1.0,
                            3.0 + movement * 12.0,
                        );
                        next.control.orientation = Quaternion::rotation_x(1.8 + movement * -1.0)
                            * Quaternion::rotation_y(-0.5 + movement * 0.5)
                            * Quaternion::rotation_z(PI + 0.2 - movement * 0.2);
                        next.chest.orientation = Quaternion::rotation_x(movement * -0.3)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(movement * 0.5);

                        next.head.orientation = Quaternion::rotation_x(0.0 + movement * -0.4);

                        next.foot_l.position = Vec3::new(
                            skeleton_attr.foot.0,
                            skeleton_attr.foot.1,
                            skeleton_attr.foot.2 - 8.0,
                        );

                        next.foot_r.position = Vec3::new(
                            skeleton_attr.foot.0,
                            skeleton_attr.foot.1,
                            skeleton_attr.foot.2 - 8.0,
                        );

                        next.foot_l.orientation = Quaternion::rotation_x(movement * 0.9);

                        next.foot_r.orientation = Quaternion::rotation_x(movement * 0.9);

                        next.belt.orientation = Quaternion::rotation_x(movement * 0.22);
                        next.shorts.orientation = Quaternion::rotation_x(movement * 0.3);

                        next.chest.position =
                            Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1 - 8.0);
                        next.torso.position =
                            Vec3::new(0.0, 0.0, 0.0 + 8.0) * skeleton_attr.scaler / 11.0;
                    },

                    StageSection::Movement => {
                        next.control.position = Vec3::new(
                            0.0, 12.0, //11
                            15.0,
                        );

                        next.chest.position =
                            Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1 - 8.0);
                        next.torso.position = Vec3::new(0.0, 0.0, 0.0 + 8.0) * skeleton_attr.scaler;
                        next.control.orientation = Quaternion::rotation_x(0.8 + movement * -0.5)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(PI);
                        next.torso.orientation = Quaternion::rotation_x(-0.3 + movement * 6.0)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(0.0);
                        next.head.orientation = Quaternion::rotation_x(-0.4 + movement * 0.4);

                        next.foot_l.position = Vec3::new(
                            -skeleton_attr.foot.0,
                            skeleton_attr.foot.1 + movement * 4.0,
                            skeleton_attr.foot.2 - 8.0 + movement * 3.0,
                        );
                        next.foot_l.orientation = Quaternion::rotation_x(0.9);

                        next.foot_r.position = Vec3::new(
                            skeleton_attr.foot.0,
                            skeleton_attr.foot.1 + movement * 4.0,
                            skeleton_attr.foot.2 - 8.0 + movement * 3.0,
                        );
                        next.foot_r.orientation = Quaternion::rotation_x(0.9);
                        next.chest.position =
                            Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1 - 8.0);
                        next.torso.position =
                            Vec3::new(0.0, 0.0, 0.0 + 8.0) * skeleton_attr.scaler / 11.0;
                        next.torso.orientation = Quaternion::rotation_x(movement * -1.8 * PI);
                        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

                        next.belt.orientation = Quaternion::rotation_x(0.22 + movement * 0.1);
                        next.shorts.orientation = Quaternion::rotation_x(0.3 + movement * 0.1);

                        next.chest.position =
                            Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1 - 8.0);
                        next.torso.position =
                            Vec3::new(0.0, 0.0, 0.0 + 8.0) * skeleton_attr.scaler / 11.0;
                    },
                    StageSection::Swing => {
                        next.control.position =
                            Vec3::new(0.0, 12.0 + movement * 3.0, 15.0 + movement * -15.0);
                        next.control.orientation = Quaternion::rotation_x(0.3 + movement * -1.0)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(PI);

                        next.head.orientation = Quaternion::rotation_x(movement * 0.2);

                        next.hand_l.position = Vec3::new(-0.5, 0.0, 4.0);

                        next.foot_l.position = Vec3::new(
                            -skeleton_attr.foot.0,
                            skeleton_attr.foot.1 + 4.0 + movement * -8.0,
                            skeleton_attr.foot.2 - 5.0 + movement * -3.0,
                        );
                        next.foot_l.orientation = Quaternion::rotation_x(0.9 - movement * 1.8);

                        next.foot_r.position = Vec3::new(
                            skeleton_attr.foot.0,
                            skeleton_attr.foot.1 + 4.0 + movement * -8.0,
                            skeleton_attr.foot.2 - 5.0 + movement * -3.0,
                        );
                        next.foot_r.orientation = Quaternion::rotation_x(0.9 - movement * 1.8);

                        next.torso.orientation =
                            Quaternion::rotation_x(-1.9 * PI - movement * 0.2 * PI);

                        next.chest.position =
                            Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1 - 8.0);
                        next.torso.position =
                            Vec3::new(0.0, 0.0, 0.0 + 8.0) * skeleton_attr.scaler / 11.0;
                    },
                    StageSection::Recover => {
                        next.control.position = Vec3::new(0.0, 15.0, 0.0);
                        next.control.orientation = Quaternion::rotation_x(-0.7)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(PI);

                        next.head.orientation = Quaternion::rotation_x(0.2);

                        next.hand_l.position = Vec3::new(-0.5, 0.0, 4.0);

                        next.chest.position =
                            Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1 - 8.0);
                        next.torso.position =
                            Vec3::new(0.0, 0.0, 0.0 + 8.0) * skeleton_attr.scaler / 11.0;
                        next.torso.orientation =
                            Quaternion::rotation_x(-6.7 + movement * -0.1 * PI);
                        next.foot_l.position = Vec3::new(
                            -skeleton_attr.foot.0,
                            skeleton_attr.foot.1 - 4.0,
                            skeleton_attr.foot.2 - 8.0,
                        );
                        next.foot_l.orientation = Quaternion::rotation_x(-0.9);

                        next.foot_r.position = Vec3::new(
                            skeleton_attr.foot.0,
                            skeleton_attr.foot.1 - 4.0,
                            skeleton_attr.foot.2 - 8.0,
                        );
                        next.foot_r.orientation = Quaternion::rotation_x(-0.9);
                    },
                    _ => {},
                }
            }
        }

        //next.lantern.position = Vec3::new(
        //    skeleton_attr.lantern.0,
        //    skeleton_attr.lantern.1,
        //    skeleton_attr.lantern.2,
        //);
        //next.glider.position = Vec3::new(0.0, 0.0, 10.0);
        //next.glider.scale = Vec3::one() * 0.0;
        //next.l_control.scale = Vec3::one();
        //next.r_control.scale = Vec3::one();

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
