use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};
pub struct RepeaterAnimation;

impl Animation for RepeaterAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        Vec3<f32>,
        f32,
        Option<StageSection>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_repeater\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_repeater")]
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

        let (move1, move2, move3, _move4) = match stage_section {
            Some(StageSection::Movement) => (anim_time, 0.0, 0.0, 0.0),
            Some(StageSection::Buildup) => (1.0, anim_time, 0.0, 0.0),
            Some(StageSection::Shoot) => (1.0, 1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.1, 1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0, 0.0),
        };

        // end spin stuff

        fn fire(x: f32) -> f32 { (x * 18.0).sin() }

        if let Some(ToolKind::Bow) = active_tool_kind {
            next.hand_l.position = Vec3::new(s_a.bhl.0, s_a.bhl.1, s_a.bhl.2);
            next.hand_l.orientation = Quaternion::rotation_x(s_a.bhl.3);
            next.hand_r.position = Vec3::new(s_a.bhr.0, s_a.bhr.1, s_a.bhr.2);
            next.hand_r.orientation = Quaternion::rotation_x(s_a.bhr.3);
            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_y(0.0) * Quaternion::rotation_z(0.0);

            next.hold.position = Vec3::new(1.2, -1.0, -5.2);
            next.hold.orientation = Quaternion::rotation_x(-1.7) * Quaternion::rotation_z(-0.1);
            next.hold.scale = Vec3::one() * 1.0 * (1.0 - move3);

            next.foot_l.position = Vec3::new(
                -s_a.foot.0 + move1 * -0.75 - 0.75,
                s_a.foot.1 + move1 * 4.0 + 4.0,
                s_a.foot.2 + move1 * 2.5 + 2.5,
            );
            next.foot_l.orientation = Quaternion::rotation_x(move1 * 0.6 + 0.6 + move2 * -0.2)
                * Quaternion::rotation_z(move1 * 0.3 + 0.3);

            next.foot_r.position = Vec3::new(
                s_a.foot.0 + move1 * 0.75 + 0.75,
                s_a.foot.1 + move1 * 4.0 + 4.0,
                s_a.foot.2 + move1 * 2.5 + 2.5,
            );
            next.foot_r.orientation = Quaternion::rotation_x(move1 * 0.6 + 0.6 + move2 * -0.2)
                * Quaternion::rotation_z(move1 * -0.3 - 0.3);
            next.shorts.position =
                Vec3::new(0.0, s_a.shorts.0 + move1 * 4.0, s_a.shorts.1 + move1 * 1.0);
            next.shorts.orientation = Quaternion::rotation_x(move1 * 0.6);
            next.belt.position = Vec3::new(0.0, s_a.belt.0 + move1 * 2.0, s_a.belt.1);
            next.belt.orientation = Quaternion::rotation_x(move1 * 0.2);
            next.control.position = Vec3::new(
                s_a.bc.0 + move1 * 5.0,
                s_a.bc.1 + move1 * 3.0,
                s_a.bc.2 + move1 * 1.0,
            );
            next.control.orientation = Quaternion::rotation_x(s_a.bc.3 + move1 * 0.4)
                * Quaternion::rotation_y(s_a.bc.4 + move1 * 0.8)
                * Quaternion::rotation_z(s_a.bc.5);
            next.head.orientation = Quaternion::rotation_y(move1 * 0.15 + move2 * 0.05);
            next.torso.orientation =
                Quaternion::rotation_x(move1 * 0.1 + move2 * 0.1 + move3 * 0.15);

            next.hand_l.position = Vec3::new(
                2.0 + fire(move3) * -6.0 - 3.0,
                1.5 + fire(move3) * -6.0 - 3.0,
                0.0,
            );
            next.hand_l.orientation = Quaternion::rotation_x(1.20)
                * Quaternion::rotation_y(-0.6)
                * Quaternion::rotation_z(-0.3);
        }

        next
    }
}
