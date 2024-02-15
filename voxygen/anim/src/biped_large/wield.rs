use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::comp::item::tool::{AbilitySpec, ToolKind};
use core::{f32::consts::PI, ops::Mul};

pub struct WieldAnimation;

impl Animation for WieldAnimation {
    type Dependency<'a> = (
        (Option<ToolKind>, Option<&'a AbilitySpec>),
        (Option<ToolKind>, Option<&'a AbilitySpec>),
        Vec3<f32>,
        f32,
        f32,
    );
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_wield\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_wield")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        ((active_tool_kind, active_tool_spec), _second_tool, velocity, global_time, acc_vel): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();

        let lab: f32 = 0.65 * s_a.tempo; //.65
        let torso = (anim_time * lab + 1.5 * PI).sin();
        let speednorm = (speed / 12.0).powf(0.4);
        let foothoril = (acc_vel * lab + PI * 1.45).sin() * speednorm;
        let foothorir = (acc_vel * lab + PI * (0.45)).sin() * speednorm;
        let footrotl = ((1.0 / (0.5 + (0.5) * ((acc_vel * lab + PI * 1.4).sin()).powi(2))).sqrt())
            * ((acc_vel * lab + PI * 1.4).sin());

        let footrotr = ((1.0 / (0.5 + (0.5) * ((acc_vel * lab + PI * 0.4).sin()).powi(2))).sqrt())
            * ((acc_vel * lab + PI * 0.4).sin());
        let slower = (anim_time * 1.0 + PI).sin();
        let slow = (anim_time * 3.5 + PI).sin();

        let tailmove = Vec2::new(
            (global_time / 2.0 + anim_time / 2.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.25,
            (global_time / 2.0 + anim_time / 2.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.125,
        );

        let look = Vec2::new(
            (global_time / 2.0 + anim_time / 8.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.5,
            (global_time / 2.0 + anim_time / 8.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.25,
        );

        let breathe = if s_a.beast {
            // Controls for the beast breathing
            let intensity = 0.04;
            let lenght = 1.5;
            let chop = 0.2;
            let chop_freq = 60.0;
            intensity * (lenght * anim_time).sin()
                + 0.05 * chop * (anim_time * chop_freq).sin() * (anim_time * lenght).cos()
        } else {
            0.0
        };

        let footvertl = (anim_time * 16.0 * lab).sin();
        let footvertr = (anim_time * 16.0 * lab + PI).sin();
        let handhoril = (anim_time * 16.0 * lab + PI * 1.4).sin();
        let handhorir = (anim_time * 16.0 * lab + PI * 0.4).sin();

        let short = (acc_vel * lab).sin() * speednorm;

        let shortalt = (anim_time * lab * 16.0 + PI / 2.0).sin();
        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation = Quaternion::rotation_x(0.0);

        if s_a.beast {
            next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
        } else {
            next.jaw.position = Vec3::new(0.0, s_a.jaw.0 - slower * 0.12, s_a.jaw.1 + slow * 0.2);

            next.hold.scale = Vec3::one() * 0.0;

            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_x(0.0);

            next.hand_l.position = Vec3::new(s_a.grip.1, 0.0, s_a.grip.0);
            next.hand_r.position = Vec3::new(-s_a.grip.1, 0.0, s_a.grip.0);

            next.hand_l.orientation = Quaternion::rotation_x(0.0);
            next.hand_r.orientation = Quaternion::rotation_x(0.0);

            if speed > 0.5 {
                next.shoulder_l.position = Vec3::new(
                    -s_a.shoulder.0,
                    s_a.shoulder.1,
                    s_a.shoulder.2 - foothorir * 1.0,
                );
                next.shoulder_l.orientation =
                    Quaternion::rotation_x(0.6 * speednorm + (footrotr * -0.2) * speednorm);

                next.shoulder_r.position = Vec3::new(
                    s_a.shoulder.0,
                    s_a.shoulder.1,
                    s_a.shoulder.2 - foothoril * 1.0,
                );
                next.shoulder_r.orientation =
                    Quaternion::rotation_x(0.6 * speednorm + (footrotl * -0.2) * speednorm);
            } else {
                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1) * 1.02;
                next.head.orientation =
                    Quaternion::rotation_z(look.x * 0.6) * Quaternion::rotation_x(look.y * 0.6);

                next.upper_torso.position =
                    Vec3::new(0.0, s_a.upper_torso.0, s_a.upper_torso.1 + torso * -0.5);
                next.upper_torso.orientation =
                    Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);

                next.lower_torso.position =
                    Vec3::new(0.0, s_a.lower_torso.0, s_a.lower_torso.1 + torso * 0.5);
                next.lower_torso.orientation =
                    Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
                next.lower_torso.scale = Vec3::one() * 1.02;

                next.jaw.position =
                    Vec3::new(0.0, s_a.jaw.0 - slower * 0.12, s_a.jaw.1 + slow * 0.2);
                next.jaw.orientation = Quaternion::rotation_x(-0.1 + breathe * 2.0);

                next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
                next.tail.orientation = Quaternion::rotation_z(0.0 + slow * 0.2 + tailmove.x)
                    * Quaternion::rotation_x(0.0);

                next.shoulder_l.position =
                    Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
                next.shoulder_l.orientation =
                    Quaternion::rotation_y(0.0) * Quaternion::rotation_x(0.3);

                next.shoulder_r.position =
                    Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
                next.shoulder_r.orientation =
                    Quaternion::rotation_y(0.0) * Quaternion::rotation_x(0.3);
            }

            match active_tool_kind {
                Some(ToolKind::Sword) => {
                    next.control_l.position = Vec3::new(-1.0, 1.0, 1.0);
                    next.control_r.position = Vec3::new(0.0, 2.0, -3.0);

                    next.control.position = Vec3::new(
                        -3.0,
                        5.0 + s_a.grip.0 / 1.2,
                        -4.0 + -s_a.grip.0 / 2.0 + short * -1.5,
                    );
                    next.second.scale = Vec3::one() * 0.0;

                    next.control_l.orientation =
                        Quaternion::rotation_x(PI / 2.0) * Quaternion::rotation_y(-0.2);
                    next.control_r.orientation =
                        Quaternion::rotation_x(PI / 2.2) * Quaternion::rotation_y(0.2);

                    next.control.orientation =
                        Quaternion::rotation_x(-0.2 + short * 0.2) * Quaternion::rotation_y(-0.1);
                },
                Some(ToolKind::Bow) => {
                    next.control_l.position = Vec3::new(-1.0, -2.0, -3.0);
                    next.control_r.position = Vec3::new(0.0, 4.0, 1.0);

                    next.control.position = Vec3::new(
                        -1.0,
                        6.0 + s_a.grip.0 / 1.2,
                        -5.0 + -s_a.grip.0 / 2.0 + short * -1.5,
                    );

                    next.control_l.orientation =
                        Quaternion::rotation_x(PI / 2.0) * Quaternion::rotation_y(-0.2);
                    next.control_r.orientation = Quaternion::rotation_x(PI / 2.2)
                        * Quaternion::rotation_y(0.2)
                        * Quaternion::rotation_z(0.0);

                    next.control.orientation = Quaternion::rotation_x(-0.2 + short * 0.2)
                        * Quaternion::rotation_y(1.0)
                        * Quaternion::rotation_z(-0.3);
                },
                Some(ToolKind::Hammer) | Some(ToolKind::Axe) => {
                    next.control_l.position = Vec3::new(-1.0, 2.0, 12.0);
                    next.control_r.position = Vec3::new(1.0, 2.0, -2.0);

                    next.control.position = Vec3::new(
                        4.0,
                        0.0 + s_a.grip.0 / 1.0,
                        -s_a.grip.0 / 0.8 + short * -1.5,
                    );

                    next.control_l.orientation =
                        Quaternion::rotation_x(PI / 2.0) * Quaternion::rotation_y(-0.0);
                    next.control_r.orientation = Quaternion::rotation_x(PI / 2.0 + 0.2)
                        * Quaternion::rotation_y(0.0)
                        * Quaternion::rotation_z(0.0);

                    next.control.orientation =
                        Quaternion::rotation_x(-1.0 + short * 0.2) * Quaternion::rotation_y(-1.8);
                },
                Some(ToolKind::Staff) | Some(ToolKind::Sceptre) => {
                    next.control_l.position = Vec3::new(-1.0, 3.0, 12.0);
                    next.control_r.position = Vec3::new(1.0, 2.0, 2.0);

                    next.control.position = Vec3::new(
                        -3.0,
                        3.0 + s_a.grip.0 / 1.2,
                        -11.0 + -s_a.grip.0 / 2.0 + short * -1.5,
                    );

                    next.control_l.orientation =
                        Quaternion::rotation_x(PI / 2.0) * Quaternion::rotation_y(-0.5);
                    next.control_r.orientation = Quaternion::rotation_x(PI / 2.5)
                        * Quaternion::rotation_y(0.5)
                        * Quaternion::rotation_z(0.0);

                    next.control.orientation =
                        Quaternion::rotation_x(-0.2 + short * 0.2) * Quaternion::rotation_y(-0.1);
                },
                Some(ToolKind::Natural) => {
                    if let Some(AbilitySpec::Custom(spec)) = active_tool_spec {
                        match spec.as_str() {
                            "Wendigo Magic" => {
                                next.control_l.position = Vec3::new(-9.0, 19.0, -13.0);
                                next.control_r.position = Vec3::new(9.0, 19.0, -13.0);

                                next.control_l.orientation = Quaternion::rotation_x(PI / 3.0)
                                    * Quaternion::rotation_y(-0.15);
                                next.control_r.orientation =
                                    Quaternion::rotation_x(PI / 3.0) * Quaternion::rotation_y(0.15);
                            },
                            "Tidal Warrior" => {
                                next.hand_l.position = Vec3::new(-14.0, 2.0, -4.0);
                                next.hand_r.position = Vec3::new(14.0, 2.0, -4.0);

                                next.hand_l.orientation = Quaternion::rotation_x(PI / 3.0)
                                    * Quaternion::rotation_z(-0.35);
                                next.hand_r.orientation =
                                    Quaternion::rotation_x(PI / 3.0) * Quaternion::rotation_z(0.35);
                            },
                            "Beast Claws" | "Tursus Claws" => {
                                next.shoulder_l.position =
                                    Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);

                                next.shoulder_r.position =
                                    Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);

                                next.hand_l.position =
                                    Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2 + torso * 0.6);

                                next.hand_r.position =
                                    Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2 + torso * 0.6);

                                if speed < 0.5 {
                                    next.head.position =
                                        Vec3::new(0.0, s_a.head.0, s_a.head.1 + torso * 0.2) * 1.02;
                                    next.head.orientation = Quaternion::rotation_z(look.x * 0.6)
                                        * Quaternion::rotation_x(look.y * 0.6);

                                    next.upper_torso.position = Vec3::new(
                                        0.0,
                                        s_a.upper_torso.0,
                                        s_a.upper_torso.1 + torso * 0.5,
                                    );

                                    next.lower_torso.position = Vec3::new(
                                        0.0,
                                        s_a.lower_torso.0,
                                        s_a.lower_torso.1 + torso * 0.15,
                                    );

                                    next.jaw.orientation = Quaternion::rotation_x(-0.1);

                                    next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
                                    next.tail.orientation =
                                        Quaternion::rotation_z(slow * 0.2 + tailmove.x);

                                    next.second.orientation = Quaternion::rotation_x(PI);

                                    next.main.position = Vec3::new(0.0, 0.0, 0.0);
                                    next.main.orientation = Quaternion::rotation_y(0.0);

                                    next.shoulder_l.position =
                                        Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);

                                    next.hand_l.position = Vec3::new(
                                        -s_a.hand.0,
                                        s_a.hand.1,
                                        s_a.hand.2 + torso * 0.6,
                                    );

                                    next.hand_r.position =
                                        Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2 + torso * 0.6);

                                    next.leg_l.position =
                                        Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2 + torso * 0.2);
                                    next.leg_l.orientation =
                                        Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);

                                    next.leg_r.position =
                                        Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2 + torso * 0.2);
                                    next.leg_r.orientation =
                                        Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
                                } else {
                                    next.head.position =
                                        Vec3::new(0.0, s_a.head.0, s_a.head.1) * 1.02;
                                    next.head.orientation = Quaternion::rotation_z(short * -0.18)
                                        * Quaternion::rotation_x(-0.05);
                                    next.head.scale = Vec3::one() * 1.02;

                                    next.upper_torso.position = Vec3::new(
                                        0.0,
                                        s_a.upper_torso.0,
                                        s_a.upper_torso.1 + shortalt * -1.5,
                                    );
                                    next.upper_torso.orientation =
                                        Quaternion::rotation_z(short * 0.18);

                                    next.lower_torso.position =
                                        Vec3::new(0.0, s_a.lower_torso.0, s_a.lower_torso.1);
                                    next.lower_torso.orientation =
                                        Quaternion::rotation_z(short * 0.15)
                                            * Quaternion::rotation_x(0.14);
                                    next.lower_torso.scale = Vec3::one() * 1.02;

                                    next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
                                    next.jaw.orientation = Quaternion::rotation_x(0.0);
                                    next.jaw.scale = Vec3::one() * 1.02;

                                    next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
                                    next.tail.orientation = Quaternion::rotation_x(shortalt * 0.3);

                                    next.second.position = Vec3::new(0.0, 0.0, 0.0);
                                    next.second.orientation = Quaternion::rotation_x(PI)
                                        * Quaternion::rotation_y(0.0)
                                        * Quaternion::rotation_z(0.0);
                                    next.second.scale = Vec3::one() * 0.0;

                                    next.control.position = Vec3::new(0.0, 0.0, 0.0);
                                    next.control.orientation = Quaternion::rotation_z(0.0);

                                    next.main.position = Vec3::new(0.0, 0.0, 0.0);
                                    next.main.orientation = Quaternion::rotation_y(0.0);

                                    next.shoulder_l.position = Vec3::new(
                                        -s_a.shoulder.0,
                                        s_a.shoulder.1 + foothoril * -3.0,
                                        s_a.shoulder.2,
                                    );
                                    next.shoulder_l.orientation =
                                        Quaternion::rotation_x(footrotl * -0.36)
                                            * Quaternion::rotation_y(0.1)
                                            * Quaternion::rotation_z(footrotl * 0.3);

                                    next.shoulder_r.position = Vec3::new(
                                        s_a.shoulder.0,
                                        s_a.shoulder.1 + foothorir * -3.0,
                                        s_a.shoulder.2,
                                    );
                                    next.shoulder_r.orientation =
                                        Quaternion::rotation_x(footrotr * -0.36)
                                            * Quaternion::rotation_y(-0.1)
                                            * Quaternion::rotation_z(footrotr * -0.3);

                                    next.hand_l.position = Vec3::new(
                                        -1.0 + -s_a.hand.0,
                                        s_a.hand.1 + foothoril * -4.0,
                                        s_a.hand.2 + foothoril * 1.0,
                                    );
                                    next.hand_l.orientation =
                                        Quaternion::rotation_x(0.15 + (handhoril * -1.2).max(-0.3))
                                            * Quaternion::rotation_y(handhoril * -0.1);

                                    next.hand_r.position = Vec3::new(
                                        1.0 + s_a.hand.0,
                                        s_a.hand.1 + foothorir * -4.0,
                                        s_a.hand.2 + foothorir * 1.0,
                                    );
                                    next.hand_r.orientation =
                                        Quaternion::rotation_x(0.15 + (handhorir * -1.2).max(-0.3))
                                            * Quaternion::rotation_y(handhorir * 0.1);
                                    next.hand_r.scale = Vec3::one() * 1.04;

                                    next.leg_l.position =
                                        Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2) * 0.98;
                                    next.leg_l.orientation = Quaternion::rotation_z(short * 0.18)
                                        * Quaternion::rotation_x(foothoril * 0.3);
                                    next.leg_r.position =
                                        Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2) * 0.98;

                                    next.leg_r.orientation = Quaternion::rotation_z(short * 0.18)
                                        * Quaternion::rotation_x(foothorir * 0.3);

                                    next.foot_l.position = Vec3::new(
                                        -s_a.foot.0,
                                        s_a.foot.1 + foothoril * 8.5,
                                        s_a.foot.2 + ((footvertl * 6.5).max(0.0)),
                                    );
                                    next.foot_l.orientation =
                                        Quaternion::rotation_x(-0.5 + footrotl * 0.85);

                                    next.foot_r.position = Vec3::new(
                                        s_a.foot.0,
                                        s_a.foot.1 + foothorir * 8.5,
                                        s_a.foot.2 + ((footvertr * 6.5).max(0.0)),
                                    );
                                    next.foot_r.orientation =
                                        Quaternion::rotation_x(-0.5 + footrotr * 0.85);

                                    next.torso.orientation = Quaternion::rotation_x(-0.25);
                                }
                            },
                            "Minotaur" => {
                                next.control_l.position = Vec3::new(0.0, 4.0, 5.0);
                                next.control_r.position = Vec3::new(0.0, 4.0, 5.0);
                                next.weapon_l.position = Vec3::new(-12.0, -6.0, -18.0);
                                next.weapon_r.position = Vec3::new(12.0, -6.0, -18.0);

                                next.weapon_l.orientation = Quaternion::rotation_x(-PI / 2.0 - 0.1);
                                next.weapon_r.orientation = Quaternion::rotation_x(-PI / 2.0 - 0.1);

                                next.control_l.orientation = Quaternion::rotation_x(PI / 2.0);
                                next.control_r.orientation = Quaternion::rotation_x(PI / 2.0);

                                next.control.orientation =
                                    Quaternion::rotation_x(0.0) * Quaternion::rotation_y(0.0);
                                next.shoulder_l.orientation = Quaternion::rotation_x(-0.3);

                                next.shoulder_r.orientation = Quaternion::rotation_x(-0.3);
                                next.second.scale = Vec3::one() * 1.0;
                            },
                            "Yeti" => {
                                next.second.scale = Vec3::one() * 0.0;
                                next.control_l.position = Vec3::new(-0.5, 4.0, 1.0);
                                next.control_r.position = Vec3::new(-0.5, 4.0, 1.0);
                                next.weapon_l.position = Vec3::new(-12.0, -1.0, -15.0);
                                next.weapon_r.position = Vec3::new(12.0, -1.0, -15.0);

                                next.weapon_l.orientation = Quaternion::rotation_x(-PI / 2.0 - 0.1);
                                next.weapon_r.orientation = Quaternion::rotation_x(-PI / 2.0 - 0.1);

                                next.control_l.orientation = Quaternion::rotation_x(PI / 2.0);
                                next.control_r.orientation = Quaternion::rotation_x(PI / 2.0);

                                next.control.orientation =
                                    Quaternion::rotation_x(0.0) * Quaternion::rotation_y(0.0);
                                next.shoulder_l.orientation = Quaternion::rotation_x(-0.3);

                                next.shoulder_r.orientation = Quaternion::rotation_x(-0.3);
                            },
                            "Harvester" => {
                                next.control_l.position = Vec3::new(1.0, 2.0, 8.0);
                                next.control_r.position = Vec3::new(1.0, 1.0, -2.0);

                                next.control.position = Vec3::new(
                                    -7.0,
                                    0.0 + s_a.grip.0 / 1.0,
                                    -s_a.grip.0 / 0.8 + short * -1.5,
                                );

                                next.control_l.orientation =
                                    Quaternion::rotation_x(PI / 2.0) * Quaternion::rotation_z(PI);
                                next.control_r.orientation = Quaternion::rotation_x(PI / 2.0 + 0.2)
                                    * Quaternion::rotation_y(-1.0)
                                    * Quaternion::rotation_z(0.0);

                                next.control.orientation =
                                    Quaternion::rotation_x(-1.4) * Quaternion::rotation_y(-2.8);

                                next.shoulder_l.position = Vec3::new(
                                    -s_a.shoulder.0,
                                    s_a.shoulder.1,
                                    s_a.shoulder.2 - foothorir * 1.0,
                                );
                                next.shoulder_l.orientation = Quaternion::rotation_x(-0.4);
                                next.shoulder_r.orientation =
                                    Quaternion::rotation_y(0.4) * Quaternion::rotation_x(0.4);
                            },
                            "Husk Brute" | "TerracottaDemolisher" => {
                                if speed > 0.1 {
                                    next.hand_l.position =
                                        Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2);
                                    next.hand_l.orientation =
                                        Quaternion::rotation_x(1.4 + slow * 0.1)
                                            * Quaternion::rotation_z(-PI / 2.0);

                                    next.hand_r.position =
                                        Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2);
                                    next.hand_r.orientation =
                                        Quaternion::rotation_x(1.4 - slow * 0.1)
                                            * Quaternion::rotation_z(PI / 2.0);

                                    next.shoulder_l.position =
                                        Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
                                    next.shoulder_l.orientation =
                                        Quaternion::rotation_x(1.1 + slow * 0.1);

                                    next.shoulder_r.position =
                                        Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
                                    next.shoulder_r.orientation =
                                        Quaternion::rotation_x(1.1 - slow * 0.1);
                                } else {
                                    next.shoulder_l.position =
                                        Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
                                    next.shoulder_l.orientation = Quaternion::rotation_x(breathe);

                                    next.shoulder_r.position =
                                        Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
                                    next.shoulder_r.orientation = Quaternion::rotation_x(breathe);

                                    next.hand_l.position = Vec3::new(
                                        -s_a.hand.0,
                                        s_a.hand.1,
                                        s_a.hand.2 + torso * -0.1,
                                    );

                                    next.hand_r.position = Vec3::new(
                                        s_a.hand.0,
                                        s_a.hand.1,
                                        s_a.hand.2 + torso * -0.1,
                                    );

                                    next.leg_l.position =
                                        Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2 + torso * -0.2);

                                    next.leg_r.position =
                                        Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2 + torso * -0.2);

                                    next.foot_l.position =
                                        Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2);

                                    next.foot_r.position =
                                        Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2);
                                }
                            },
                            _ => {},
                        }
                    }
                },
                _ => {},
            }
        }

        if s_a.float {
            next.upper_torso.position = Vec3::new(
                0.0,
                s_a.upper_torso.0,
                s_a.upper_torso.1 + slower * 1.0 + 4.0,
            );
            next.foot_l.orientation = Quaternion::rotation_x(-0.5 + slow * 0.1);
            next.foot_r.orientation = Quaternion::rotation_x(-0.5 + slow * 0.1);
        }

        next
    }
}
