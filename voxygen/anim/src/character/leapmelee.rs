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
        f32,
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
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (move1, move2, move3, move4) = match stage_section {
            Some(StageSection::Buildup) => (anim_time, 0.0, 0.0, 0.0),
            Some(StageSection::Movement) => (1.0, anim_time.powf(0.25), 0.0, 0.0),
            Some(StageSection::Swing) => (1.0, 1.0, anim_time.powf(0.25), 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0, 0.0),
        };

        if let Some(ToolKind::Hammer | ToolKind::Pick) = active_tool_kind {
            next.hand_l.position = Vec3::new(s_a.hhl.0, s_a.hhl.1, s_a.hhl.2);
            next.hand_l.orientation = Quaternion::rotation_x(s_a.hhl.3);
            next.hand_r.position = Vec3::new(s_a.hhr.0, s_a.hhr.1, s_a.hhr.2);
            next.hand_r.orientation = Quaternion::rotation_x(s_a.hhr.3);
            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_y(0.0) * Quaternion::rotation_z(0.0);

            next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

            next.control.position = Vec3::new(
                s_a.hc.0 + move2 * -10.0 + move3 * 6.0,
                s_a.hc.1 + move2 * 5.0 + move3 * 7.0,
                s_a.hc.2 + move2 * 5.0 + move3 * -10.0,
            );
            next.control.orientation =
                Quaternion::rotation_x(s_a.hc.3 + move2 * 1.57 + move3 * -2.3)
                    * Quaternion::rotation_y(s_a.hc.4 + move2 * 1.3)
                    * Quaternion::rotation_z(s_a.hc.5 + move2 * -1.0 + move3 * 0.5);
            next.chest.orientation =
                Quaternion::rotation_x(move1 * 0.3 + move2 * 0.3 + move3 * -0.9 + move4 * 0.3)
                    * Quaternion::rotation_z(move1 * 0.5 + move2 * 0.2 + move3 * -0.7);

            next.head.orientation = Quaternion::rotation_x(move3 * 0.2)
                * Quaternion::rotation_y(0.0 + move2 * -0.1)
                * Quaternion::rotation_z(move1 * -0.4 + move2 * -0.2 + move3 * 0.6);

            next.foot_l.position = Vec3::new(
                -s_a.foot.0,
                s_a.foot.1 + move3 * 13.0,
                s_a.foot.2 + move3 * -2.0,
            );
            next.foot_l.orientation = Quaternion::rotation_x(-0.8 + move3 * 1.7);

            next.foot_r.position = Vec3::new(
                s_a.foot.0,
                s_a.foot.1 + 8.0 + move3 * -13.0,
                s_a.foot.2 + 5.0 + move3 * -5.0,
            );
            next.foot_r.orientation = Quaternion::rotation_x(0.9 + move3 * -1.7);
        } else if let Some(ToolKind::Axe) = active_tool_kind {
            next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
            next.hand_l.orientation = Quaternion::rotation_x(s_a.ahl.3);
            next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
            next.hand_r.orientation = Quaternion::rotation_x(s_a.ahr.3);
            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_y(0.0) * Quaternion::rotation_z(0.0);

            next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

            next.control.position = Vec3::new(
                s_a.ac.0 + move1 * 8.0,
                s_a.ac.1 + move1 * 4.0 + move3 * 3.0,
                s_a.ac.2 + move1 * 6.0 + move2 * 1.0 + move3 * -14.0,
            );
            next.control.orientation = Quaternion::rotation_x(
                s_a.ac.3 + move1 * -2.0 + move2 * 0.7 + move3 * -2.3
            ) * Quaternion::rotation_y(s_a.ac.4)// + move1 * 0.5)
                * Quaternion::rotation_z(s_a.ac.5+move1*PI); // - move1 * 0.2);

            next.torso.orientation = Quaternion::rotation_x(
                -0.3 + move2 * -1.6 * PI + move2 * -0.3 + move3 * -0.2 * PI + move4 * -0.1 * PI,
            ) * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(0.0);

            next.head.orientation =
                Quaternion::rotation_x(0.0 + move1 * -0.4 + move2 * 0.4 + move3 * 0.2);

            next.foot_l.position = Vec3::new(
                -s_a.foot.0,
                s_a.foot.1 + move2 * 4.0 + move3 * -1.0,
                s_a.foot.2,
            );

            next.foot_r.position = Vec3::new(
                s_a.foot.0,
                s_a.foot.1 + move2 * 4.0 + move3 * -8.0,
                s_a.foot.2 + move3 * -3.0,
            );

            next.foot_l.orientation =
                Quaternion::rotation_x(move1 * 0.9 - move2 * 1.0 + move3 * 1.8);

            next.foot_r.orientation = Quaternion::rotation_x(move1 * 0.9 - move3 * 1.8);

            next.belt.orientation = Quaternion::rotation_x(move1 * 0.22 + move2 * 0.1);
            next.shorts.orientation = Quaternion::rotation_x(move1 * 0.3 + move2 * 0.1);

            next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);
        }

        next
    }
}
