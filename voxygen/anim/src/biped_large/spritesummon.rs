use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{AbilitySpec, ToolKind},
    states::utils::StageSection,
};
use std::f32::consts::PI;

pub struct SpriteSummonAnimation;

type SpriteSummonAnimationDependency<'a> = (
    (Option<ToolKind>, Option<&'a AbilitySpec>),
    (Option<ToolKind>, Option<&'a AbilitySpec>),
    f32,
    f32,
    Option<StageSection>,
);
impl Animation for SpriteSummonAnimation {
    type Dependency<'a> = SpriteSummonAnimationDependency<'a>;
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_sprite_summon\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_sprite_summon")]
    #[allow(clippy::single_match)] // TODO: Pending review in #587
    fn update_skeleton_inner<'a>(
        skeleton: &Self::Skeleton,
        (
            (active_tool_kind, active_tool_spec),
            _second_tool_kind,
            _global_time,
            velocity,
            stage_section,
        ): Self::Dependency<'a>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_x(0.0);

        next.hand_l.position = Vec3::new(0.0, 0.0, s_a.grip.0);
        next.hand_r.position = Vec3::new(0.0, 0.0, s_a.grip.0);

        next.hand_l.orientation = Quaternion::rotation_x(0.0);
        next.hand_r.orientation = Quaternion::rotation_x(0.0);

        match active_tool_kind {
            Some(ToolKind::Natural) => {
                if let Some(AbilitySpec::Custom(spec)) = active_tool_spec {
                    match spec.as_str() {
                        "Harvester" => {
                            let (move1, move1pow, move2, move3) = match stage_section {
                                Some(StageSection::Buildup) => {
                                    (anim_time, anim_time.powf(0.1), 0.0, 0.0)
                                },
                                Some(StageSection::Cast) => {
                                    (1.0, 1.0, (anim_time.powf(4.0) * 80.0).min(1.0), 0.0)
                                },
                                Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time),
                                _ => (0.0, 0.0, 0.0, 0.0),
                            };

                            let speed = Vec2::<f32>::from(velocity).magnitude();

                            let pullback = 1.0 - move3;
                            let move1 = move1 * pullback;
                            let move1pow = move1pow * pullback;
                            let move2 = move2 * pullback;

                            next.head.orientation = Quaternion::rotation_x(move1 * 0.2);
                            next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
                            next.jaw.orientation = Quaternion::rotation_x(move2 * -0.3);

                            let twist = move1 * 0.8 + move3 * -0.8;
                            next.upper_torso.position = Vec3::new(
                                0.0,
                                s_a.upper_torso.0,
                                s_a.upper_torso.1 + move1 * 1.0 + move2 * -1.0,
                            );
                            next.upper_torso.orientation =
                                Quaternion::rotation_x(move1 * 0.8 + move2 * -1.1)
                                    * Quaternion::rotation_z(
                                        twist * -0.2 + move1 * -0.1 + move2 * 0.3,
                                    );

                            next.lower_torso.orientation =
                                Quaternion::rotation_x(move1 * -0.8 + move2 * 1.1)
                                    * Quaternion::rotation_z(-twist + move1 * 0.4);

                            next.control_l.position = Vec3::new(1.0, 2.0, 8.0);
                            next.control_r.position = Vec3::new(1.0, 1.0, -2.0);

                            next.control.position = Vec3::new(
                                -7.0 + move1pow * 7.0,
                                0.0 + s_a.grip.0 / 1.0 + move1pow * 12.0,
                                -s_a.grip.0 / 0.8 + move1pow * 20.0 + move2 * -3.0,
                            );

                            next.control_l.orientation =
                                Quaternion::rotation_x(PI / 2.0) * Quaternion::rotation_z(PI);
                            next.control_r.orientation = Quaternion::rotation_x(PI / 2.0 + 0.2)
                                * Quaternion::rotation_y(-1.0)
                                * Quaternion::rotation_z(0.0);

                            next.control.orientation =
                                Quaternion::rotation_x(-1.4 + move1pow * 2.2 + move2 * -0.6)
                                    * Quaternion::rotation_y(-PI);

                            next.shoulder_l.position =
                                Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
                            next.shoulder_l.orientation =
                                Quaternion::rotation_x(-0.4 + move1pow * 1.6);
                            next.shoulder_r.orientation = Quaternion::rotation_y(0.4)
                                * Quaternion::rotation_x(0.4 + move1pow * 1.0);

                            if speed == 0.0 {
                                next.leg_l.orientation =
                                    Quaternion::rotation_x(move1 * 0.8 + move2 * -0.8);

                                next.foot_l.position = Vec3::new(
                                    -s_a.foot.0,
                                    s_a.foot.1,
                                    s_a.foot.2 + move1 * 4.0 + move2 * -4.0,
                                );
                                next.foot_l.orientation =
                                    Quaternion::rotation_x(move1 * -0.6 + move2 * 0.6);
                            }
                        },
                        _ => {},
                    }
                }
            },
            _ => {},
        }
        next
    }
}
