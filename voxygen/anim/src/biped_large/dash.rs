use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::{
    comp::item::tool::{AbilitySpec, ToolKind},
    states::utils::StageSection,
};
use core::f32::consts::PI;

pub struct DashAnimation;

impl Animation for DashAnimation {
    type Dependency<'a> = (
        Option<ToolKind>,
        (Option<ToolKind>, Option<&'a AbilitySpec>),
        Vec3<f32>,
        f32,
        Option<StageSection>,
        f32,
        Option<&'a str>,
    );
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_dash\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_dash")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            active_tool_kind,
            _second_tool,
            velocity,
            _global_time,
            stage_section,
            acc_vel,
            ability_id,
        ): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();
        let lab: f32 = 0.65 * s_a.tempo;
        let speed = Vec2::<f32>::from(velocity).magnitude();

        let speednorm = (speed.min(16.0) / 12.0).powf(0.4);
        let foothoril = (acc_vel * lab + PI * 1.45).sin() * speednorm;
        let foothorir = (acc_vel * lab + PI * (0.45)).sin() * speednorm;
        let footrotl = ((1.0 / (0.5 + (0.5) * ((acc_vel * lab + PI * 1.4).sin()).powi(2))).sqrt())
            * ((acc_vel * lab + PI * 1.4).sin());

        let footrotr = ((1.0 / (0.5 + (0.5) * ((acc_vel * lab + PI * 0.4).sin()).powi(2))).sqrt())
            * ((acc_vel * lab + PI * 0.4).sin());

        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_x(0.0);

        next.hand_l.position = Vec3::new(0.0, 0.0, s_a.grip.0);
        next.hand_r.position = Vec3::new(0.0, 0.0, s_a.grip.0);
        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation = Quaternion::rotation_x(0.0);
        next.hand_l.orientation = Quaternion::rotation_x(0.0);
        next.hand_r.orientation = Quaternion::rotation_x(0.0);
        let (move1base, motion, move2base, move3base, move4) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0, 0.0, 0.0),
            Some(StageSection::Charge) => (
                1.0,
                (acc_vel * lab).sin(),
                (anim_time.powf(4.0)).min(1.0),
                0.0,
                0.0,
            ),
            Some(StageSection::Action) => (1.0, 1.0, 1.0, anim_time.powf(4.0), 0.0),
            Some(StageSection::Recover) => (1.1, 1.0, 1.0, 1.0, anim_time.powf(4.0)),
            _ => (0.0, 0.0, 0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - move4;
        let move1 = move1base * pullback;
        let move2 = move2base * pullback;
        let move3 = move3base * pullback;

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
        next.torso.orientation = Quaternion::rotation_z(0.0);
        match active_tool_kind {
            Some(ToolKind::Sword) => {
                next.control_l.position = Vec3::new(-1.0, 1.0, 1.0);
                next.control_r.position = Vec3::new(0.0, 2.0, -3.0);
                next.head.orientation = Quaternion::rotation_x(move1 * -0.25)
                    * Quaternion::rotation_z(move1 * -0.2 + move2 * 0.6);
                next.control.position = Vec3::new(
                    -3.0 + move1 * -2.0 + move2 * 2.0,
                    5.0 + s_a.grip.0 / 1.2 + move1 * -4.0 + move2 * 2.0 + move3 * 8.0,
                    -4.0 + -s_a.grip.0 / 2.0 + move2 * -5.0 + move3 * 5.0,
                );
                next.upper_torso.orientation = Quaternion::rotation_x(move2 * -0.2 + move3 * 0.2)
                    * Quaternion::rotation_z(move1 * 0.8 + move3 * -0.7);
                next.lower_torso.orientation = Quaternion::rotation_x(move2 * 0.2 + move3 * -0.2)
                    * Quaternion::rotation_z(move1 * -0.8 + move3 * 0.7);
                next.control_l.orientation =
                    Quaternion::rotation_x(PI / 2.0 + move1 * -0.5 + move2 * 1.5)
                        * Quaternion::rotation_y(-0.2);
                next.control_r.orientation =
                    Quaternion::rotation_x(PI / 2.2 + move1 * -0.5 + move2 * 1.5)
                        * Quaternion::rotation_y(0.2)
                        * Quaternion::rotation_z(0.0);

                next.control.orientation =
                    Quaternion::rotation_x(-0.2 + move1 * 0.5 + move2 * -1.5 + move3 * -0.2)
                        * Quaternion::rotation_y(-0.1 + move1 * -0.5 + move2 * 1.5 + move3 * -1.0)
                        * Quaternion::rotation_z(-move3 * -1.5);
            },
            Some(ToolKind::Axe) => {
                next.control_l.position = Vec3::new(-1.0, 2.0, 12.0 + move3 * 3.0);
                next.control_r.position = Vec3::new(1.0, 2.0, -2.0);

                next.control.position = Vec3::new(
                    4.0 + move1 * -3.0 + move3 * -5.0,
                    (s_a.grip.0 / 1.0) + move1 * -1.0 + move3 * 1.0 + footrotl * 2.0,
                    (-s_a.grip.0 / 0.8) + move1 * 2.0 + move3 * -3.0,
                );
                next.head.orientation = Quaternion::rotation_x(move1 * -0.5 + move3 * 0.5)
                    * Quaternion::rotation_z(move1 * 0.3 + move3 * 0.3);
                next.upper_torso.orientation = Quaternion::rotation_x(move1 * -0.4 + move3 * 0.9)
                    * Quaternion::rotation_z(move1 * 0.6 + move3 * -1.5);
                next.lower_torso.orientation = Quaternion::rotation_y(move1 * -0.2 + move3 * -0.1)
                    * Quaternion::rotation_x(move1 * 0.4 + move3 * -0.7 + footrotr * 0.1)
                    * Quaternion::rotation_z(move1 * -0.6 + move3 * 1.6);

                next.control_l.orientation = Quaternion::rotation_x(PI / 2.0 + move3 * 0.3)
                    * Quaternion::rotation_y(move1 * 0.7);
                next.control_r.orientation = Quaternion::rotation_x(PI / 2.0 + 0.2 + move3 * -0.2)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);

                next.control.orientation =
                    Quaternion::rotation_x(-1.0 + move1 * -0.2 + move3 * -0.2)
                        * Quaternion::rotation_y(-1.8 + move1 * -0.2 + move3 * -0.2)
                        * Quaternion::rotation_z(move1 * -0.8 + move3 * -0.1);
            },
            Some(ToolKind::Natural) => match ability_id {
                Some("common.abilities.custom.minotaur.charge") => {
                    next.head.orientation = Quaternion::rotation_x(move1 * 0.4 + move3 * 0.5)
                        * Quaternion::rotation_z(move1 * -0.3 + move3 * -0.3);
                    next.upper_torso.orientation =
                        Quaternion::rotation_x(move1 * -0.4 + move3 * 0.9)
                            * Quaternion::rotation_z(move1 * 0.6 + move3 * -1.5);
                    next.lower_torso.orientation =
                        Quaternion::rotation_y(move1 * -0.2 + move3 * -0.1)
                            * Quaternion::rotation_x(move1 * 0.4 + move3 * -0.7 + footrotr * 0.1)
                            * Quaternion::rotation_z(move1 * -0.6 + move3 * 1.6);
                    next.control_l.position = Vec3::new(0.0, 4.0, 5.0);
                    next.control_r.position = Vec3::new(0.0, 4.0, 5.0);
                    next.weapon_l.position = Vec3::new(-12.0 + move1 * -3.0, -6.0, -18.0);
                    next.weapon_r.position =
                        Vec3::new(12.0 + move1 * -3.0, -6.0 + move1 * 2.0, -18.0 + move1 * 2.0);
                    next.second.scale = Vec3::one() * 1.0;

                    next.weapon_l.orientation = Quaternion::rotation_x(-1.67 + move1 * 0.4)
                        * Quaternion::rotation_y(move1 * 0.4 + move2 * 0.2)
                        * Quaternion::rotation_z(move3 * -0.5);
                    next.weapon_r.orientation = Quaternion::rotation_x(-1.67 + move1 * 0.3)
                        * Quaternion::rotation_y(move1 * 0.6 + move2 * -0.6)
                        * Quaternion::rotation_z(move3 * -0.5);

                    next.control_l.orientation = Quaternion::rotation_x(PI / 2.0);
                    next.control_r.orientation = Quaternion::rotation_x(PI / 2.0);

                    next.control.orientation =
                        Quaternion::rotation_x(0.0) * Quaternion::rotation_y(0.0);
                    next.shoulder_l.orientation = Quaternion::rotation_x(-0.3);

                    next.shoulder_r.orientation = Quaternion::rotation_x(-0.3);
                },
                Some("common.abilities.custom.tidalwarrior.scuttle") => {
                    next.head.orientation =
                        Quaternion::rotation_x(0.0) * Quaternion::rotation_z(move1 * -0.3);
                    next.upper_torso.orientation = Quaternion::rotation_x(move1 * -0.1)
                        * Quaternion::rotation_z(move1 * PI / 2.0);
                    next.lower_torso.orientation = Quaternion::rotation_x(move1 * 0.1)
                        * Quaternion::rotation_x(move1 * -0.1)
                        * Quaternion::rotation_z(move1 * -0.2);

                    next.hand_l.position = Vec3::new(-14.0, 2.0 + motion * 1.5, -4.0);

                    next.hand_l.orientation = Quaternion::rotation_x(PI / 3.0 + move1 * 1.0)
                        * Quaternion::rotation_y(0.0)
                        * Quaternion::rotation_z(-0.35 + motion * -0.6);
                    next.hand_r.position = Vec3::new(14.0, 2.0 + motion * -1.5, -4.0);

                    next.hand_r.orientation = Quaternion::rotation_x(PI / 3.0 + move1 * 1.0)
                        * Quaternion::rotation_y(0.0)
                        * Quaternion::rotation_z(0.35 + motion * 0.6);

                    next.shoulder_l.orientation = Quaternion::rotation_x(move1 * 0.8);

                    next.shoulder_r.orientation = Quaternion::rotation_x(move1 * 0.8);
                },
                _ => {},
            },
            _ => {},
        }

        next
    }
}
