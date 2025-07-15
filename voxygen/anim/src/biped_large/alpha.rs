use super::{
    super::{Animation, vek::*},
    BipedLargeSkeleton, SkeletonAttr, biped_large_alpha_axe, biped_large_alpha_hammer,
    biped_large_alpha_sword, init_biped_large_alpha, init_gigas_fire,
};
use common::{
    comp::item::tool::{AbilitySpec, ToolKind},
    states::utils::StageSection,
};
use core::f32::consts::PI;

pub struct AlphaAnimation;

impl Animation for AlphaAnimation {
    type Dependency<'a> = (
        Option<ToolKind>,
        (Option<ToolKind>, Option<&'a AbilitySpec>),
        Vec3<f32>,
        f32,
        Option<StageSection>,
        f32,
        f32,
        Option<&'a str>,
    );
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_alpha\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "biped_large_alpha"))]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            active_tool_kind,
            _second_tool,
            velocity,
            global_time,
            stage_section,
            acc_vel,
            timer,
            ability_id,
        ): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (move1base, move2base, move3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - move3;
        let move1 = move1base * pullback;
        let move2 = move2base * pullback;

        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let speed = Vec2::<f32>::from(velocity).magnitude();

        let foothorir = init_biped_large_alpha(&mut next, s_a, speed, acc_vel, move1);

        match active_tool_kind {
            Some(ToolKind::Sword) => match ability_id {
                Some("common.abilities.custom.gigas_fire.fast_slash") => {
                    init_gigas_fire(&mut next);

                    next.head.orientation.rotate_z(-PI / 9.0 * move1base);
                    next.torso.orientation.rotate_z(PI / 5.0 * move1base);
                    next.lower_torso.orientation.rotate_z(-PI / 9.0 * move1base);
                    next.control.position += Vec3::new(-10.0, 4.0, 5.0) * move1base;
                    next.control.orientation.rotate_x(PI / 4.0 * move1);
                    next.shoulder_l.position += Vec3::new(3.0, 3.0, 0.0) * move1base;
                    next.shoulder_l.orientation.rotate_z(-PI / 8.0 * move1base);
                    next.shoulder_r.position += Vec3::new(-3.0, 3.0, 0.0) * move1base;
                    next.shoulder_r.orientation.rotate_z(PI / 4.0 * move1base);
                    next.control_l.orientation.rotate_x(-PI / 4.0 * move1);
                    next.control_r.orientation.rotate_z(PI / 3.0 * move1);
                    next.foot_l.position += Vec3::new(-2.0, -2.0, 0.0) * move1;
                    next.foot_l.orientation.rotate_z(PI / 5.0 * move1);

                    next.head
                        .orientation
                        .rotate_z(PI / 9.0 * move2base + PI / 9.0 * move2);
                    next.torso
                        .orientation
                        .rotate_z(-PI / 5.0 * move2base - PI / 5.0 * move2);
                    next.lower_torso
                        .orientation
                        .rotate_z(PI / 9.0 * move2base + PI / 9.0 * move2);
                    next.control.position += Vec3::new(10.0, -4.0, -5.0) * move2base;
                    next.control.orientation.rotate_x(-PI / 1.2 * move2);
                    next.control.orientation.rotate_y(-PI / 4.0 * move2);
                    next.control.orientation.rotate_z(PI / 8.0 * move2);
                    next.shoulder_l.position += Vec3::new(-3.0, -3.0, 0.0) * move2base;
                    next.shoulder_r.position += Vec3::new(3.0, -3.0, 0.0) * move2base;
                    next.shoulder_r.orientation.rotate_z(-PI / 4.0 * move2base);
                    next.control_l.orientation.rotate_x(PI / 3.0 * move2);
                    next.control_l.orientation.rotate_y(PI / 6.0 * move2);
                    next.control_l.orientation.rotate_z(-PI / 8.0 * move2);
                    next.control_r.orientation.rotate_x(PI / 3.0 * move2);
                    next.control_r.orientation.rotate_y(PI / 6.0 * move2);
                    next.control_r.orientation.rotate_z(PI / 6.0 * move2);
                    next.leg_l.position += Vec3::new(1.0, 1.0, 0.0) * move2;
                    next.leg_l.orientation.rotate_x(PI / 8.0 * move2);
                    next.foot_l.position += Vec3::new(4.0, 4.0, 0.0) * move2;
                    next.foot_l.orientation.rotate_z(-PI / 5.0 * move2);
                    next.foot_r.position += Vec3::new(2.0, -2.0, 0.0) * move2;
                    next.foot_r.orientation.rotate_z(-PI / 5.0 * move2);
                },
                // Mirrored version of the fast slash
                Some("common.abilities.custom.gigas_fire.slow_slash") => {
                    init_gigas_fire(&mut next);

                    next.head.orientation.rotate_z(PI / 9.0 * move1base);
                    next.torso.orientation.rotate_z(-PI / 5.0 * move1base);
                    next.lower_torso.orientation.rotate_z(PI / 9.0 * move1base);
                    next.control.position += Vec3::new(10.0, 4.0, 5.0) * move1base;
                    next.control.orientation.rotate_x(PI / 4.0 * move1);
                    next.shoulder_l.position += Vec3::new(3.0, 3.0, 0.0) * move1base;
                    next.shoulder_l.orientation.rotate_z(-PI / 4.0 * move1base);
                    next.shoulder_r.position += Vec3::new(-3.0, 3.0, 0.0) * move1base;
                    next.shoulder_r.orientation.rotate_z(PI / 8.0 * move1base);
                    next.control_l.orientation.rotate_z(-PI / 3.0 * move1);
                    next.control_r.orientation.rotate_x(-PI / 4.0 * move1);
                    next.foot_r.position += Vec3::new(2.0, -2.0, 0.0) * move1;
                    next.foot_r.orientation.rotate_z(-PI / 5.0 * move1);

                    next.head
                        .orientation
                        .rotate_z(-PI / 9.0 * move2base - PI / 9.0 * move2);
                    next.torso
                        .orientation
                        .rotate_z(PI / 5.0 * move2base + PI / 5.0 * move2);
                    next.lower_torso
                        .orientation
                        .rotate_z(-PI / 9.0 * move2base - PI / 9.0 * move2);
                    next.control.position += Vec3::new(-10.0, -4.0, -5.0) * move2base;
                    next.control.orientation.rotate_x(-PI / 1.2 * move2);
                    next.control.orientation.rotate_y(PI / 4.0 * move2);
                    next.control.orientation.rotate_z(-PI / 8.0 * move2);
                    next.shoulder_l.position += Vec3::new(-3.0, -3.0, 0.0) * move2base;
                    next.shoulder_l.orientation.rotate_z(PI / 4.0 * move2base);
                    next.shoulder_r.position += Vec3::new(3.0, -3.0, 0.0) * move2base;
                    next.control_l.orientation.rotate_x(PI / 3.0 * move2);
                    next.control_l.orientation.rotate_y(-PI / 6.0 * move2);
                    next.control_l.orientation.rotate_z(-PI / 6.0 * move2);
                    next.control_r.orientation.rotate_x(PI / 3.0 * move2);
                    next.control_r.orientation.rotate_y(-PI / 6.0 * move2);
                    next.control_r.orientation.rotate_z(PI / 8.0 * move2);
                    next.foot_l.position += Vec3::new(-2.0, -2.0, 0.0) * move2;
                    next.foot_l.orientation.rotate_z(PI / 5.0 * move2);
                    next.leg_r.position += Vec3::new(-1.0, 1.0, 0.0) * move2;
                    next.leg_r.orientation.rotate_x(PI / 8.0 * move2);
                    next.foot_r.position += Vec3::new(-4.0, 4.0, 0.0) * move2;
                    next.foot_r.orientation.rotate_z(PI / 5.0 * move2);
                },
                Some("common.abilities.custom.gigas_fire.fast_thrust") => {
                    init_gigas_fire(&mut next);

                    next.foot_l.position += Vec3::new(-1.0, -1.0, 0.0) * move1;
                    next.foot_l.orientation.rotate_z(PI / 5.0 * move1);
                    next.leg_r.position += Vec3::new(1.0, 1.0, 0.0) * move1;
                    next.leg_r.orientation.rotate_x(PI / 8.0 * move1);
                    next.foot_r.position += Vec3::new(3.0, 3.0, 0.0) * move1;
                    next.foot_r.orientation.rotate_z(-PI / 6.0 * move1);
                    next.torso.orientation.rotate_z(PI / 2.0 * move1);
                    next.lower_torso.orientation.rotate_z(-PI / 4.0 * move1);
                    next.head.orientation.rotate_z(-PI / 4.0 * move1);
                    next.control.orientation.rotate_y(PI / 2.0 * move1);
                    next.control.orientation.rotate_x(-PI / 2.5 * move1);
                    next.shoulder_l.position += Vec3::new(5.0, 5.0, 0.0) * move1;
                    next.shoulder_l.orientation.rotate_z(-PI / 4.0 * move1);
                    next.control_l.orientation.rotate_x(PI / 3.0 * move1);
                    next.control_l.orientation.rotate_y(-PI / 6.0 * move1);
                    next.shoulder_r.position += Vec3::new(-2.0, 2.0, 0.0) * move1;
                    next.shoulder_r.orientation.rotate_z(PI / 6.0 * move1);
                    next.control_r.orientation.rotate_z(PI / 4.0 * move1);
                    next.control_r.orientation.rotate_y(-PI / 4.0 * move1);

                    next.leg_r.orientation.rotate_x(-PI / 8.0 * move2);
                    next.foot_r.position += Vec3::new(-2.0, -2.0, 0.0) * move2;
                    next.foot_r.orientation.rotate_z(PI / 5.0 * move2);
                    next.foot_l.position += Vec3::new(-1.0, 2.0, 0.0) * move2;
                    next.torso.orientation.rotate_z(-PI / 1.5 * move2);
                    next.lower_torso.orientation.rotate_z(PI / 4.0 * move2);
                    next.head.orientation.rotate_z(PI / 4.0 * move2);
                    next.control.position += Vec3::new(-10.0, 5.0, -8.0) * move2;
                    next.control
                        .orientation
                        .rotate_x(PI / 2.5 * (4.0 * move2).min(1.0));
                    next.control.orientation.rotate_y(PI / 20.0 * move2);
                    next.control.orientation.rotate_z(PI / 1.7 * move2);
                    next.main.position += Vec3::new(-3.0, 0.0, -3.0) * move2;
                    next.shoulder_l.position += Vec3::new(-2.0, -2.0, 0.0) * move2;
                    next.shoulder_l.orientation.rotate_z(PI / 6.0 * move2);
                    next.control_l.orientation.rotate_x(-PI / 8.0 * move2);
                    next.control_l.orientation.rotate_y(PI / 6.0 * move2);
                    next.control_l.orientation.rotate_z(-PI / 1.8 * move2);
                    next.control_r.orientation.rotate_x(PI / 3.0 * move2);
                    next.control_r.orientation.rotate_z(-PI / 4.0 * move2);
                },
                // Mirrored version of the fast thrust
                Some("common.abilities.custom.gigas_fire.slow_thrust") => {
                    init_gigas_fire(&mut next);

                    next.leg_l.position += Vec3::new(-1.0, 1.0, 0.0) * move1;
                    next.leg_l.orientation.rotate_x(PI / 8.0 * move1);
                    next.foot_l.position += Vec3::new(-3.0, 3.0, 0.0) * move1;
                    next.foot_l.orientation.rotate_z(PI / 6.0 * move1);
                    next.foot_r.position += Vec3::new(1.0, -1.0, 0.0) * move1;
                    next.foot_r.orientation.rotate_z(-PI / 5.0 * move1);
                    next.torso.orientation.rotate_z(-PI / 2.0 * move1);
                    next.lower_torso.orientation.rotate_z(PI / 4.0 * move1);
                    next.head.orientation.rotate_z(PI / 4.0 * move1);
                    next.control.orientation.rotate_y(-PI / 2.0 * move1);
                    next.control.orientation.rotate_x(-PI / 2.5 * move1);
                    next.shoulder_l.position += Vec3::new(2.0, 2.0, 0.0) * move1;
                    next.shoulder_l.orientation.rotate_z(-PI / 6.0 * move1);
                    next.control_l.orientation.rotate_z(-PI / 4.0 * move1);
                    next.control_l.orientation.rotate_y(PI / 4.0 * move1);
                    next.shoulder_r.position += Vec3::new(-5.0, 5.0, 0.0) * move1;
                    next.shoulder_r.orientation.rotate_z(PI / 4.0 * move1);
                    next.control_r.orientation.rotate_x(PI / 3.0 * move1);
                    next.control_r.orientation.rotate_y(PI / 6.0 * move1);

                    next.foot_r.position += Vec3::new(1.0, 2.0, 0.0) * move2;
                    next.leg_l.orientation.rotate_x(-PI / 8.0 * move2);
                    next.foot_l.position += Vec3::new(2.0, -2.0, 0.0) * move2;
                    next.foot_l.orientation.rotate_z(-PI / 5.0 * move2);
                    next.torso.orientation.rotate_z(PI / 1.5 * move2);
                    next.lower_torso.orientation.rotate_z(-PI / 4.0 * move2);
                    next.head.orientation.rotate_z(-PI / 4.0 * move2);
                    next.control.position += Vec3::new(10.0, 5.0, -8.0) * move2;
                    next.control
                        .orientation
                        .rotate_x(PI / 2.5 * (4.0 * move2).min(1.0));
                    next.control.orientation.rotate_y(-PI / 20.0 * move2);
                    next.control.orientation.rotate_z(-PI / 1.7 * move2);
                    next.main.position += Vec3::new(3.0, 0.0, -3.0) * move2;
                    next.control_l.orientation.rotate_x(PI / 3.0 * move2);
                    next.control_l.orientation.rotate_z(PI / 3.0 * move2);
                    next.shoulder_r.position += Vec3::new(2.0, -2.0, 0.0) * move2;
                    next.shoulder_r.orientation.rotate_z(-PI / 6.0 * move2);
                    next.control_r.orientation.rotate_x(-PI / 8.0 * move2);
                    next.control_r.orientation.rotate_y(-PI / 6.0 * move2);
                    next.control_r.orientation.rotate_z(PI / 1.8 * move2);
                },
                Some("common.abilities.custom.gigas_fire.whirlwind") => {
                    let move1 = move1base * pullback;
                    let move2 = move2base;

                    init_gigas_fire(&mut next);

                    next.head.orientation.rotate_z(-PI / 8.0 * move1);
                    next.lower_torso.orientation.rotate_z(-PI / 6.0 * move1base);
                    next.shoulder_r.position += Vec3::new(-4.0, 4.0, 0.0) * move1base;
                    next.shoulder_r.orientation.rotate_z(PI / 5.0 * move1base);
                    next.control_l.orientation.rotate_x(-PI / 4.0 * move1);
                    next.control_r.orientation.rotate_z(PI / 4.0 * move1);
                    next.control_r.orientation.rotate_y(PI / 4.0 * move1);
                    next.control.position += Vec3::new(-5.0, -5.0, -5.0) * move1;
                    next.control.orientation.rotate_y(-PI / 2.3 * move1);
                    next.control.orientation.rotate_z(PI / 6.0 * move1 * move1);
                    next.leg_r.position += Vec3::new(1.5, -4.0, 0.0) * move1base;
                    next.leg_r.orientation.rotate_x(-PI / 8.0 * move1base);
                    next.leg_r.orientation.rotate_y(-PI / 8.0 * move1base);
                    next.leg_r.orientation.rotate_z(-PI / 8.0 * move1base);
                    next.foot_r.position += Vec3::new(3.0, -8.0, 0.0) * move1base;
                    next.foot_r.orientation.rotate_z(-PI / 4.0 * move1base);

                    next.head.orientation.rotate_z(-PI / 8.0 * move2);
                    next.torso.orientation.rotate_z(-2.0 * PI * move2);
                    next.lower_torso.orientation.rotate_z(PI / 6.0 * move2base);
                    next.shoulder_r.position += Vec3::new(4.0, -4.0, 0.0) * move2base;
                    next.shoulder_r.orientation.rotate_z(-PI / 5.0 * move2base);
                    next.shoulder_l.position += Vec3::new(4.0, 4.0, 0.0) * move2;
                    next.shoulder_l.orientation.rotate_z(-PI / 7.0 * move2);
                    next.control_l.orientation.rotate_x(PI / 2.0 * move2);
                    next.control_l.orientation.rotate_z(PI / 3.0 * move2);
                    next.control_r.orientation.rotate_z(PI / 2.4 * move2);
                    next.control_r.orientation.rotate_y(PI / 2.8 * move2);
                    next.control_r.orientation.rotate_x(PI / 2.8 * move2);
                    next.control.position += Vec3::new(4.0, 0.0, 0.0) * move2;
                    next.control.orientation.rotate_z(-PI / 1.2 * move2);
                    next.control.orientation.rotate_x(-PI / 5.0 * move2);
                    next.leg_r.position += Vec3::new(-1.5, 4.0, 0.0) * move2base.powi(2);
                    next.leg_r
                        .orientation
                        .rotate_x(PI / 8.0 * move2base.powi(2));
                    next.leg_r
                        .orientation
                        .rotate_y(PI / 8.0 * move2base.powi(2));
                    next.leg_r
                        .orientation
                        .rotate_z(PI / 8.0 * move2base.powi(2));
                    next.foot_r.position += Vec3::new(-3.0, 8.0, 0.0) * move2base.powi(2);
                    next.foot_r
                        .orientation
                        .rotate_z(PI / 4.0 * move2base.powi(2));
                    next.foot_r.orientation.rotate_z(-PI / 8.0 * move2.powi(2));
                    next.leg_l.position += Vec3::new(0.0, 2.5, 0.0) * move2.powi(2);
                    next.leg_l.orientation.rotate_x(PI / 6.0 * move2.powi(2));
                    next.foot_l.position += Vec3::new(0.0, 5.0, 0.0) * move2.powi(2);
                    if matches!(stage_section, Some(StageSection::Action)) {
                        next.leg_l.position.z += 3.0 * (PI * move2).sin();
                        next.foot_l.position.z += 3.0 * (PI * move2).sin();
                    }
                    next.foot_l.orientation.rotate_z(-PI / 8.0 * move2);
                },
                Some("common.abilities.custom.gigas_fire.vertical_strike") => {
                    let pullback = (1.0 - move3).powi(3);
                    let move1 = move1base * pullback;
                    let move2 = move2base * pullback;

                    init_gigas_fire(&mut next);

                    next.control.position += Vec3::new(5.0, 0.0, -20.0) * move1;
                    next.control.orientation.rotate_y(PI / 2.5 * move1);
                    next.control.orientation.rotate_x(-PI / 3.0 * move1);
                    next.control.orientation.rotate_z(-PI / 4.0 * move1);
                    next.shoulder_l.position += Vec3::new(5.0, 5.0, 0.0) * move1;
                    next.shoulder_l.orientation.rotate_z(-PI / 4.0 * move1);
                    next.torso.orientation.rotate_z(-PI / 3.0 * move1);
                    next.lower_torso.orientation.rotate_z(PI / 8.0 * move1);
                    next.leg_r.position += Vec3::new(2.0, -2.0, 0.0) * move1;
                    next.leg_r.orientation.rotate_y(-PI / 8.0 * move1);
                    next.leg_r.orientation.rotate_z(-PI / 8.0 * move1);
                    next.foot_r.position += Vec3::new(4.0, -4.0, 0.0) * move1;
                    next.foot_r.orientation.rotate_z(-PI / 7.0 * move1);
                    next.leg_l.position += Vec3::new(2.0, 3.0, 0.0) * move1;
                    next.leg_l.orientation.rotate_y(PI / 8.0 * move1);
                    next.leg_l.orientation.rotate_z(-PI / 8.0 * move1);
                    next.foot_l.position += Vec3::new(0.0, 4.0, 0.0) * move1;
                    next.foot_l.orientation.rotate_z(PI / 8.0 * move1);

                    next.control.position += Vec3::new(-8.0, -6.0, 40.0) * move2;
                    next.control.orientation.rotate_y(1.3 * PI * move2);
                    next.control
                        .orientation
                        .rotate_x(PI / 3.0 * (PI * move2).sin());
                    next.control.orientation.rotate_z(-PI / 1.1 * move2);
                    next.shoulder_l.position += Vec3::new(-5.0, -5.0, 0.0) * move2;
                    next.shoulder_l.orientation.rotate_z(PI / 4.0 * move2);
                    next.shoulder_l.orientation.rotate_x(PI / 2.5 * move2);
                    next.shoulder_r.position += Vec3::new(-5.0, 5.0, 0.0) * move2;
                    next.shoulder_r.orientation.rotate_z(PI / 4.0 * move2);
                    next.shoulder_r.orientation.rotate_x(PI / 2.5 * move2);
                    next.torso.orientation.rotate_z(PI / 2.0 * move2);
                    next.lower_torso.orientation.rotate_z(-PI / 8.0 * move2);
                    next.foot_r.position += Vec3::new(-2.0, 8.0, 0.0) * move2
                        + Vec3::new(0.0, 0.0, 5.0) * move2.powi(2);
                    next.leg_r.position += Vec3::new(-2.0, 4.0, 0.0) * move2
                        + Vec3::new(0.0, 0.0, 5.0) * move2.powi(2);
                    next.leg_r.orientation.rotate_z(PI / 4.0 * move2);
                    next.foot_l.position += Vec3::new(0.0, -8.0, 0.0) * move2;
                    next.foot_l.orientation.rotate_z(-PI / 5.0 * move2);
                    next.leg_l.position += Vec3::new(0.0, -4.0, 0.0) * move2;
                    next.leg_l.orientation.rotate_z(PI / 4.0 * move2);
                },
                // Mirrored version of the vertical strike
                Some("common.abilities.custom.gigas_fire.parry_punish") => {
                    let pullback = (1.0 - move3).powi(3);
                    let move1 = move1base * pullback;
                    let move2 = move2base * pullback;

                    init_gigas_fire(&mut next);

                    next.control.position += Vec3::new(0.0, 0.0, -20.0) * move1;
                    next.control.orientation.rotate_y(-PI / 2.5 * move1);
                    next.control.orientation.rotate_x(-PI / 3.0 * move1);
                    next.control.orientation.rotate_z(PI / 4.0 * move1);
                    next.shoulder_r.position += Vec3::new(-5.0, 5.0, 0.0) * move1;
                    next.shoulder_r.orientation.rotate_z(PI / 4.0 * move1);
                    next.torso.orientation.rotate_z(PI / 3.0 * move1);
                    next.lower_torso.orientation.rotate_z(-PI / 8.0 * move1);
                    next.leg_l.position += Vec3::new(-2.0, -2.0, 0.0) * move1;
                    next.leg_l.orientation.rotate_y(PI / 8.0 * move1);
                    next.leg_l.orientation.rotate_z(PI / 8.0 * move1);
                    next.foot_l.position += Vec3::new(-4.0, -4.0, 0.0) * move1;
                    next.foot_l.orientation.rotate_z(PI / 7.0 * move1);
                    next.leg_r.position += Vec3::new(-2.0, 3.0, 0.0) * move1;
                    next.leg_r.orientation.rotate_y(-PI / 8.0 * move1);
                    next.leg_r.orientation.rotate_z(PI / 8.0 * move1);
                    next.foot_r.position += Vec3::new(0.0, 4.0, 0.0) * move1;
                    next.foot_r.orientation.rotate_z(-PI / 8.0 * move1);

                    next.control.position += Vec3::new(-3.0, -6.0, 40.0) * move2;
                    next.control.orientation.rotate_y(-1.3 * PI * move2);
                    next.control
                        .orientation
                        .rotate_x(PI / 3.0 * (PI * move2).sin());
                    next.control.orientation.rotate_z(PI / 1.1 * move2);
                    next.shoulder_r.position += Vec3::new(5.0, -5.0, 0.0) * move2;
                    next.shoulder_r.orientation.rotate_z(-PI / 4.0 * move2);
                    next.shoulder_r.orientation.rotate_x(PI / 2.5 * move2);
                    next.shoulder_l.position += Vec3::new(5.0, 5.0, 0.0) * move2;
                    next.shoulder_l.orientation.rotate_z(-PI / 4.0 * move2);
                    next.shoulder_l.orientation.rotate_x(PI / 2.5 * move2);
                    next.torso.orientation.rotate_z(-PI / 2.0 * move2);
                    next.lower_torso.orientation.rotate_z(PI / 8.0 * move2);
                    next.foot_l.position +=
                        Vec3::new(2.0, 8.0, 0.0) * move2 + Vec3::new(0.0, 0.0, 5.0) * move2.powi(2);
                    next.leg_l.position +=
                        Vec3::new(2.0, 4.0, 0.0) * move2 + Vec3::new(0.0, 0.0, 5.0) * move2.powi(2);
                    next.leg_l.orientation.rotate_z(-PI / 4.0 * move2);
                    next.foot_r.position += Vec3::new(0.0, -8.0, 0.0) * move2;
                    next.foot_r.orientation.rotate_z(PI / 5.0 * move2);
                    next.leg_r.position += Vec3::new(0.0, -4.0, 0.0) * move2;
                    next.leg_r.orientation.rotate_z(-PI / 4.0 * move2);
                },
                Some("common.abilities.custom.gigas_fire.explosive_strike") => {
                    let (move1base, move2base, move3) = match stage_section {
                        Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                        Some(StageSection::Action) => (1.0, anim_time, 0.0),
                        Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                        _ => (0.0, 0.0, 0.0),
                    };

                    let (pre_move1base, move1base) = if move1base < 0.2 {
                        (5.0 * move1base, 0.0)
                    } else {
                        (1.0, 1.25 * (move1base - 0.2))
                    };

                    let pullback = 1.0 - move3;
                    let pre_move1 = pre_move1base * pullback;
                    let move2 = move2base * pullback;

                    init_gigas_fire(&mut next);

                    next.torso.orientation.rotate_z(PI / 8.0 * pre_move1base);
                    next.lower_torso
                        .orientation
                        .rotate_z(-PI / 16.0 * pre_move1);
                    next.shoulder_l
                        .orientation
                        .rotate_x(PI / 3.0 * pre_move1base);
                    next.shoulder_r
                        .orientation
                        .rotate_x(PI / 3.0 * pre_move1base);
                    next.shoulder_r.position += Vec3::new(-2.0, 8.0, 0.0) * pre_move1base;
                    next.shoulder_r
                        .orientation
                        .rotate_z(PI / 5.0 * pre_move1base);
                    next.control.position +=
                        Vec3::new(-15.0 * pre_move1base, 0.0, 25.0 * pre_move1base);
                    next.control.orientation.rotate_x(PI / 2.5 * pre_move1);
                    next.leg_l.position += Vec3::new(0.0, -2.5, 0.0) * pre_move1base;
                    next.leg_l.orientation.rotate_x(-PI / 8.0 * pre_move1base);
                    next.foot_l.position += Vec3::new(0.0, -5.0, 0.0) * pre_move1base;
                    next.foot_l.orientation.rotate_z(PI / 4.0 * pre_move1base);

                    next.torso.orientation.rotate_z(-PI / 8.0 * move1base);
                    next.control.orientation.rotate_z(2.0 * PI * move1base);
                    next.leg_l.position += Vec3::new(0.0, 2.5, 0.0) * move1base;
                    next.leg_l.orientation.rotate_x(PI / 8.0 * move1base);
                    next.foot_l.position += Vec3::new(0.0, 5.0, 0.0) * move1base;

                    next.torso.position += Vec3::new(0.0, -10.0, 0.0) * move2;
                    next.torso.orientation.rotate_z(-PI / 8.0 * move2);
                    next.torso.orientation.rotate_x(-PI / 8.0 * move2);
                    next.lower_torso.orientation.rotate_z(PI / 8.0 * move2);
                    next.lower_torso.orientation.rotate_x(PI / 8.0 * move2);
                    next.torso.position += Vec3::new(0.0, 0.0, 1.5) * move2;
                    next.shoulder_l.orientation.rotate_x(-PI / 3.0 * move2base);
                    next.shoulder_l.orientation.rotate_z(-PI / 4.0 * move2);
                    next.shoulder_r.position += Vec3::new(2.0, -8.0, 0.0) * move2base;
                    next.shoulder_r.orientation.rotate_z(-PI / 5.0 * move2base);
                    next.shoulder_r.orientation.rotate_x(-PI / 3.0 * move2base);
                    next.control.position +=
                        Vec3::new(15.0 * move2base, 0.0, -25.0 * move2base - 20.0 * move2);
                    next.control.orientation.rotate_x(-PI * move2);
                    next.leg_l.position += Vec3::new(0.0, 1.0, 0.0) * move2;
                    next.foot_l.position += Vec3::new(0.0, 2.5, 0.0) * move2;
                    next.foot_l.orientation.rotate_z(-PI / 4.0 * move2base);
                },
                _ => {
                    biped_large_alpha_sword(&mut next, s_a, move1, move2);
                },
            },
            Some(ToolKind::Hammer) => {
                biped_large_alpha_hammer(&mut next, s_a, move1, move2);
            },
            Some(ToolKind::Axe) => match ability_id {
                Some("common.abilities.custom.gigas_frost.cleave") => {
                    next.control_l.position = Vec3::new(-1.0, 2.0, 12.0 + move2 * -10.0);
                    next.control_r.position = Vec3::new(1.0, 2.0, -2.0);

                    next.control.position = Vec3::new(
                        4.0 + move1 * -12.0 + move2 * 28.0,
                        (s_a.grip.0 / 1.0) + move1 * -3.0 + move2 * -5.0,
                        (-s_a.grip.0 / 0.8) + move1 * 2.0 + move2 * -6.0,
                    );
                    next.head.orientation = Quaternion::rotation_x(move1 * -0.25)
                        * Quaternion::rotation_z(move1 * -0.2 + move2 * 0.6);
                    next.upper_torso.orientation =
                        Quaternion::rotation_z(move1 * 0.6 + move2 * -0.9);
                    next.lower_torso.orientation =
                        Quaternion::rotation_z(move1 * -0.6 + move2 * 0.9);

                    next.control_l.orientation = Quaternion::rotation_x(PI / 2.0 + move2 * 0.8)
                        * Quaternion::rotation_y(-0.0);
                    next.control_r.orientation =
                        Quaternion::rotation_x(PI / 2.0 + 0.2 + move2 * 0.8)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(0.0);

                    next.control.orientation = Quaternion::rotation_x(-0.3 + move2 * -1.5)
                        * Quaternion::rotation_y(move1 * -0.9 + move2 * 2.0)
                        * Quaternion::rotation_z(-0.3);
                },
                Some("common.abilities.custom.gigas_frost.wide_cleave") => {
                    next.control_l.position = Vec3::new(-1.0, 2.0, 12.0 + move2 * -10.0);
                    next.control_r.position = Vec3::new(1.0, 2.0, -2.0);

                    next.control.position = Vec3::new(
                        4.0 + move1 * -12.0 + move2 * 28.0,
                        (s_a.grip.0 / 1.0) + move1 * -3.0 + move2 * -5.0,
                        (-s_a.grip.0 / 0.8) + move1 * 2.0 + move2 * 8.0,
                    );
                    next.head.orientation = Quaternion::rotation_x(move1 * -0.25)
                        * Quaternion::rotation_z(move1 * -0.2 + move2 * 0.6);
                    next.upper_torso.orientation =
                        Quaternion::rotation_z(move1 * 0.6 + move2 * -0.9);
                    next.lower_torso.orientation =
                        Quaternion::rotation_z(move1 * -0.6 + move2 * 0.9);

                    next.control_l.orientation = Quaternion::rotation_x(PI / 2.0 + move2 * 0.8)
                        * Quaternion::rotation_y(-0.0);
                    next.control_r.orientation =
                        Quaternion::rotation_x(PI / 2.0 + 0.2 + move2 * 0.8)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(0.0);

                    next.control.orientation =
                        Quaternion::rotation_x(-1.0 + move1 * -0.5 + move2 * -0.3)
                            * Quaternion::rotation_y(-1.8 + move1 * -0.4 + move2 * 3.5)
                            * Quaternion::rotation_z(move1 * -1.0 + move2 * -1.5);
                },
                Some("common.abilities.custom.gigas_frost.bonk") => {
                    next.head.orientation = Quaternion::rotation_x(move1 * 0.8 + move2 * -1.2);
                    next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
                    next.jaw.orientation = Quaternion::rotation_x(move2 * -0.3);
                    next.control_l.position = Vec3::new(-0.5, 4.0, 1.0);
                    next.control_r.position = Vec3::new(-0.5, 4.0, 1.0);
                    next.control_l.orientation = Quaternion::rotation_x(PI / 2.0);
                    next.control_r.orientation = Quaternion::rotation_x(PI / 2.0);
                    next.weapon_l.position =
                        Vec3::new(-12.0 + (move1 * 20.0).min(10.0), -1.0, -15.0);
                    next.weapon_r.position =
                        Vec3::new(12.0 + (move1 * -20.0).max(-10.0), -1.0, -15.0);

                    next.weapon_l.orientation = Quaternion::rotation_x(-PI / 2.0 - 0.1)
                        * Quaternion::rotation_z(move1 * -1.0);
                    next.weapon_r.orientation = Quaternion::rotation_x(-PI / 2.0 - 0.1)
                        * Quaternion::rotation_z(move1 * 1.0);

                    next.shoulder_l.orientation =
                        Quaternion::rotation_x(-0.3 + move1 * 2.0 + move2 * -1.0);

                    next.shoulder_r.orientation =
                        Quaternion::rotation_x(-0.3 + move1 * 2.0 + move2 * -1.0);

                    next.control.orientation = Quaternion::rotation_x(move1 * 1.5 + move2 * -0.4);

                    let twist = move1 * 0.6 + move3 * -0.6;
                    next.upper_torso.position =
                        Vec3::new(0.0, s_a.upper_torso.0, s_a.upper_torso.1);
                    next.upper_torso.orientation =
                        Quaternion::rotation_x(move1 * 0.4 + move2 * -1.1)
                            * Quaternion::rotation_z(twist * -0.2 + move1 * -0.1 + move2 * 0.3);

                    next.lower_torso.orientation =
                        Quaternion::rotation_x(move1 * -0.4 + move2 * 1.1)
                            * Quaternion::rotation_z(twist);

                    next.foot_l.position = Vec3::new(
                        -s_a.foot.0,
                        s_a.foot.1 + move1 * -7.0 + move2 * 7.0,
                        s_a.foot.2,
                    );
                    next.foot_l.orientation = Quaternion::rotation_x(move1 * -0.8 + move2 * 0.8)
                        * Quaternion::rotation_z(move1 * 0.3 + move2 * -0.3);

                    next.foot_r.position = Vec3::new(
                        s_a.foot.0,
                        s_a.foot.1 + move1 * 5.0 + move2 * -5.0,
                        s_a.foot.2,
                    );
                    next.foot_r.orientation = Quaternion::rotation_y(move1 * -0.3 + move2 * 0.3)
                        * Quaternion::rotation_z(move1 * 0.4 + move2 * -0.4);
                    next.main.position = Vec3::new(-5.0 + (move1 * 20.0).min(10.0), 6.0, 4.0);
                    next.main.orientation = Quaternion::rotation_y(move1 * 0.4 + move2 * -1.2);
                },
                _ => {
                    biped_large_alpha_axe(&mut next, s_a, move1, move2);
                },
            },
            Some(ToolKind::Natural) => match ability_id {
                Some("common.abilities.custom.wendigomagic.singlestrike") => {
                    next.torso.position = Vec3::new(0.0, 0.0, move1 * -2.18);
                    next.upper_torso.orientation =
                        Quaternion::rotation_x(move1 * -0.5 + move2 * -0.4);
                    next.lower_torso.orientation =
                        Quaternion::rotation_x(move1 * 0.5 + move2 * 0.4);

                    next.control_l.position =
                        Vec3::new(-9.0 + move2 * 6.0, 19.0 + move1 * 6.0, -13.0 + move1 * 10.5);
                    next.control_r.position =
                        Vec3::new(9.0 + move2 * -6.0, 19.0 + move1 * 6.0, -13.0 + move1 * 14.5);

                    next.control_l.orientation = Quaternion::rotation_x(PI / 3.0 + move1 * 0.5)
                        * Quaternion::rotation_y(-0.15)
                        * Quaternion::rotation_z(move1 * 0.5 + move2 * -0.6);
                    next.control_r.orientation = Quaternion::rotation_x(PI / 3.0 + move1 * 0.5)
                        * Quaternion::rotation_y(0.15)
                        * Quaternion::rotation_z(move1 * -0.5 + move2 * 0.6);
                    next.head.orientation = Quaternion::rotation_x(move1 * 0.3);
                },
                Some("common.abilities.custom.minotaur.cripplingstrike") => {
                    next.control_l.position = Vec3::new(0.0, 4.0, 5.0);
                    next.control_r.position = Vec3::new(0.0, 4.0, 5.0);
                    next.weapon_l.position = Vec3::new(
                        -12.0 + move1 * -9.0 + move2 * 16.0,
                        -6.0 + move2 * 8.0,
                        -18.0 + move1 * 8.0 + move2 * -4.0,
                    );
                    next.weapon_r.position = Vec3::new(
                        12.0 + move1 * 9.0 + move2 * -16.0,
                        -6.0 + move2 * 8.0,
                        -18.0 + move1 * 8.0 + move2 * -8.0,
                    );

                    next.weapon_l.orientation = Quaternion::rotation_x(-1.67)
                        * Quaternion::rotation_y(move1 * -0.3 + move2 * 1.0)
                        * Quaternion::rotation_z(move1 * 0.8 + move2 * -1.8);
                    next.weapon_r.orientation = Quaternion::rotation_x(-1.67)
                        * Quaternion::rotation_y(move1 * 0.3 + move2 * -0.6)
                        * Quaternion::rotation_z(move1 * -0.8 + move2 * 1.8);

                    next.control_l.orientation = Quaternion::rotation_x(PI / 2.0 + move2 * 1.0);
                    next.control_r.orientation = Quaternion::rotation_x(PI / 2.0 + move2 * 1.0);

                    next.shoulder_l.orientation = Quaternion::rotation_x(-0.3)
                        * Quaternion::rotation_y(move1 * 0.7 + move2 * -0.7);

                    next.shoulder_r.orientation = Quaternion::rotation_x(-0.3)
                        * Quaternion::rotation_y(move1 * -0.7 + move2 * 0.7);
                    next.second.scale = Vec3::one() * 1.0;
                    next.head.orientation = Quaternion::rotation_x(move1 * -0.6 + move2 * 0.4)
                },
                Some("common.abilities.custom.yeti.strike") => {
                    next.control_l.position = Vec3::new(-1.0, 2.0, 12.0 + move2 * -10.0);
                    next.control_r.position = Vec3::new(1.0, 2.0, -2.0);

                    next.control.position = Vec3::new(
                        4.0 + move1 * -12.0 + move2 * 20.0,
                        (s_a.grip.0 / 1.0) + move1 * -3.0 + move2 * 5.0,
                        (-s_a.grip.0 / 0.8) + move1 * -2.0 + move2 * 8.0,
                    );
                    next.head.orientation = Quaternion::rotation_x(move1 * -0.25)
                        * Quaternion::rotation_z(move1 * -0.2 + move2 * 0.6);
                    next.upper_torso.orientation =
                        Quaternion::rotation_z(move1 * 0.2 + move2 * -0.4);
                    next.lower_torso.orientation =
                        Quaternion::rotation_z(move1 * -0.2 + move2 * 0.2);

                    next.control_l.orientation = Quaternion::rotation_x(PI / 2.0 + move2 * 0.8)
                        * Quaternion::rotation_y(-0.0);
                    next.control_r.orientation =
                        Quaternion::rotation_x(PI / 2.0 + 0.2 + move2 * 0.8)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(0.0);

                    next.control.orientation =
                        Quaternion::rotation_x(-1.0 + move1 * -0.5 + move2 * -0.3)
                            * Quaternion::rotation_y(-1.8 + move1 * -0.8 + move2 * 3.0)
                            * Quaternion::rotation_z(move1 * -0.8 + move2 * -0.8);
                },
                Some("common.abilities.custom.harvester.scythe") => {
                    next.head.orientation = Quaternion::rotation_x(move1 * -0.25 + move2 * 0.25)
                        * Quaternion::rotation_z(move1 * -0.3 + move2 * 0.4);

                    next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1 + move2 * -0.5);
                    next.jaw.orientation = Quaternion::rotation_x(move2 * -0.15);

                    next.upper_torso.orientation =
                        Quaternion::rotation_x(move1 * 0.2 + move2 * -0.2)
                            * Quaternion::rotation_z(move1 * -1.0 + move2 * 1.0);

                    next.shoulder_l.position = Vec3::new(
                        -s_a.shoulder.0,
                        s_a.shoulder.1,
                        s_a.shoulder.2 - foothorir * 1.0,
                    );
                    next.shoulder_l.orientation =
                        Quaternion::rotation_x(-0.4 + move1 * 1.0 + move2 * -1.0)
                            * Quaternion::rotation_y(move1 * -0.2);
                    next.shoulder_r.orientation =
                        Quaternion::rotation_y(0.4 + move1 * -0.8 + move2 * 0.8)
                            * Quaternion::rotation_x(0.4 + move1 * -0.4 + move2 * 0.8);

                    if speed == 0.0 {
                        next.leg_l.orientation = Quaternion::rotation_x(move1 * 0.4 + move2 * -0.4);

                        next.foot_l.position = Vec3::new(
                            -s_a.foot.0,
                            s_a.foot.1,
                            s_a.foot.2 + move1 * 2.0 + move2 * -2.0,
                        );
                        next.foot_l.orientation =
                            Quaternion::rotation_x(move1 * -0.6 + move2 * 0.6);
                    }

                    next.control_l.position = Vec3::new(1.0, 2.0, 8.0);
                    next.control_r.position = Vec3::new(1.0, 1.0, -2.0);

                    next.control.position = Vec3::new(
                        -7.0 + move1 * 26.0 - move2 * 32.0,
                        0.0 + s_a.grip.0 / 1.0 - move1 * 4.0,
                        -s_a.grip.0 / 0.8,
                    );

                    next.control_l.orientation = Quaternion::rotation_x(PI / 2.0)
                        * Quaternion::rotation_y(-0.0)
                        * Quaternion::rotation_z(PI);
                    next.control_r.orientation = Quaternion::rotation_x(PI / 2.0 + 0.2)
                        * Quaternion::rotation_y(-1.0 + move1 * 1.0)
                        * Quaternion::rotation_z(0.0);

                    next.control.orientation = Quaternion::rotation_x(-1.4 + move1 * -0.4)
                        * Quaternion::rotation_y(-2.8 + move1 * 3.0 + move2 * -3.0)
                        * Quaternion::rotation_z(move1 * -1.5);
                },
                Some(
                    "common.abilities.custom.husk_brute.singlestrike"
                    | "common.abilities.vampire.strigoi.singlestrike",
                ) => {
                    next.shoulder_l.position =
                        Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
                    next.shoulder_r.position =
                        Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
                    next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2);
                    next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2);

                    if mirror > 0.0 {
                        next.shoulder_l.orientation = Quaternion::rotation_x(move1 * 1.0)
                            * Quaternion::rotation_y(move1 * 1.0 + move2 * -2.0);

                        next.shoulder_r.orientation = Quaternion::rotation_x(move1 * -0.6);

                        next.upper_torso.orientation =
                            Quaternion::rotation_z(move1 * 0.4 + move2 * -1.1);
                        next.lower_torso.orientation =
                            Quaternion::rotation_z(move1 * -0.2 + move2 * 0.6);

                        next.hand_l.orientation = Quaternion::rotation_x(move1 * 1.2)
                            * Quaternion::rotation_y(move1 * 1.0 + move2 * -2.0);

                        next.hand_r.orientation = Quaternion::rotation_z(move1 * 1.0)
                            * Quaternion::rotation_y(move1 * 0.8 + move2 * -1.2);
                    } else {
                        next.shoulder_r.orientation = Quaternion::rotation_x(move1 * 1.0)
                            * Quaternion::rotation_y(move1 * -1.0 + move2 * 2.0);

                        next.shoulder_l.orientation = Quaternion::rotation_x(move1 * 0.6);

                        next.upper_torso.orientation =
                            Quaternion::rotation_z(move1 * -0.4 + move2 * 1.1);
                        next.lower_torso.orientation =
                            Quaternion::rotation_z(move1 * 0.2 + move2 * -0.6);

                        next.hand_r.orientation = Quaternion::rotation_x(move1 * 1.2)
                            * Quaternion::rotation_y(move1 * -1.0 + move2 * 2.0);

                        next.hand_l.orientation = Quaternion::rotation_z(move1 * 1.0)
                            * Quaternion::rotation_y(move1 * -0.8 + move2 * 1.2);
                    }
                },
                Some("common.abilities.custom.beastclaws.basic") => {
                    next.shoulder_l.position =
                        Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
                    next.shoulder_r.position =
                        Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
                    next.hand_l.position =
                        Vec3::new(-s_a.hand.0 - 3.0, s_a.hand.1 + 4.5, s_a.hand.2 + 0.0);
                    next.hand_r.position =
                        Vec3::new(s_a.hand.0 + 3.0, s_a.hand.1 + 4.5, s_a.hand.2 + 0.0);

                    if mirror > 0.0 {
                        next.shoulder_l.orientation = Quaternion::rotation_x(move1 * 1.0)
                            * Quaternion::rotation_y(move1 * 0.5 + move2 * -1.0);

                        next.shoulder_r.orientation = Quaternion::rotation_x(move1 * 0.6);

                        next.upper_torso.orientation =
                            Quaternion::rotation_z(move1 * 0.4 + move2 * -1.1);
                        next.lower_torso.orientation =
                            Quaternion::rotation_z(move1 * -0.2 + move2 * 0.6);

                        next.hand_l.orientation = Quaternion::rotation_x(move1 * 1.2)
                            * Quaternion::rotation_y(move1 * 0.2 + move2 * -0.5)
                            * Quaternion::rotation_z(move1 * 2.0);

                        next.hand_r.orientation = Quaternion::rotation_z(move1 * -2.0)
                            * Quaternion::rotation_y(move1 * 0.8 + move2 * -0.6);
                    } else {
                        next.shoulder_r.orientation = Quaternion::rotation_x(move1 * 1.0)
                            * Quaternion::rotation_y(move1 * -0.5 + move2 * 1.0);

                        next.shoulder_l.orientation = Quaternion::rotation_x(move1 * 0.6);

                        next.upper_torso.orientation =
                            Quaternion::rotation_z(move1 * -0.4 + move2 * 1.1);
                        next.lower_torso.orientation =
                            Quaternion::rotation_z(move1 * 0.2 + move2 * -0.6);

                        next.hand_r.orientation = Quaternion::rotation_x(move1 * 1.2)
                            * Quaternion::rotation_y(move1 * -0.3 + move2 * 0.5)
                            * Quaternion::rotation_z(move1 * -2.0);

                        next.hand_l.orientation = Quaternion::rotation_z(move1 * 1.0)
                            * Quaternion::rotation_y(move1 * -0.8 + move2 * 0.6);
                    }
                },
                Some("common.abilities.custom.tursus.tursus_claws") => {
                    next.shoulder_l.position =
                        Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
                    next.shoulder_r.position =
                        Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
                    next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2 + 4.0);
                    next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2 + 4.0);

                    if mirror > 0.0 {
                        next.shoulder_l.orientation = Quaternion::rotation_x(move1 * 1.0)
                            * Quaternion::rotation_y(move1 * 1.0 + move2 * -2.0);

                        next.shoulder_r.orientation = Quaternion::rotation_x(move1 * 0.6);

                        next.upper_torso.orientation =
                            Quaternion::rotation_z(move1 * 0.4 + move2 * -1.1);
                        next.lower_torso.orientation =
                            Quaternion::rotation_z(move1 * -0.2 + move2 * 0.6);

                        next.hand_l.orientation = Quaternion::rotation_x(move1 * 1.2)
                            * Quaternion::rotation_y(move1 * 1.0 + move2 * -2.0);

                        next.hand_r.orientation = Quaternion::rotation_z(move1 * -1.0)
                            * Quaternion::rotation_y(move1 * 0.8 + move2 * -1.2);
                    } else {
                        next.shoulder_r.orientation = Quaternion::rotation_x(move1 * 1.0)
                            * Quaternion::rotation_y(move1 * -1.0 + move2 * 2.0);

                        next.shoulder_l.orientation = Quaternion::rotation_x(move1 * 0.6);

                        next.upper_torso.orientation =
                            Quaternion::rotation_z(move1 * -0.4 + move2 * 1.1);
                        next.lower_torso.orientation =
                            Quaternion::rotation_z(move1 * 0.2 + move2 * -0.6);

                        next.hand_r.orientation = Quaternion::rotation_x(move1 * 1.2)
                            * Quaternion::rotation_y(move1 * -1.0 + move2 * 2.0);

                        next.hand_l.orientation = Quaternion::rotation_z(move1 * 1.0)
                            * Quaternion::rotation_y(move1 * -0.8 + move2 * 1.2);
                    }
                },
                _ => {},
            },
            Some(ToolKind::Spear) => match ability_id {
                Some("common.abilities.custom.tidalwarrior.pincer") => {
                    next.head.orientation = Quaternion::rotation_z(move1 * -0.75);
                    next.upper_torso.orientation =
                        Quaternion::rotation_x(move1 * 0.2 + move2 * 0.4)
                            * Quaternion::rotation_z(move1 * 1.0 + move2 * -1.3);
                    next.lower_torso.orientation =
                        Quaternion::rotation_x(move1 * 0.2 + move2 * -0.9)
                            * Quaternion::rotation_y(move1 * 0.1 + move2 * -0.2)
                            * Quaternion::rotation_z(move1 * -1.0 + move2 * 1.2);

                    next.hand_r.position = Vec3::new(
                        14.0 + move1 * 2.0 + move2 * 3.0,
                        6.0 + move2 * 2.0,
                        -6.0 - move2 * 3.0,
                    );

                    next.hand_r.orientation = Quaternion::rotation_y(move2 * -0.5);
                    next.hand_l.position = Vec3::new(-14.0, 7.0, -9.0);

                    next.hand_l.orientation =
                        Quaternion::rotation_x(0.1 * move2) * Quaternion::rotation_z(-0.35);
                    next.torso.position = Vec3::new(0.0, move2 * -10.35, move2 * -1.0);

                    next.main.position = Vec3::new(-14.0, 0.0, -14.0);
                    next.main.orientation = Quaternion::rotation_x(PI / -2.0 - 0.2 * move2)
                        * Quaternion::rotation_y(0.4 + 0.4 * move2);
                },
                _ => {},
            },
            _ => {},
        }

        next
    }
}
