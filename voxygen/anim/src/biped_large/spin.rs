use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};
use core::f32::consts::PI;

pub struct SpinAnimation;

impl Animation for SpinAnimation {
    type Dependency<'a> = (
        Option<ToolKind>,
        Option<ToolKind>,
        f32,
        Option<StageSection>,
    );
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_spin\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_spin")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, _second_tool_kind, _global_time, stage_section): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let lab: f32 = 1.0;

        let (movement1, movement2, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time.powf(1.8), 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
            _ => (0.0, 0.0, 0.0),
        };

        let foot = ((5.0 / (1.1 + 3.9 * ((anim_time * lab * 10.32).sin()).powi(2))).sqrt())
            * ((anim_time * lab * 10.32).sin());

        let decel = (anim_time * 16.0 * lab).min(PI / 2.0).sin();

        let spin = (anim_time * 2.8 * lab).sin();
        let spinhalf = (anim_time * 1.4 * lab).sin();

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

        if let Some(ToolKind::Sword) = active_tool_kind {
            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_x(0.0);

            next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
            next.hand_l.orientation =
                Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
            next.hand_r.position = Vec3::new(s_a.shr.0, s_a.shr.1, s_a.shr.2);
            next.hand_r.orientation =
                Quaternion::rotation_x(s_a.shr.3) * Quaternion::rotation_y(s_a.shr.4);

            next.control.position = Vec3::new(
                s_a.sc.0 + movement1 * 2.0 + movement2 * -4.0 + movement3 * -7.0,
                s_a.sc.1 + 0.0 + movement1 * 0.6 + movement3 * -10.0,
                s_a.sc.2 + 5.0 + movement1 * 0.6 + movement2 * 1.5 + movement3 * -4.0,
            );
            next.control.orientation = Quaternion::rotation_x(-0.5 + s_a.sc.3 + movement1 * -1.2)
                * Quaternion::rotation_y(s_a.sc.4 - 0.6 + movement1 * 1.0)
                * Quaternion::rotation_z(s_a.sc.5 + 0.1 + movement1 * PI / 2.0);
            next.head.position = Vec3::new(
                0.0 + 2.0 + movement2 * -2.0,
                2.0 + movement2 * -2.0 + s_a.head.0,
                s_a.head.1,
            );
            next.head.orientation = Quaternion::rotation_z(movement2 * -0.4);

            next.upper_torso.orientation = Quaternion::rotation_x(movement2 * 0.15)
                * Quaternion::rotation_y(movement1 * -0.1 + movement2 * 0.3 + movement3 * -0.1)
                * Quaternion::rotation_z(
                    -1.0 + movement1 * -0.6 + movement2 * 1.5 + movement3 * 0.5,
                );

            next.lower_torso.orientation = Quaternion::rotation_x(movement1 * 0.1)
                * Quaternion::rotation_z(movement2.sin() * 1.5);

            next.head.orientation = Quaternion::rotation_y(movement1 * 0.1 - movement2 * -0.1)
                * Quaternion::rotation_z(1.07 + movement1 * 0.4 + movement2 * -1.5);

            next.torso.orientation = Quaternion::rotation_z(movement2 * std::f32::consts::TAU);
        }

        if let Some(ToolKind::Axe | ToolKind::Hammer | ToolKind::Dagger) = active_tool_kind {
            next.hand_l.position = Vec3::new(-0.75, -1.0, -2.5);
            next.hand_l.orientation = Quaternion::rotation_x(1.27);
            next.hand_r.position = Vec3::new(0.75, -1.5, -5.5);
            next.hand_r.orientation = Quaternion::rotation_x(1.27);
            next.main.position = Vec3::new(0.0, 6.0, -1.0);
            next.main.orientation = Quaternion::rotation_x(-0.3)
                * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(0.0);

            next.control.position = Vec3::new(-4.5 + spinhalf * 4.0, 11.0, 8.0);
            next.control.orientation = Quaternion::rotation_x(-1.7)
                * Quaternion::rotation_y(0.2 + spin * -2.0)
                * Quaternion::rotation_z(1.4 + spin * 0.1);
            next.head.position = Vec3::new(0.0, -1.0 + s_a.head.0 + spin * -0.8, s_a.head.1);
            next.head.orientation = Quaternion::rotation_z(spin * -0.25)
                * Quaternion::rotation_x(0.0 + spin * -0.1)
                * Quaternion::rotation_y(spin * -0.2);
            next.upper_torso.position = Vec3::new(0.0, s_a.upper_torso.0, s_a.upper_torso.1);
            next.upper_torso.orientation = Quaternion::rotation_z(spin * 0.1)
                * Quaternion::rotation_x(0.0 + spin * 0.1)
                * Quaternion::rotation_y(decel * -0.2);

            next.lower_torso.position = Vec3::new(0.0, 0.0, -5.0);
            next.torso.orientation = Quaternion::rotation_z((spin * 7.0).max(0.3));

            next.foot_l.position = Vec3::new(-s_a.foot.0, foot * 1.0, s_a.foot.2);
            next.foot_l.orientation = Quaternion::rotation_x(foot * -1.2);

            next.foot_r.position = Vec3::new(s_a.foot.0, foot * -1.0, s_a.foot.2);
            next.foot_r.orientation = Quaternion::rotation_x(foot * 1.2);
        }

        next
    }
}
