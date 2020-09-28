use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::StageSection,
};
use std::f32::consts::PI;

pub struct AlphaAnimation;

impl Animation for AlphaAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        f32,
        f64,
        Option<StageSection>,
    );
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_alpha\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_alpha")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, velocity, _global_time, stage_section): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab = 0.55;

        let slower = (((1.0)
            / (0.05
                + 0.95
                    * ((anim_time as f32 * lab as f32 * 10.0 - 0.8 * PI).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 10.0 - 0.8 * PI).sin());

        let slowersmooth = (anim_time as f32 * lab as f32 * 4.0).sin();

        let foothoril = (((1.0)
            / (0.4
                + (0.6)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin());
        let foothorir = (((1.0)
            / (0.4
                + (0.6)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin());

        let footrotl = (((5.0)
            / (2.5
                + (2.5)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin());

        let footrotr = (((5.0)
            / (1.0
                + (4.0)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin());

        let short = (anim_time as f32 * lab as f32 * 16.0).sin();

        next.head.position = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1) * 1.02;
        next.head.orientation = Quaternion::rotation_z(slower * 1.0) * Quaternion::rotation_x(0.0);
        next.head.scale = Vec3::one() * 1.02;

        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_x(0.0)
            * Quaternion::rotation_y(-1.57)
            * Quaternion::rotation_z(1.0);
        next.main.scale = Vec3::one() * 1.02;

        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation =
            Quaternion::rotation_x(PI) * Quaternion::rotation_y(0.0) * Quaternion::rotation_z(0.0);
        next.second.scale = Vec3::one() * 0.0;

        next.hand_l.position = Vec3::new(
            -skeleton_attr.hand.0 - 7.0,
            skeleton_attr.hand.1 - 7.0,
            skeleton_attr.hand.2 + 10.0,
        );
        next.hand_l.orientation = Quaternion::rotation_x(0.57) * Quaternion::rotation_z(1.57);
        next.hand_l.scale = Vec3::one() * 1.02;

        next.hand_r.position = Vec3::new(
            skeleton_attr.hand.0 - 7.0,
            skeleton_attr.hand.1 - 7.0,
            skeleton_attr.hand.2 + 10.0,
        );
        next.hand_r.orientation = Quaternion::rotation_x(0.57) * Quaternion::rotation_z(1.57);
        next.hand_r.scale = Vec3::one() * 1.02;

        next.upper_torso.position = Vec3::new(
            0.0,
            skeleton_attr.upper_torso.0,
            skeleton_attr.upper_torso.1,
        );
        next.upper_torso.orientation =
            Quaternion::rotation_z(slower * -1.2) * Quaternion::rotation_x(-0.3);
        next.upper_torso.scale = Vec3::one();

        next.control.position = Vec3::new(7.0, 9.0, -10.0);
        next.control.orientation = Quaternion::rotation_x(slowersmooth * 0.35)
            * Quaternion::rotation_y(0.0)
            * Quaternion::rotation_z(slowersmooth * -0.5 + slower * -0.5);
        next.control.scale = Vec3::one();
        if velocity < 0.5 {
            next.lower_torso.position = Vec3::new(
                0.0,
                skeleton_attr.lower_torso.0,
                skeleton_attr.lower_torso.1,
            );
            next.lower_torso.orientation =
                Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.2);
            next.lower_torso.scale = Vec3::one() * 1.02;

            next.jaw.position = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1 * 0.0);
            next.jaw.orientation = Quaternion::rotation_z(0.0);
            next.jaw.scale = Vec3::one();

            next.tail.position = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
            next.tail.orientation = Quaternion::rotation_z(0.0);
            next.tail.scale = Vec3::one();

            next.shoulder_l.position = Vec3::new(
                -skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
            next.shoulder_l.scale = Vec3::one();

            next.shoulder_r.position = Vec3::new(
                skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
            next.shoulder_r.scale = Vec3::one();

            next.leg_l.position = Vec3::new(
                -skeleton_attr.leg.0,
                skeleton_attr.leg.1,
                skeleton_attr.leg.2,
            ) * 1.02;
            next.leg_l.orientation = Quaternion::rotation_z(0.0);
            next.leg_l.scale = Vec3::one() * 1.02;

            next.leg_r.position = Vec3::new(
                skeleton_attr.leg.0,
                skeleton_attr.leg.1,
                skeleton_attr.leg.2,
            ) * 1.02;
            next.leg_r.orientation = Quaternion::rotation_z(0.0);
            next.leg_r.scale = Vec3::one() * 1.02;

            next.foot_l.position = Vec3::new(
                -skeleton_attr.foot.0,
                skeleton_attr.foot.1,
                skeleton_attr.foot.2,
            ) / 8.0;
            next.foot_l.orientation = Quaternion::rotation_z(0.0);
            next.foot_l.scale = Vec3::one() / 8.0;

            next.foot_r.position = Vec3::new(
                skeleton_attr.foot.0,
                skeleton_attr.foot.1,
                skeleton_attr.foot.2,
            ) / 8.0;
            next.foot_r.orientation = Quaternion::rotation_z(0.0);
            next.foot_r.scale = Vec3::one() / 8.0;

            next.torso.position = Vec3::new(0.0, 0.0, 0.0) / 8.0;
            next.torso.orientation = Quaternion::rotation_z(0.0);
            next.torso.scale = Vec3::one() / 8.0;
        } else {
            next.lower_torso.position = Vec3::new(
                0.0,
                skeleton_attr.lower_torso.0,
                skeleton_attr.lower_torso.1,
            );
            next.lower_torso.orientation =
                Quaternion::rotation_z(short * 0.15) * Quaternion::rotation_x(0.14);
            next.lower_torso.scale = Vec3::one() * 1.02;

            next.shoulder_l.position = Vec3::new(
                -skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1 + foothoril * -1.0,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_l.orientation = Quaternion::rotation_x(0.5 + footrotl * -0.16)
                * Quaternion::rotation_y(0.1)
                * Quaternion::rotation_z(footrotl * 0.1);
            next.shoulder_l.scale = Vec3::one();

            next.shoulder_r.position = Vec3::new(
                skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1 + foothorir * -1.0,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_r.orientation = Quaternion::rotation_x(0.5 + footrotr * -0.16)
                * Quaternion::rotation_y(-0.1)
                * Quaternion::rotation_z(footrotr * -0.1);
            next.shoulder_r.scale = Vec3::one();

            next.torso.position = Vec3::new(0.0, 0.0, 0.0) / 8.0;
            next.torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.25);
            next.torso.scale = Vec3::one() / 8.0;
        }

        next
    }
}
