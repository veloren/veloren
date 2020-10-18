use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::StageSection,
};
use std::f32::consts::PI;

pub struct Input {
    pub attack: bool,
}
pub struct SpinAnimation;

impl Animation for SpinAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        f64,
        Option<StageSection>,
    );
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_spin\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_spin")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, _global_time, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let lab = 1.0;

        let foot = (((5.0)
            / (1.1 + 3.9 * ((anim_time as f32 * lab as f32 * 10.32).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 10.32).sin());

        let decel = (anim_time as f32 * 16.0 * lab as f32).min(PI / 2.0).sin();

        let spin = (anim_time as f32 * 2.8 * lab as f32).sin();
        let spinhalf = (anim_time as f32 * 1.4 * lab as f32).sin();

        let build = (anim_time as f32 * 8.0).sin();
        let recover = (anim_time as f32 * 8.0).sin();

        let movement = anim_time as f32 * 1.0;
        let stab = (anim_time as f32 * 8.0).sin();
        let rotate = (anim_time as f32 * 1.0).sin();

        next.head.position = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1);

        if let Some(ToolKind::Sword(_)) = active_tool_kind {
            next.hand_l.position = Vec3::new(-4.75, -1.0, 2.5);
            next.hand_l.orientation = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(-0.2);
            next.hand_l.scale = Vec3::one() * 1.02;
            next.hand_r.position = Vec3::new(0.75, -1.5, -0.5);
            next.hand_r.orientation = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(0.3);
            next.hand_r.scale = Vec3::one() * 1.02;
            next.main.position = Vec3::new(0.0, 5.0, -6.0);
            next.main.orientation = Quaternion::rotation_x(-0.1)
                * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(0.0);

            if let Some(stage_section) = stage_section {
                match stage_section {
                    StageSection::Buildup => {
                        next.control.position =
                            Vec3::new(5.0, 11.0 + build * 0.6, 2.0 + build * 0.6);
                        next.control.orientation = Quaternion::rotation_x(0.0)
                            * Quaternion::rotation_y(-0.57 + movement * 2.0)
                            * Quaternion::rotation_z(0.0);
                        next.upper_torso.orientation = Quaternion::rotation_y(movement * -0.1)
                            * Quaternion::rotation_z(-1.07 + movement * -0.6);

                        next.lower_torso.orientation = Quaternion::rotation_x(movement * 0.1);

                        next.head.orientation = Quaternion::rotation_y(movement * 0.1)
                            * Quaternion::rotation_z(1.07 + movement * 0.4);
                    },
                    StageSection::Swing => {
                        next.control.position = Vec3::new(
                            7.0 + movement * -8.0,
                            11.0 + stab * 3.0,
                            2.0 + stab * 3.5 + movement * 3.0,
                        );
                        next.control.orientation =
                            Quaternion::rotation_x(-1.57 + movement * -0.6 + stab * -0.25)
                                * Quaternion::rotation_y(2.8 + movement * -2.0)
                                * Quaternion::rotation_z(1.0 + movement * 1.0);
                        next.head.orientation = Quaternion::rotation_z(-stab * 0.8);
                        next.upper_torso.orientation = Quaternion::rotation_x(stab * 0.15)
                            * Quaternion::rotation_y(movement * 0.3)
                            * Quaternion::rotation_z(movement * 1.5);
                        next.lower_torso.orientation = Quaternion::rotation_z(rotate * 1.5);
                        next.torso.orientation = Quaternion::rotation_z(rotate * 7.2);
                    },
                    StageSection::Recover => {
                        next.control.position = Vec3::new(
                            -8.0,
                            11.0 - recover * 0.8 + movement * -10.0,
                            6.0 - recover * 0.4 + movement * -4.0,
                        );
                        next.control.orientation = Quaternion::rotation_x(-1.57)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(1.0);
                        next.upper_torso.orientation = Quaternion::rotation_y(movement * -0.1)
                            * Quaternion::rotation_z(movement * 0.4);
                        next.head.orientation = Quaternion::rotation_y(movement * 0.1)
                            * Quaternion::rotation_z(movement * -0.1);
                    },
                    _ => {},
                }
            }
        }

        if let Some(ToolKind::Axe(_) | ToolKind::Hammer(_) | ToolKind::Dagger(_)) = active_tool_kind
        {
            next.hand_l.position = Vec3::new(-0.75, -1.0, -2.5);
            next.hand_l.orientation = Quaternion::rotation_x(1.27);
            next.hand_l.scale = Vec3::one() * 1.04;
            next.hand_r.position = Vec3::new(0.75, -1.5, -5.5);
            next.hand_r.orientation = Quaternion::rotation_x(1.27);
            next.hand_r.scale = Vec3::one() * 1.05;
            next.main.position = Vec3::new(0.0, 6.0, -1.0);
            next.main.orientation = Quaternion::rotation_x(-0.3)
                * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(0.0);
            next.main.scale = Vec3::one();

            next.control.position = Vec3::new(-4.5 + spinhalf * 4.0, 11.0, 8.0);
            next.control.orientation = Quaternion::rotation_x(-1.7)
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
            next.upper_torso.position = Vec3::new(
                0.0,
                skeleton_attr.upper_torso.0,
                skeleton_attr.upper_torso.1,
            );
            next.upper_torso.orientation = Quaternion::rotation_z(spin * 0.1)
                * Quaternion::rotation_x(0.0 + spin * 0.1)
                * Quaternion::rotation_y(decel * -0.2);
            next.upper_torso.scale = Vec3::one();

            next.lower_torso.position = Vec3::new(0.0, 0.0, -5.0);
            next.lower_torso.scale = Vec3::one();
            next.torso.position = Vec3::new(0.0, 0.0, 0.1) * 1.01;
            next.torso.orientation = Quaternion::rotation_z((spin * 7.0).max(0.3))
                * Quaternion::rotation_x(0.0)
                * Quaternion::rotation_y(0.0);
            next.torso.scale = Vec3::one() / 11.0 * 1.01;

            next.foot_l.position =
                Vec3::new(-skeleton_attr.foot.0, foot * 1.0, skeleton_attr.foot.2);
            next.foot_l.orientation = Quaternion::rotation_x(foot * -1.2);
            next.foot_l.scale = Vec3::one();

            next.foot_r.position =
                Vec3::new(skeleton_attr.foot.0, foot * -1.0, skeleton_attr.foot.2);
            next.foot_r.orientation = Quaternion::rotation_x(foot * 1.2);
            next.foot_r.scale = Vec3::one();

            next.shoulder_l.position = Vec3::new(-5.0, 0.0, 4.7);
            next.shoulder_l.orientation = Quaternion::rotation_x(0.0);
            next.shoulder_l.scale = Vec3::one() * 1.1;

            next.shoulder_r.position = Vec3::new(5.0, 0.0, 4.7);
            next.shoulder_r.orientation = Quaternion::rotation_x(0.0);
            next.shoulder_r.scale = Vec3::one() * 1.1;
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
