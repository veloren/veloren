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
pub struct DashAnimation;

impl Animation for DashAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        f64,
        Option<StageSection>,
    );
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_dash\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_dash")]
    #[allow(clippy::single_match)] // TODO: Pending review in #587
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

        let slow = (((5.0)
            / (1.1 + 3.9 * ((anim_time as f32 * lab as f32 * 12.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 12.4).sin());

        let short = (((5.0)
            / (1.5 + 3.5 * ((anim_time as f32 * lab as f32 * 5.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 5.0).sin());
        let foothoril = (anim_time as f32 * 5.0 * lab as f32 + PI * 1.45).sin();
        let foothorir = (anim_time as f32 * 5.0 * lab as f32 + PI * (0.45)).sin();

        let footvertl = (anim_time as f32 * 5.0 * lab as f32).sin();
        let footvertr = (anim_time as f32 * 5.0 * lab as f32 + PI).sin();

        let footrotl = (((1.0)
            / (0.05
                + (0.95)
                    * ((anim_time as f32 * 5.0 * lab as f32 + PI * 1.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 5.0 * lab as f32 + PI * 1.4).sin());

        let footrotr = (((1.0)
            / (0.05
                + (0.95)
                    * ((anim_time as f32 * 5.0 * lab as f32 + PI * 0.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 5.0 * lab as f32 + PI * 0.4).sin());

        let shortalt = (anim_time as f32 * lab as f32 * 5.0 + PI / 2.0).sin();

        let movement = (anim_time as f32 * 1.0).min(1.0);

        next.head.position = Vec3::new(0.0, -2.0 + skeleton_attr.head.0, skeleton_attr.head.1);

        next.hand_l.position = Vec3::new(-0.75, -1.0, 2.5);
        next.hand_l.orientation = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(-0.2);
        next.hand_l.scale = Vec3::one() * 1.02;
        next.hand_r.position = Vec3::new(0.75, -1.5, -0.5);
        next.hand_r.orientation = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(0.3);
        next.hand_r.scale = Vec3::one() * 1.02;
        next.main.position = Vec3::new(0.0, 0.0, 2.0);
        next.main.orientation = Quaternion::rotation_x(-0.1)
            * Quaternion::rotation_y(0.0)
            * Quaternion::rotation_z(0.0);

        match active_tool_kind {
            //TODO: Inventory
            Some(ToolKind::Sword(_)) => {
                if let Some(stage_section) = stage_section {
                    match stage_section {
                        StageSection::Buildup => {
                            next.head.orientation = Quaternion::rotation_z(movement * -0.9);

                            next.upper_torso.orientation = Quaternion::rotation_z(movement * 1.1);

                            next.control.position = Vec3::new(-7.0 + movement * -5.0, 7.0, 2.0);
                            next.control.orientation = Quaternion::rotation_x(movement * -1.0)
                                * Quaternion::rotation_y(movement * 1.5)
                                * Quaternion::rotation_z(0.0);
                            next.control.scale = Vec3::one();
                            next.foot_l.position = Vec3::new(
                                -skeleton_attr.foot.0,
                                skeleton_attr.foot.1 + movement * -12.0,
                                skeleton_attr.foot.2,
                            ) / 8.0;
                            next.foot_l.orientation = Quaternion::rotation_x(movement * -1.0);
                            next.foot_r.position = Vec3::new(
                                skeleton_attr.foot.0,
                                skeleton_attr.foot.1,
                                skeleton_attr.foot.2,
                            ) / 8.0;
                        },
                        StageSection::Charge => {
                            next.head.position = Vec3::new(
                                0.0,
                                -2.0 + skeleton_attr.head.0,
                                skeleton_attr.head.1 + movement * 1.0,
                            );

                            next.head.orientation = Quaternion::rotation_x(0.0)
                                * Quaternion::rotation_y(movement * -0.3)
                                * Quaternion::rotation_z(-0.9 + movement * -0.2 + short * -0.05);
                            next.upper_torso.position = Vec3::new(
                                0.0,
                                skeleton_attr.upper_torso.0,
                                skeleton_attr.upper_torso.1 + 2.0 + shortalt * -2.5,
                            );

                            next.upper_torso.orientation = Quaternion::rotation_x(movement * -0.4)
                                * Quaternion::rotation_y(movement * -0.2)
                                * Quaternion::rotation_z(1.1);

                            next.control.position =
                                Vec3::new(-13.0, 7.0 + movement * -2.0, 2.0 + movement * 2.0);
                            next.control.orientation =
                                Quaternion::rotation_x(-1.0) * Quaternion::rotation_y(1.5);
                            next.control.scale = Vec3::one();

                            next.upper_torso.orientation = Quaternion::rotation_z(short * 0.25);
                            
                            next.foot_l.position = Vec3::new(
                                2.0 - skeleton_attr.foot.0,
                                skeleton_attr.foot.1 + foothoril * -7.5,
                                2.0 + skeleton_attr.foot.2 + ((footvertl * -4.0).max(-1.0)),
                            ) / 8.0;
                            next.foot_l.orientation =
                                Quaternion::rotation_x(-0.6 + footrotl * -0.6)
                                    * Quaternion::rotation_z(-0.2);

                            next.foot_r.position = Vec3::new(
                                2.0 + skeleton_attr.foot.0,
                                skeleton_attr.foot.1 + foothorir * -7.5,
                                2.0 + skeleton_attr.foot.2 + ((footvertr * -4.0).max(-1.0)),
                            ) / 8.0;
                            next.foot_r.orientation =
                                Quaternion::rotation_x(-0.6 + footrotr * -0.6)
                                    * Quaternion::rotation_z(-0.2);
                        },
                        StageSection::Swing => {
                            next.head.orientation = Quaternion::rotation_y(0.2 + movement * -0.2)
                                * Quaternion::rotation_z(-1.1 + movement * 1.8);

                            next.upper_torso.orientation = Quaternion::rotation_y(-0.2 + movement * 0.3)
                                * Quaternion::rotation_z(1.1 + movement * -2.2);

                            next.control.position = Vec3::new(-13.0 + movement * -2.0, 5.0, 4.0);
                            next.control.orientation =
                                Quaternion::rotation_x(-1.0 + movement * -0.5)
                                    * Quaternion::rotation_y(1.5 + movement * -2.5);
                            next.control.scale = Vec3::one();
                        },
                        StageSection::Recover => {
                            next.head.orientation = Quaternion::rotation_z(0.7);

                            next.upper_torso.orientation = Quaternion::rotation_z(-1.1);

                            next.control.position = Vec3::new(-15.0, 5.0, 2.0);
                            next.control.orientation =
                                Quaternion::rotation_x(-1.5) * Quaternion::rotation_y(-1.0);
                            next.control.scale = Vec3::one();
                        },
                        _ => {},
                    }
                }
            },
            Some(ToolKind::Dagger(_)) => {
                next.head.position = Vec3::new(
                    0.0,
                    -2.0 + skeleton_attr.head.0,
                    -2.0 + skeleton_attr.head.1,
                );
                next.head.orientation = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0);
                next.head.scale = Vec3::one() * 1.01;

                next.upper_torso.position = Vec3::new(0.0, 0.0, 7.0 + slow * 2.0);
                next.upper_torso.orientation =
                    Quaternion::rotation_x(-0.5) * Quaternion::rotation_z(-0.7);

                next.upper_torso.position = Vec3::new(0.0, 3.0, -3.0);
                next.upper_torso.orientation = Quaternion::rotation_x(0.4) * Quaternion::rotation_z(0.3);

                next.hand_l.position = Vec3::new(-0.75, -1.0, -2.5);
                next.hand_l.orientation = Quaternion::rotation_x(1.27);
                next.hand_l.scale = Vec3::one() * 1.02;
                next.hand_r.position = Vec3::new(0.75, -1.5, -5.5);
                next.hand_r.orientation = Quaternion::rotation_x(1.27);
                next.hand_r.scale = Vec3::one() * 1.02;
                next.main.position = Vec3::new(0.0, 6.0, -1.0);
                next.main.orientation = Quaternion::rotation_x(-0.3);
                next.main.scale = Vec3::one();

                next.control.position = Vec3::new(-8.0 - slow * 0.5, 3.0, 3.0);
                next.control.orientation =
                    Quaternion::rotation_x(-0.3) * Quaternion::rotation_z(1.1 + slow * 0.2);
                next.control.scale = Vec3::one();
                next.foot_l.position = Vec3::new(-1.4, 2.0, skeleton_attr.foot.2);
                next.foot_l.orientation = Quaternion::rotation_x(-0.8);

                next.foot_r.position = Vec3::new(5.4, -1.0, skeleton_attr.foot.2);
                next.foot_r.orientation = Quaternion::rotation_x(-0.8);
            },
            _ => {},
        }

        match second_tool_kind {
            //TODO: Inventory
            Some(ToolKind::Dagger(_)) => {
                next.head.position = Vec3::new(
                    0.0,
                    -2.0 + skeleton_attr.head.0,
                    -2.0 + skeleton_attr.head.1,
                );
                next.head.orientation = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0);
                next.head.scale = Vec3::one() * 1.01;

                next.upper_torso.position = Vec3::new(0.0, 0.0, 7.0 + slow * 2.0);
                next.upper_torso.orientation = Quaternion::rotation_x(0.0);

                next.upper_torso.position = Vec3::new(0.0, 3.0, -3.0);
                next.upper_torso.orientation = Quaternion::rotation_x(0.0);

                next.control.position = Vec3::new(0.0, 0.0, 0.0);
                next.control.orientation = Quaternion::rotation_x(0.0);
                next.control.scale = Vec3::one();

                next.hand_l.position = Vec3::new(0.0, 0.0, 0.0);
                next.hand_l.orientation = Quaternion::rotation_x(0.0);
                next.hand_l.scale = Vec3::one() * 1.04;

                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);
                next.main.scale = Vec3::one();

                next.hand_r.position = Vec3::new(0.0, 0.0, 0.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.0);
                next.hand_r.scale = Vec3::one() * 1.05;

                next.second.position = Vec3::new(0.0, 6.0, -1.0);
                next.second.orientation = Quaternion::rotation_x(-0.3);
                next.second.scale = Vec3::one();

                next.foot_l.position = Vec3::new(-1.4, 2.0, skeleton_attr.foot.2);
                next.foot_l.orientation = Quaternion::rotation_x(-0.8);

                next.foot_r.position = Vec3::new(5.4, -1.0, skeleton_attr.foot.2);
                next.foot_r.orientation = Quaternion::rotation_x(-0.8);
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
