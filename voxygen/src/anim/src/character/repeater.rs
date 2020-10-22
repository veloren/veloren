use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::StageSection,
};
pub struct RepeaterAnimation;

impl Animation for RepeaterAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        Vec3<f32>,
        f64,
        Option<StageSection>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_repeater\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_repeater")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, _velocity, _global_time, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (movement1, movement2, movement3, _movement4) = match stage_section {
            Some(StageSection::Movement) => (anim_time as f32, 0.0, 0.0, 0.0),
            Some(StageSection::Buildup) => (1.0, anim_time as f32, 0.0, 0.0),
            Some(StageSection::Shoot) => (1.0, 1.0, anim_time as f32, 0.0),
            Some(StageSection::Recover) => (1.1, 1.0, 1.0, anim_time as f32),
            _ => (0.0, 0.0, 0.0, 0.0),
        };

        // end spin stuff

        fn fire(x: f32) -> f32 { (x * 18.0).sin() }

        if let Some(ToolKind::Bow(_)) = active_tool_kind {
            next.hand_l.position = Vec3::new(2.0, 1.5, 0.0);
            next.hand_l.orientation = Quaternion::rotation_x(1.20)
                * Quaternion::rotation_y(-0.6)
                * Quaternion::rotation_z(-0.3);
            next.hand_r.position = Vec3::new(5.9, 4.5, -5.0);
            next.hand_r.orientation = Quaternion::rotation_x(1.20)
                * Quaternion::rotation_y(-0.6)
                * Quaternion::rotation_z(-0.3);
            next.main.position = Vec3::new(3.0, 2.0, -13.0);
            next.main.orientation = Quaternion::rotation_x(-0.3)
                * Quaternion::rotation_y(0.3)
                * Quaternion::rotation_z(-0.6);

            next.hold.position = Vec3::new(1.2, -1.0, -5.2);
            next.hold.orientation = Quaternion::rotation_x(-1.7)
                * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(-0.1);
            next.hold.scale = Vec3::one() * 1.0;

            next.control.position = Vec3::new(-7.0, 6.0, 6.0);
            next.control.orientation = Quaternion::rotation_x(0.0) * Quaternion::rotation_z(0.0);

            next.foot_l.position = Vec3::new(
                -s_a.foot.0 + movement1 * -0.75 - 0.75,
                s_a.foot.1 + movement1 * 4.0 + 4.0,
                s_a.foot.2 + movement1 * 2.5 + 2.5,
            );
            next.foot_l.orientation =
                Quaternion::rotation_x(movement1 * 0.6 + 0.6 + movement2 * -0.2)
                    * Quaternion::rotation_z(movement1 * 0.3 + 0.3);

            next.foot_r.position = Vec3::new(
                s_a.foot.0 + movement1 * 0.75 + 0.75,
                s_a.foot.1 + movement1 * 4.0 + 4.0,
                s_a.foot.2 + movement1 * 2.5 + 2.5,
            );
            next.foot_r.orientation =
                Quaternion::rotation_x(movement1 * 0.6 + 0.6 + movement2 * -0.2)
                    * Quaternion::rotation_z(movement1 * -0.3 - 0.3);
            next.shorts.position = Vec3::new(
                0.0,
                s_a.shorts.0 + movement1 * 4.0,
                s_a.shorts.1 + movement1 * 1.0,
            );
            next.shorts.orientation = Quaternion::rotation_x(movement1 * 0.6);
            next.belt.position = Vec3::new(0.0, s_a.belt.0 + movement1 * 2.0, s_a.belt.1);
            next.belt.orientation = Quaternion::rotation_x(movement1 * 0.2);
            next.control.position = Vec3::new(
                -7.0 + movement1 * 5.0,
                6.0 + movement1 * 3.0,
                6.0 + movement1 * 1.0,
            );
            next.control.orientation =
                Quaternion::rotation_x(movement1 * 0.4) * Quaternion::rotation_y(movement1 * 0.8);
            next.head.orientation = Quaternion::rotation_y(movement1 * 0.15 + movement2 * 0.05);
            next.torso.orientation =
                Quaternion::rotation_x(movement1 * 0.1 + movement2 * 0.1 + movement3 * 0.15);

            next.hand_l.position = Vec3::new(
                2.0 + fire(movement3) * -6.0 - 3.0,
                1.5 + fire(movement3) * -6.0 - 3.0,
                0.0,
            );
            next.hand_l.orientation = Quaternion::rotation_x(1.20)
                * Quaternion::rotation_y(-0.6)
                * Quaternion::rotation_z(-0.3);
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
