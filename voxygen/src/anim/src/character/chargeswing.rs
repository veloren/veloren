use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::StageSection,
};
use std::f32::consts::PI;
pub struct ChargeswingAnimation;

impl Animation for ChargeswingAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        Vec3<f32>,
        f64,
        Option<StageSection>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_chargeswing\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_chargeswing")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, velocity, _global_time, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();

        let lab = 1.0;

        let short = (((5.0)
            / (1.5 + 3.5 * ((anim_time as f32 * lab as f32 * 8.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 8.0).sin());
        // end spin stuff

        let movement = anim_time as f32 * 1.0;
        let fire = (anim_time as f32 * 18.0 * lab as f32).sin();

        let foothoril = (anim_time as f32 * 8.0 * lab as f32 + PI * 1.45).sin();
        let foothorir = (anim_time as f32 * 8.0 * lab as f32 + PI * (0.45)).sin();

        let footvertl = (anim_time as f32 * 8.0 * lab as f32).sin();
        let footvertr = (anim_time as f32 * 8.0 * lab as f32 + PI).sin();
        let footrotl = (((1.0)
            / (0.5
                + (0.5)
                    * ((anim_time as f32 * 8.0 * lab as f32 + PI * 1.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 8.0 * lab as f32 + PI * 1.4).sin());

        let footrotr = (((1.0)
            / (0.5
                + (0.5)
                    * ((anim_time as f32 * 8.0 * lab as f32 + PI * 0.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 8.0 * lab as f32 + PI * 0.4).sin());
        if let Some(ToolKind::Hammer(_)) = active_tool_kind {
            next.hand_l.position = Vec3::new(-12.0, 0.0, 0.0);
            next.hand_l.orientation = Quaternion::rotation_x(-0.0) * Quaternion::rotation_y(0.0);
            next.hand_r.position = Vec3::new(2.0, 0.0, 0.0);
            next.hand_r.orientation = Quaternion::rotation_x(0.0) * Quaternion::rotation_y(0.0);
            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_y(-1.57) * Quaternion::rotation_z(1.57);

            next.control.position = Vec3::new(6.0, 7.0, 1.0);
            next.control.orientation = Quaternion::rotation_x(0.3)
                * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(0.0);
            if let Some(stage_section) = stage_section {
                match stage_section {
                    StageSection::Charge => {
                        next.control.position = Vec3::new(
                            6.0 + (movement * -4.0).max(-8.0),
                            7.0 + (movement * 2.0).min(2.0),
                            1.0,
                        );
                        next.control.orientation = Quaternion::rotation_x(0.3)
                            * Quaternion::rotation_y(
                                0.0 + (movement * 0.7).min(0.7)
                                    + fire * 0.1 * (anim_time as f32).min(2.0),
                            )
                            * Quaternion::rotation_z(0.0 + (movement * 0.2).min(0.5));

                        next.chest.position =
                            Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1);
                        next.chest.orientation =
                            Quaternion::rotation_z((movement * 2.0).min(PI / 2.0));
                        next.belt.orientation =
                            Quaternion::rotation_z((short * 0.08 + movement * -1.0).max(-PI / 5.0));
                        next.shorts.orientation =
                            Quaternion::rotation_z((short * 0.15 + movement * -1.0).max(-PI / 4.0));

                        next.head.position = Vec3::new(
                            0.0,
                            skeleton_attr.head.0 - 2.0 + (movement * 2.0).min(2.0),
                            skeleton_attr.head.1,
                        );

                        next.head.orientation =
                            Quaternion::rotation_z((movement * -1.8).max(PI / -2.0));
                        next.belt.orientation = Quaternion::rotation_z(short * 0.05);

                        next.shorts.orientation = Quaternion::rotation_z(short * 0.15);
                        if speed > 0.5 {
                            next.foot_l.position = Vec3::new(
                                -skeleton_attr.foot.0,
                                skeleton_attr.foot.1 + foothoril * -2.5 - 3.5,
                                skeleton_attr.foot.2 + ((footvertl * -1.2).max(-1.0)),
                            );

                            next.foot_r.position = Vec3::new(
                                skeleton_attr.foot.0,
                                skeleton_attr.foot.1 + foothorir * -2.5 + 6.0,
                                skeleton_attr.foot.2 + ((footvertr * -1.2).max(-1.0)),
                            );

                            next.foot_l.orientation =
                                Quaternion::rotation_x(-0.4 + footrotl * -0.2)
                                    * Quaternion::rotation_z((movement * 0.5).min(0.5));

                            next.foot_r.orientation =
                                Quaternion::rotation_x(-0.4 + footrotr * -0.2)
                                    * Quaternion::rotation_z((movement * 0.5).min(0.5));
                        } else {
                            next.foot_l.position = Vec3::new(
                                -skeleton_attr.foot.0,
                                skeleton_attr.foot.1 - 5.0,
                                skeleton_attr.foot.2,
                            );

                            next.foot_r.position = Vec3::new(
                                skeleton_attr.foot.0,
                                skeleton_attr.foot.1 + 7.0,
                                skeleton_attr.foot.2,
                            );

                            next.foot_l.orientation =
                                Quaternion::rotation_x(-0.2) * Quaternion::rotation_z(0.5);

                            next.foot_r.orientation =
                                Quaternion::rotation_x(0.2) * Quaternion::rotation_z(0.5);
                        };
                    },

                    StageSection::Swing => {
                        next.chest.orientation = Quaternion::rotation_z(-0.5);
                        next.control.position = Vec3::new(6.0, 7.0, 1.0 + 3.0);
                        next.control.orientation = Quaternion::rotation_x(PI / 2.0)
                            * Quaternion::rotation_y(-1.6)
                            * Quaternion::rotation_z(0.3 - movement * 2.5);
                        next.head.orientation = Quaternion::rotation_z(0.8);
                        next.hand_l.position = Vec3::new(-3.0, 0.0, 0.0);
                    },
                    StageSection::Recover => {
                        next.chest.orientation = Quaternion::rotation_z(-0.5 + movement * 0.5);
                        next.control.position = Vec3::new(6.0, 7.0, 1.0 + 3.0 + movement * -3.0);
                        next.control.orientation = Quaternion::rotation_x(PI / 2.0)
                            * Quaternion::rotation_y(-1.6 + movement * 1.6)
                            * Quaternion::rotation_z(-2.2 + movement * 2.2);
                        next.head.orientation = Quaternion::rotation_z(0.8 + movement * -0.8);
                        next.hand_l.position = Vec3::new(-3.0 + movement * -9.0, 0.0, 0.0);
                    },
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
        next
    }
}
