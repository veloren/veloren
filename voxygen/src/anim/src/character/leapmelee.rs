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

        let lab = 1.0;

        // Spin stuff here
        let foot = (((5.0)
            / (1.1 + 3.9 * ((anim_time as f32 * lab as f32 * 10.32).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 10.32).sin());

        let decel = (anim_time as f32 * 16.0 * lab as f32).min(PI / 2.0).sin();

        let spin = (anim_time as f32 * 2.8 * lab as f32).sin();
        let spinhalf = (anim_time as f32 * 1.4 * lab as f32).sin();

        // end spin stuff

        let movement = (anim_time as f32 * 1.0).min(1.0);

        if let Some(ToolKind::Hammer(_)) = active_tool_kind {
            next.l_hand.position = Vec3::new(-12.0, 0.0, 0.0);
            next.l_hand.orientation = Quaternion::rotation_x(PI) * Quaternion::rotation_y(0.0);
            next.l_hand.scale = Vec3::one() * 1.08;
            next.r_hand.position = Vec3::new(2.0, 0.0, 0.0);
            next.r_hand.orientation = Quaternion::rotation_x(PI) * Quaternion::rotation_y(0.0);
            next.r_hand.scale = Vec3::one() * 1.06;
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

                        next.l_foot.position = Vec3::new(
                            -skeleton_attr.foot.0,
                            skeleton_attr.foot.1 - 5.0,
                            skeleton_attr.foot.2,
                        );
                        next.l_foot.orientation = Quaternion::rotation_x(-0.8);

                        next.r_foot.position = Vec3::new(
                            skeleton_attr.foot.0,
                            skeleton_attr.foot.1 + 8.0,
                            skeleton_attr.foot.2 + 5.0,
                        );
                        next.r_foot.orientation = Quaternion::rotation_x(0.9);
                    },
                    StageSection::Swing => {
                        next.control.position =
                            Vec3::new(-4.0, 12.0 + movement * 13.0, 6.0 + movement * -7.0);
                        next.control.orientation = Quaternion::rotation_x(0.3 + movement * -3.0)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(1.0 + movement * 0.5);
                        next.chest.orientation = Quaternion::rotation_x(0.6 + movement * -0.9)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(0.7 + movement * -0.7);
                        next.head.orientation = Quaternion::rotation_x(movement * 0.2)
                            * Quaternion::rotation_y(-0.1)
                            * Quaternion::rotation_z(-0.6 + movement * 0.6);

                        next.l_hand.position = Vec3::new(-12.0 + movement * 10.0, 0.0, 0.0);

                        next.l_foot.position = Vec3::new(
                            -skeleton_attr.foot.0,
                            skeleton_attr.foot.1 + 8.0,
                            skeleton_attr.foot.2 - 5.0,
                        );
                        next.l_foot.orientation = Quaternion::rotation_x(0.9);

                        next.r_foot.position = Vec3::new(
                            skeleton_attr.foot.0,
                            skeleton_attr.foot.1 - 5.0,
                            skeleton_attr.foot.2,
                        );
                        next.r_foot.orientation = Quaternion::rotation_x(-0.8);
                    },
                    StageSection::Recover => {
                        next.control.position = Vec3::new(-4.0, 25.0, -1.0);
                        next.control.orientation = Quaternion::rotation_x(-2.7)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(1.5);
                        next.chest.orientation = Quaternion::rotation_x(-0.3 + movement * 0.3)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(0.0);
                        next.head.orientation = Quaternion::rotation_x(0.2)
                            * Quaternion::rotation_y(-0.1)
                            * Quaternion::rotation_z(0.0);

                        next.l_hand.position = Vec3::new(-2.0, 0.0, 0.0);
                    },
                    _ => {},
                }
            }
        } else if let Some(ToolKind::Axe(_)) = active_tool_kind {
            //INTENTION: SWORD
            next.l_hand.position = Vec3::new(-0.75, -1.0, -2.5);
            next.l_hand.orientation = Quaternion::rotation_x(1.27);
            next.l_hand.scale = Vec3::one() * 1.04;
            next.r_hand.position = Vec3::new(0.75, -1.5, -5.5);
            next.r_hand.orientation = Quaternion::rotation_x(1.27);
            next.r_hand.scale = Vec3::one() * 1.05;
            //next.main.position = Vec3::new(0.0, 0.0, 10.0);
            next.main.position = Vec3::new(0.0, 0.0, -5.0);
            next.main.orientation = Quaternion::rotation_x(1.6)
                * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(-0.4);
            next.main.scale = Vec3::one();

            next.control.position = Vec3::new(-4.5 + spinhalf * 4.0, 11.0, 8.0);
            next.control.orientation = Quaternion::rotation_x(0.6 + spinhalf * -3.3)
                * Quaternion::rotation_y(0.2 + spin * -2.0)
                * Quaternion::rotation_z(1.4 + spin * 0.1);
            next.control.scale = Vec3::one();
            next.head.position = Vec3::new(
                0.0,
                -2.0 + skeleton_attr.head.0 + spin * -0.8,
                skeleton_attr.head.1,
            );
            next.head.orientation = Quaternion::rotation_z(spin * -0.25)
                * Quaternion::rotation_x(0.0 + spin * -0.1)
                * Quaternion::rotation_y(spin * -0.2);
            next.chest.position = Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1);
            next.chest.orientation = Quaternion::rotation_z(spin * 0.1)
                * Quaternion::rotation_x(0.0 + spin * 0.1)
                * Quaternion::rotation_y(decel * -0.2);
            next.chest.scale = Vec3::one();

            next.belt.position = Vec3::new(0.0, 0.0, -2.0);
            next.belt.orientation = next.chest.orientation * -0.1;
            next.belt.scale = Vec3::one();

            next.shorts.position = Vec3::new(0.0, 0.0, -5.0);
            next.belt.orientation = next.chest.orientation * -0.08;
            next.shorts.scale = Vec3::one();
            next.torso.position = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
            next.torso.orientation = Quaternion::rotation_z((spin * 7.0).max(0.3))
                * Quaternion::rotation_x(0.0)
                * Quaternion::rotation_y(0.3);
            next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

            // Stuff after the branch in the spin animation file

            next.l_foot.position =
                Vec3::new(-skeleton_attr.foot.0, foot * 1.0, skeleton_attr.foot.2);
            next.l_foot.orientation = Quaternion::rotation_x(foot * -1.2);
            next.l_foot.scale = Vec3::one();

            next.r_foot.position =
                Vec3::new(skeleton_attr.foot.0, foot * -1.0, skeleton_attr.foot.2);
            next.r_foot.orientation = Quaternion::rotation_x(foot * 1.2);
            next.r_foot.scale = Vec3::one();

            next.l_shoulder.position = Vec3::new(-5.0, 0.0, 4.7);
            next.l_shoulder.orientation = Quaternion::rotation_x(0.0);
            next.l_shoulder.scale = Vec3::one() * 1.1;

            next.r_shoulder.position = Vec3::new(5.0, 0.0, 4.7);
            next.r_shoulder.orientation = Quaternion::rotation_x(0.0);
            next.r_shoulder.scale = Vec3::one() * 1.1;

            next.glider.position = Vec3::new(0.0, 5.0, 0.0);
            next.glider.orientation = Quaternion::rotation_y(0.0);
            next.glider.scale = Vec3::one() * 0.0;

            next.lantern.position = Vec3::new(
                skeleton_attr.lantern.0,
                skeleton_attr.lantern.1,
                skeleton_attr.lantern.2,
            );
            next.lantern.orientation =
                Quaternion::rotation_x(spin * -0.7 + 0.4) * Quaternion::rotation_y(spin * 0.4);
            next.lantern.scale = Vec3::one() * 0.65;
            next.hold.scale = Vec3::one() * 0.0;

            next.l_control.position = Vec3::new(0.0, 0.0, 0.0);
            next.l_control.orientation = Quaternion::rotation_x(0.0);
            next.l_control.scale = Vec3::one();

            next.r_control.position = Vec3::new(0.0, 0.0, 0.0);
            next.r_control.orientation = Quaternion::rotation_x(0.0);
            next.r_control.scale = Vec3::one();
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
