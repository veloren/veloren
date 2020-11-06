use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};
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
        (active_tool_kind, _second_tool_kind, _velocity, _global_time, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (movement1, movement2, movement3, movement4) = match stage_section {
            Some(StageSection::Buildup) => (anim_time as f32, 0.0, 0.0, 0.0),
            Some(StageSection::Movement) => (1.0, (anim_time as f32).powf(0.25), 0.0, 0.0),
            Some(StageSection::Swing) => (1.0, 1.0, (anim_time as f32).powf(0.25), 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time as f32),
            _ => (0.0, 0.0, 0.0, 0.0),
        };

        if let Some(ToolKind::Hammer) = active_tool_kind {
            next.hand_l.position = Vec3::new(s_a.hhl.0, s_a.hhl.1, s_a.hhl.2);
            next.hand_l.orientation = Quaternion::rotation_x(s_a.hhl.3);
            next.hand_r.position = Vec3::new(s_a.hhr.0, s_a.hhr.1, s_a.hhr.2);
            next.hand_r.orientation = Quaternion::rotation_x(s_a.hhr.3);
            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_y(0.0) * Quaternion::rotation_z(0.0);

            next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

            next.control.position = Vec3::new(
                s_a.hc.0 + movement2 * -10.0 + movement3 * 6.0,
                s_a.hc.1 + movement2 * 5.0 + movement3 * 7.0,
                s_a.hc.2 + movement2 * 5.0 + movement3 * -10.0,
            );
            next.control.orientation =
                Quaternion::rotation_x(s_a.hc.3 + movement2 * 1.57 + movement3 * -2.3)
                    * Quaternion::rotation_y(s_a.hc.4 + movement2 * 1.3)
                    * Quaternion::rotation_z(s_a.hc.5 + movement2 * -1.0 + movement3 * 0.5);
            next.chest.orientation =
                Quaternion::rotation_x(
                    movement1 * 0.3 + movement2 * 0.3 + movement3 * -0.9 + movement4 * 0.3,
                ) * Quaternion::rotation_z(movement1 * 0.5 + movement2 * 0.2 + movement3 * -0.7);

            next.head.orientation = Quaternion::rotation_x(movement3 * 0.2)
                * Quaternion::rotation_y(0.0 + movement2 * -0.1)
                * Quaternion::rotation_z(movement1 * -0.4 + movement2 * -0.2 + movement3 * 0.6);

            //next.hand_l.position = Vec3::new(-12.0 + movement3 * 10.0, 0.0, 0.0);

            next.foot_l.position = Vec3::new(
                -s_a.foot.0,
                s_a.foot.1 + movement3 * 13.0,
                s_a.foot.2 + movement3 * -2.0,
            );
            next.foot_l.orientation = Quaternion::rotation_x(-0.8 + movement3 * 1.7);

            next.foot_r.position = Vec3::new(
                s_a.foot.0,
                s_a.foot.1 + 8.0 + movement3 * -13.0,
                s_a.foot.2 + 5.0 + movement3 * -5.0,
            );
            next.foot_r.orientation = Quaternion::rotation_x(0.9 + movement3 * -1.7);
        } else if let Some(ToolKind::Axe) = active_tool_kind {
            next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
            next.hand_l.orientation = Quaternion::rotation_x(s_a.ahl.3);
            next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
            next.hand_r.orientation = Quaternion::rotation_x(s_a.ahr.3);
            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_y(0.0) * Quaternion::rotation_z(0.0);

            next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

            next.control.position = Vec3::new(
                s_a.ac.0 + movement1 * 8.0,
                s_a.ac.1 + movement1 * 4.0 + movement3 * 3.0,
                s_a.ac.2 + movement1 * 6.0 + movement2 * 1.0 + movement3 * -14.0,
            );
            next.control.orientation = Quaternion::rotation_x(
                s_a.ac.3 + movement1 * -2.0 + movement2 * 0.7 + movement3 * -2.3
            ) * Quaternion::rotation_y(s_a.ac.4)// + movement1 * 0.5)
                * Quaternion::rotation_z(s_a.ac.5+movement1*PI); // - movement1 * 0.2);

            next.torso.orientation = Quaternion::rotation_x(
                -0.3 + movement2 * -1.6 * PI
                    + movement2 * -0.3
                    + movement3 * -0.2 * PI
                    + movement4 * -0.1 * PI,
            ) * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(0.0);

            next.head.orientation =
                Quaternion::rotation_x(0.0 + movement1 * -0.4 + movement2 * 0.4 + movement3 * 0.2);

            next.foot_l.position = Vec3::new(
                -s_a.foot.0,
                s_a.foot.1 + movement2 * 4.0 + movement3 * -1.0,
                s_a.foot.2,
            );

            next.foot_r.position = Vec3::new(
                s_a.foot.0,
                s_a.foot.1 + movement2 * 4.0 + movement3 * -8.0,
                s_a.foot.2 + movement3 * -3.0,
            );

            next.foot_l.orientation =
                Quaternion::rotation_x(movement1 * 0.9 - movement2 * 1.0 + movement3 * 1.8);

            next.foot_r.orientation = Quaternion::rotation_x(movement1 * 0.9 - movement3 * 1.8);

            next.belt.orientation = Quaternion::rotation_x(movement1 * 0.22 + movement2 * 0.1);
            next.shorts.orientation = Quaternion::rotation_x(movement1 * 0.3 + movement2 * 0.1);

            next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);
        }

        next
    }
}
