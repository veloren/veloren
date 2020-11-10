use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};
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
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_alpha\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_alpha")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, _second_tool_kind, velocity, _global_time, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let lab = 1.0;

        let (movement1, movement2, movement3) = match stage_section {
            Some(StageSection::Buildup) => ((anim_time as f32).powf(0.25), 0.0, 0.0),
            Some(StageSection::Swing) => (1.0, anim_time as f32, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, (anim_time as f32).powf(4.0)),
            _ => (0.0, 0.0, 0.0),
        };

        let foot = (((1.0)
            / (0.2
                + 0.8
                    * ((anim_time as f32 * lab as f32 * 2.0 * velocity).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 2.0 * velocity).sin());
        let push = anim_time as f32 * lab as f32 * 4.0;
        let slow = (((5.0)
            / (0.4 + 4.6 * ((anim_time as f32 * lab as f32 * 9.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 9.0).sin());
        let slower = (((1.0)
            / (0.0001 + 0.999 * ((anim_time as f32 * lab as f32 * 4.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 4.0).sin());

        next.torso.position = Vec3::new(0.0, 0.0, 0.1) * s_a.scaler;
        next.torso.orientation = Quaternion::rotation_z(0.0);
        match active_tool_kind {
            Some(ToolKind::Sword) => {
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position = Vec3::new(s_a.shr.0, s_a.shr.1, s_a.shr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.shr.3) * Quaternion::rotation_y(s_a.shr.4);

                next.control.position = Vec3::new(
                    s_a.sc.0,
                    s_a.sc.1 + movement1 * -4.0 + movement2 * 16.0 + movement3 * -4.0,
                    s_a.sc.2 + movement1 * 1.0,
                );
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3 + movement1 * -0.5)
                    * Quaternion::rotation_y(
                        s_a.sc.4 + movement1 * -1.0 + movement2 * -0.6 + movement3 * 1.0,
                    )
                    * Quaternion::rotation_z(s_a.sc.5 + movement1 * -1.2 + movement2 * 1.3);

                next.chest.orientation = Quaternion::rotation_z(
                    movement1 * 1.5 + (movement2 * 1.75).sin() * -3.0 + movement3 * 0.5,
                );

                next.head.position = Vec3::new(0.0 + movement2 * 2.0, s_a.head.0, s_a.head.1);
                next.head.orientation = Quaternion::rotation_z(
                    movement1 * -0.9 + (movement2 * 1.75).sin() * 2.5 + movement3 * -0.5,
                );
            },
            Some(ToolKind::Dagger) => {
                next.control_l.position = Vec3::new(-10.0 + push * 5.0, 6.0 + push * 5.0, 2.0);
                next.control_l.orientation = Quaternion::rotation_x(-1.4 + slow * 0.4)
                    * Quaternion::rotation_y(slow * -1.3)
                    * Quaternion::rotation_z(1.4 + slow * -0.5);
            },
            Some(ToolKind::Axe) => {
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);
                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.head.position = Vec3::new(
                    0. + movement2 * 2.0,
                    s_a.head.0 + movement2 * 2.0,
                    s_a.head.1,
                );

                let (movement1, movement2, movement3) = match stage_section {
                    Some(StageSection::Buildup) => ((anim_time as f32).powf(0.25), 0.0, 0.0),
                    Some(StageSection::Swing) => (1.0, anim_time as f32, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, (anim_time as f32).powf(4.0)),
                    _ => (0.0, 0.0, 0.0),
                };
                next.control.position = Vec3::new(
                    s_a.ac.0 + movement1 * -1.0 + movement2 * -2.0 + movement3 * 0.0,
                    s_a.ac.1 + movement1 * -3.0 + movement2 * 3.0 + movement3 * -3.5,
                    s_a.ac.2 + movement1 * 6.0 + movement2 * -15.0 + movement3 * -2.0,
                );
                next.control.orientation = Quaternion::rotation_x(
                    s_a.ac.3 + movement1 * 0.0 + movement2 * -3.0 + movement3 * 0.4,
                ) * Quaternion::rotation_y(
                    s_a.ac.4 + movement1 * -0.0 + movement2 * -0.6 + movement3 * 0.8,
                ) * Quaternion::rotation_z(
                    s_a.ac.5 + movement1 * -2.0 + movement2 * -1.0 + movement3 * 2.5,
                );
                next.control.scale = Vec3::one();

                next.chest.orientation = Quaternion::rotation_x(
                    0.0 + movement1 * 0.6 + movement2 * -0.6 + movement3 * 0.4,
                ) * Quaternion::rotation_y(
                    0.0 + movement1 * 0.0 + movement2 * 0.0 + movement3 * 0.0,
                ) * Quaternion::rotation_z(
                    0.0 + movement1 * 1.5 + movement2 * -2.5 + movement3 * 1.5,
                );
                next.head.orientation = Quaternion::rotation_z(
                    0.0 + movement1 * -1.5 + movement2 * 2.5 + movement3 * -1.0,
                );
            },
            Some(ToolKind::Hammer) => {
                let (movement1, movement2, movement3) = match stage_section {
                    Some(StageSection::Buildup) => ((anim_time as f32).powf(0.25), 0.0, 0.0),
                    Some(StageSection::Swing) => (1.0, anim_time as f32, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, (anim_time as f32).powf(4.0)),
                    _ => (0.0, 0.0, 0.0),
                };
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);
                next.hand_l.position = Vec3::new(s_a.hhl.0, s_a.hhl.1, s_a.hhl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.hhl.3) * Quaternion::rotation_y(s_a.hhl.4);
                next.hand_r.position = Vec3::new(s_a.hhr.0, s_a.hhr.1, s_a.hhr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.hhr.3) * Quaternion::rotation_y(s_a.hhr.4);

                next.control.position = Vec3::new(
                    s_a.hc.0 + (movement1 * -13.0) * (1.0 - movement3),
                    s_a.hc.1 + (movement2 * 5.0) * (1.0 - movement3),
                    s_a.hc.2,
                );
                next.control.orientation =
                    Quaternion::rotation_x(s_a.hc.3 + (movement1 * 1.5 + movement2 * -2.5))
                        * (1.0 - movement3)
                        * Quaternion::rotation_y(s_a.hc.4 + (movement1 * 1.57))
                        * (1.0 - movement3)
                        * Quaternion::rotation_z(s_a.hc.5 + (movement2 * -0.5) * (1.0 - movement3));
                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
                next.head.orientation = Quaternion::rotation_x(
                    (movement1 * 0.3 + movement2 * -0.5) * (1.0 - movement3),
                ) * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(
                        (movement1 * 0.2 + movement2 * -0.5) * (1.0 - movement3),
                    );
                next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);
                next.chest.orientation = Quaternion::rotation_x(
                    (movement1 * 0.8 + movement2 * -1.2) * (1.0 - movement3),
                ) * Quaternion::rotation_y(
                    (movement1 * 0.3 + movement2 * -0.4) * (1.0 - movement3),
                ) * Quaternion::rotation_z(
                    (movement1 * 0.5 + movement2 * -0.5) * (1.0 - movement3),
                );

                if velocity > 0.5 {
                    next.foot_l.position = Vec3::new(-s_a.foot.0, foot * -6.0, s_a.foot.2);
                    next.foot_l.orientation = Quaternion::rotation_x(foot * -0.4)
                        * Quaternion::rotation_z((slower * 0.3).max(0.0));

                    next.foot_r.position = Vec3::new(s_a.foot.0, foot * 6.0, s_a.foot.2);
                    next.foot_r.orientation = Quaternion::rotation_x(foot * 0.4)
                        * Quaternion::rotation_z((slower * 0.3).max(0.0));
                    next.torso.orientation = Quaternion::rotation_x(-0.15);
                } else {
                    next.foot_l.position =
                        Vec3::new(-s_a.foot.0, -2.5, s_a.foot.2 + (slower * 2.5).max(0.0));
                    next.foot_l.orientation = Quaternion::rotation_x(slower * -0.2 - 0.2)
                        * Quaternion::rotation_z((slower * 1.0).max(0.0));

                    next.foot_r.position = Vec3::new(s_a.foot.0, 3.5 - slower * 2.0, s_a.foot.2);
                    next.foot_r.orientation = Quaternion::rotation_x(slower * 0.1)
                        * Quaternion::rotation_z((slower * 0.5).max(0.0));

                    next.belt.orientation =
                        Quaternion::rotation_x(movement1 * -0.2 + movement2 * 0.2);
                    next.shorts.orientation =
                        Quaternion::rotation_x(movement1 * -0.3 + movement2 * 0.3);
                }
            },
            Some(ToolKind::Debug) => {
                next.hand_l.position = Vec3::new(-7.0, 4.0, 3.0);
                next.hand_l.orientation = Quaternion::rotation_x(1.27);
                next.main.position = Vec3::new(-5.0, 5.0, 23.0);
                next.main.orientation = Quaternion::rotation_x(PI);
            },
            _ => {},
        }
        next
    }
}
