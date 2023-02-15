use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::{
    comp::item::tool::{AbilitySpec, ToolKind},
    states::utils::StageSection,
};
use core::f32::consts::PI;

pub struct SelfBuffAnimation;

impl Animation for SelfBuffAnimation {
    type Dependency<'a> = (
        (Option<ToolKind>, Option<&'a AbilitySpec>),
        (Option<ToolKind>, Option<&'a AbilitySpec>),
        Vec3<f32>,
        f32,
        Option<StageSection>,
        f32,
    );
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_selfbuff\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_selfbuff")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            (active_tool_kind, active_tool_spec),
            _second_tool,
            velocity,
            _global_time,
            stage_section,
            acc_vel,
        ): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();

        let lab: f32 = 0.65 * s_a.tempo;
        let speednorm = (speed / 12.0).powf(0.4);
        let foothoril = (acc_vel * lab + PI * 1.45).sin() * speednorm;
        let foothorir = (acc_vel * lab + PI * (0.45)).sin() * speednorm;
        let footrotl = ((1.0 / (0.5 + (0.5) * ((acc_vel * lab + PI * 1.4).sin()).powi(2))).sqrt())
            * ((acc_vel * lab + PI * 1.4).sin());

        let footrotr = ((1.0 / (0.5 + (0.5) * ((acc_vel * lab + PI * 0.4).sin()).powi(2))).sqrt())
            * ((acc_vel * lab + PI * 0.4).sin());
        let (move1base, movement3, tensionbase, tension2base) = match stage_section {
            Some(StageSection::Buildup) => (
                (anim_time.powf(0.25)).min(1.0),
                0.0,
                (anim_time * 10.0).sin(),
                0.0,
            ),
            Some(StageSection::Action) => {
                (1.0, 0.0, (anim_time * 30.0).sin(), (anim_time * 12.0).sin())
            },
            Some(StageSection::Recover) => (1.0, anim_time.powi(4), 1.0, 1.0),
            _ => (0.0, 0.0, 0.0, 0.0),
        };

        let pullback = 1.0 - movement3;
        let move1 = move1base * pullback;
        let tension = tensionbase * pullback;
        let tension2 = tension2base * pullback;

        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation = Quaternion::rotation_x(0.0);
        next.shoulder_l.position = Vec3::new(
            -s_a.shoulder.0,
            s_a.shoulder.1,
            s_a.shoulder.2 - foothorir * 1.0,
        );
        next.shoulder_l.orientation =
            Quaternion::rotation_x(move1 * 0.8 + 0.6 * speednorm + (footrotr * -0.2) * speednorm);

        next.shoulder_r.position = Vec3::new(
            s_a.shoulder.0,
            s_a.shoulder.1,
            s_a.shoulder.2 - foothoril * 1.0,
        );
        next.shoulder_r.orientation =
            Quaternion::rotation_x(move1 * 0.8 + 0.6 * speednorm + (footrotl * -0.2) * speednorm);
        next.torso.orientation = Quaternion::rotation_z(0.0);

        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_x(0.0);

        next.hand_l.position = Vec3::new(0.0, 0.0, s_a.grip.0);
        next.hand_r.position = Vec3::new(0.0, 0.0, s_a.grip.0);

        next.hand_l.orientation = Quaternion::rotation_x(0.0);
        next.hand_r.orientation = Quaternion::rotation_x(0.0);

        // TODO: Remove clippy allow when second species is added
        #[allow(clippy::single_match)]
        match active_tool_kind {
            Some(ToolKind::Axe) => {
                next.control_l.position = Vec3::new(-1.0, 2.0, 12.0);
                next.control_r.position = Vec3::new(1.0, 2.0, -2.0);

                next.control.position = Vec3::new(
                    4.0 + move1 * -20.0 + tension2 * 5.0,
                    0.0 + s_a.grip.0 / 1.0 + move1 * -5.0,
                    -s_a.grip.0 / 0.8 + move1 * 5.0,
                );
                next.jaw.orientation = Quaternion::rotation_x(move1 * -0.3 + tension2 * -0.15);
                next.head.orientation =
                    Quaternion::rotation_x(move1 * 0.3) * Quaternion::rotation_z(tension2 * 0.5);
                next.control_l.orientation = Quaternion::rotation_x(PI / 2.0 + move1 * 0.2)
                    * Quaternion::rotation_y(move1 * -1.0);
                next.control_r.orientation = Quaternion::rotation_x(PI / 2.0 + 0.2 + move1 * -0.2)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);

                next.control.orientation = Quaternion::rotation_x(-1.0 + move1 * 1.0)
                    * Quaternion::rotation_y(-1.8 + move1 * 1.2 + tension * 0.09)
                    * Quaternion::rotation_z(move1 * 1.5);
                next.shoulder_l.orientation =
                    Quaternion::rotation_x(move1 * -0.5) * Quaternion::rotation_y(move1 * 0.5);
                next.shoulder_r.orientation =
                    Quaternion::rotation_x(move1 * 0.6) * Quaternion::rotation_y(move1 * 0.4);
                next.upper_torso.orientation = Quaternion::rotation_z(tension2 * -0.08);
                next.lower_torso.orientation = Quaternion::rotation_z(tension2 * 0.08);
            },
            Some(ToolKind::Natural) => {
                if let Some(AbilitySpec::Custom(spec)) = active_tool_spec {
                    match spec.as_str() {
                        "Minotaur" => {
                            next.upper_torso.orientation =
                                Quaternion::rotation_x(move1 * -0.1 + tension2 * 0.05);
                            next.lower_torso.orientation =
                                Quaternion::rotation_x(move1 * 0.1 + tension2 * -0.05);

                            next.head.orientation =
                                Quaternion::rotation_x(move1 * 0.8 + tension2 * -0.1)
                                    * Quaternion::rotation_y(tension2 * -0.1);

                            next.control_l.position = Vec3::new(0.0, 4.0, 5.0);
                            next.control_r.position = Vec3::new(0.0, 4.0, 5.0);
                            next.weapon_l.position = Vec3::new(
                                -12.0 + move1 * -15.0,
                                -6.0 + move1 * 13.0,
                                -18.0 + move1 * 16.0 + tension2 * 3.0,
                            );
                            next.weapon_r.position = Vec3::new(
                                12.0 + move1 * 1.0,
                                -6.0 + move1 * 7.0 + tension * 0.3,
                                -18.0 + move1 * -2.0,
                            );
                            next.second.scale = Vec3::one() * 1.0;

                            next.weapon_l.orientation = Quaternion::rotation_x(-1.67 + move1 * 1.9)
                                * Quaternion::rotation_y(move1 * 0.25 + tension2 * 0.06)
                                * Quaternion::rotation_z(move1 * 1.3);
                            next.weapon_r.orientation = Quaternion::rotation_x(-1.67 + move1 * 0.8)
                                * Quaternion::rotation_y(move1 * -0.85 + tension * 0.12)
                                * Quaternion::rotation_z(move1 * 0.7);

                            next.control_l.orientation =
                                Quaternion::rotation_x(PI / 2.0 + move1 * 0.1)
                                    * Quaternion::rotation_y(0.0);
                            next.control_r.orientation =
                                Quaternion::rotation_x(PI / 2.0 + move1 * 0.1)
                                    * Quaternion::rotation_y(0.0);

                            next.control.orientation =
                                Quaternion::rotation_x(0.0) * Quaternion::rotation_y(0.0);
                            next.shoulder_l.orientation =
                                Quaternion::rotation_x(-0.3 + move1 * 2.2 + tension2 * 0.17)
                                    * Quaternion::rotation_y(move1 * 0.95);

                            next.shoulder_r.orientation =
                                Quaternion::rotation_x(-0.3 + move1 * 0.1)
                                    * Quaternion::rotation_y(move1 * -0.35);
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
