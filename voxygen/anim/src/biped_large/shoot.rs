use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::{
    comp::item::tool::{AbilitySpec, ToolKind},
    states::utils::StageSection,
};
use core::f32::consts::PI;

pub struct ShootAnimation;

type ShootAnimationDependency<'a> = (
    Option<ToolKind>,
    (Option<ToolKind>, Option<&'a AbilitySpec>),
    Vec3<f32>,
    Vec3<f32>,
    Vec3<f32>,
    f32,
    Option<StageSection>,
    f32,
    Option<&'a str>,
);
impl Animation for ShootAnimation {
    type Dependency<'a> = ShootAnimationDependency<'a>;
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_shoot\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_shoot")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            active_tool_kind,
            _second_tool,
            velocity,
            _orientation,
            _last_ori,
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
        let speed = Vec2::<f32>::from(velocity).magnitude();

        let mut next = (*skeleton).clone();

        let lab: f32 = 0.65 * s_a.tempo;
        let speednorm = (speed / 12.0).powf(0.4);
        let foothoril = (acc_vel * lab + PI * 1.45).sin() * speednorm;
        let foothorir = (acc_vel * lab + PI * (0.45)).sin() * speednorm;
        let footrotl = ((1.0 / (0.5 + (0.5) * ((acc_vel * lab + PI * 1.4).sin()).powi(2))).sqrt())
            * ((acc_vel * lab + PI * 1.4).sin())
            * speednorm;

        let footrotr = ((1.0 / (0.5 + (0.5) * ((acc_vel * lab + PI * 0.4).sin()).powi(2))).sqrt())
            * ((acc_vel * lab + PI * 0.4).sin())
            * speednorm;

        next.shoulder_l.position = Vec3::new(
            -s_a.shoulder.0,
            s_a.shoulder.1,
            s_a.shoulder.2 - foothorir * 1.0,
        );
        next.shoulder_l.orientation =
            Quaternion::rotation_x(0.8 + 1.2 * speednorm + (footrotr * -0.2) * speednorm);

        next.shoulder_r.position = Vec3::new(
            s_a.shoulder.0,
            s_a.shoulder.1,
            s_a.shoulder.2 - foothoril * 1.0,
        );
        next.shoulder_r.orientation =
            Quaternion::rotation_x(0.8 + 1.2 * speednorm + (footrotl * -0.2) * speednorm);
        next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
        next.jaw.orientation = Quaternion::rotation_x(0.0);

        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_x(0.0);

        next.hand_l.position = Vec3::new(0.0, 0.0, s_a.grip.0);
        next.hand_r.position = Vec3::new(0.0, 0.0, s_a.grip.0);

        next.hand_l.orientation = Quaternion::rotation_x(0.0);
        next.hand_r.orientation = Quaternion::rotation_x(0.0);

        match active_tool_kind {
            Some(ToolKind::Staff) | Some(ToolKind::Sceptre) => {
                let (move1base, move1shake, move2base, move3) = match stage_section {
                    Some(StageSection::Buildup) => {
                        (anim_time, (anim_time * 10.0 + PI).sin(), 0.0, 0.0)
                    },
                    Some(StageSection::Action) => (1.0, 1.0, anim_time.powf(0.25), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1base * pullback;
                let move2 = move2base * pullback;
                next.control_l.position = Vec3::new(-1.0, 3.0, 12.0);
                next.control_r.position = Vec3::new(1.0, 2.0, 2.0);

                next.control.position = Vec3::new(
                    -3.0,
                    3.0 + s_a.grip.0 / 1.2 + move1 * 4.0 + move2 + move1shake * 2.0 + move2 * -2.0,
                    -11.0 + -s_a.grip.0 / 2.0 + move1 * 3.0,
                );
                next.head.orientation = Quaternion::rotation_x(move1 * -0.15)
                    * Quaternion::rotation_y(move1 * 0.25)
                    * Quaternion::rotation_z(move1 * 0.25);
                next.jaw.orientation = Quaternion::rotation_x(move1 * -0.5);

                next.control_l.orientation = Quaternion::rotation_x(PI / 2.0 + move1 * 0.5)
                    * Quaternion::rotation_y(move1 * -0.4);
                next.control_r.orientation = Quaternion::rotation_x(PI / 2.5 + move1 * 0.5)
                    * Quaternion::rotation_y(0.5)
                    * Quaternion::rotation_z(0.0);

                next.control.orientation =
                    Quaternion::rotation_x(-0.2 + move1 * -0.2 + move1shake * 0.1)
                        * Quaternion::rotation_y(-0.1 + move1 * 0.8 + move2 * -0.3);
                next.shoulder_l.position = Vec3::new(
                    -s_a.shoulder.0,
                    s_a.shoulder.1,
                    s_a.shoulder.2 - foothorir * 1.0,
                );
                next.shoulder_l.orientation = Quaternion::rotation_x(
                    move1 * 0.8 + 0.8 * speednorm + (footrotr * -0.2) * speednorm,
                );

                next.shoulder_r.position = Vec3::new(
                    s_a.shoulder.0,
                    s_a.shoulder.1,
                    s_a.shoulder.2 - foothoril * 1.0,
                );
                next.shoulder_r.orientation =
                    Quaternion::rotation_x(move1 * 0.8 + 0.6 * speednorm + (footrotl * -0.2));
            },
            Some(ToolKind::Bow) => {
                let (move1base, move2base, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1base * pullback;
                let move2 = move2base * pullback;
                next.control_l.position = Vec3::new(-1.0, -2.0 + move2 * -7.0, -3.0);
                next.control_r.position = Vec3::new(0.0, 4.0, 1.0);

                next.control.position = Vec3::new(
                    -1.0 + move1 * 2.0,
                    6.0 + s_a.grip.0 / 1.2 + move1 * 7.0,
                    -5.0 + -s_a.grip.0 / 2.0 + move1 * s_a.height * 3.4,
                );

                next.control_l.orientation =
                    Quaternion::rotation_x(move1 * 0.2 + PI / 2.0 + move2 * 0.4)
                        * Quaternion::rotation_y(-0.2);
                next.control_r.orientation = Quaternion::rotation_x(PI / 2.2 + move1 * 0.4)
                    * Quaternion::rotation_y(0.4)
                    * Quaternion::rotation_z(0.0);

                next.control.orientation = Quaternion::rotation_x(-0.2)
                    * Quaternion::rotation_y(1.0 + move1 * -0.4)
                    * Quaternion::rotation_z(-0.1);
                next.head.orientation = Quaternion::rotation_z(move1 * 0.25);
                next.shoulder_l.position = Vec3::new(
                    -s_a.shoulder.0,
                    s_a.shoulder.1,
                    s_a.shoulder.2 - foothorir * 1.0,
                );
                next.shoulder_l.orientation =
                    Quaternion::rotation_x(move1 * 1.2 + 1.2 * speednorm + (footrotr * -0.2));

                next.shoulder_r.position = Vec3::new(
                    s_a.shoulder.0,
                    s_a.shoulder.1,
                    s_a.shoulder.2 - foothoril * 1.0,
                );
                next.shoulder_r.orientation =
                    Quaternion::rotation_x(move1 * 0.8 + 1.2 * speednorm + (footrotl * -0.2));
            },
            Some(ToolKind::Axe) => {
                let (move1base, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => ((anim_time.powf(0.25)), 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, (anim_time), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1base * pullback;
                let move2 = move2 * pullback;

                next.shoulder_r.orientation =
                    Quaternion::rotation_y(move1 * -0.5) * Quaternion::rotation_x(move1 * -0.5);
                next.head.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(move1 * 0.3)
                    * Quaternion::rotation_z(move1 * -0.2 + move2 * 0.5);

                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_z(move1 * 5.0);

                next.hand_l.position = Vec3::new(s_a.grip.1, 0.0, s_a.grip.0);
                next.hand_r.position = Vec3::new(-s_a.grip.1, 0.0, s_a.grip.0);

                next.hand_l.orientation = Quaternion::rotation_x(0.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.0);

                next.control_l.position = Vec3::new(-1.0, 2.0, 12.0);
                next.control_r.position = Vec3::new(1.0 + move1 * 40.0, 2.0, -2.0 + move1 * 10.0);

                next.control.position = Vec3::new(
                    4.0 + move1 * -25.0,
                    0.0 + s_a.grip.0 / 1.0 + move1 * -6.0,
                    -s_a.grip.0 / 0.8 + move1 * 10.0,
                );

                next.control_l.orientation =
                    Quaternion::rotation_x(PI / 2.0 + move1 * 0.3) * Quaternion::rotation_y(-0.0);
                next.control_r.orientation = Quaternion::rotation_x(PI / 2.0 + 0.2 + move1 * 1.0)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.control.orientation = Quaternion::rotation_x(-1.0 + move1 * 0.0)
                    * Quaternion::rotation_y(-1.8 + move1 * 2.0)
                    * Quaternion::rotation_z(0.0 + move1 * -0.0);
                next.upper_torso.orientation = Quaternion::rotation_y(move1 * 0.3);

                next.lower_torso.orientation = Quaternion::rotation_y(move1 * -0.3);
                next.torso.position = Vec3::new(move1, 0.0, 0.0);
            },
            Some(ToolKind::Natural) => match ability_id {
                Some("common.abilities.custom.wendigomagic.frostbomb") => {
                    let (move1base, _move2base, move3) = match stage_section {
                        Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                        Some(StageSection::Action) => (1.0, anim_time, 0.0),
                        Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                        _ => (0.0, 0.0, 0.0),
                    };
                    let pullback = 1.0 - move3;
                    let move1 = move1base * pullback;
                    next.control_l.position =
                        Vec3::new(-9.0 + move1 * 6.0, 19.0 + move1 * 6.0, -13.0 + move1 * 10.5);
                    next.control_r.position =
                        Vec3::new(9.0 + move1 * -6.0, 19.0 + move1 * 6.0, -13.0 + move1 * 14.5);

                    next.control_l.orientation = Quaternion::rotation_x(PI / 3.0 + move1 * 0.5)
                        * Quaternion::rotation_y(-0.15)
                        * Quaternion::rotation_z(move1 * 0.5);
                    next.control_r.orientation = Quaternion::rotation_x(PI / 3.0 + move1 * 0.5)
                        * Quaternion::rotation_y(0.15)
                        * Quaternion::rotation_z(move1 * -0.5);
                    next.head.orientation = Quaternion::rotation_x(move1 * -0.3);
                },
                Some("common.abilities.custom.yeti.snowball") => {
                    let (move1, move2, move3) = match stage_section {
                        Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                        Some(StageSection::Action) => (1.0, anim_time, 0.0),
                        Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                        _ => (0.0, 0.0, 0.0),
                    };
                    next.second.scale = Vec3::one() * 0.0;

                    next.head.orientation = Quaternion::rotation_x(move1 * 0.4);
                    next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
                    next.jaw.orientation = Quaternion::rotation_x(move2 * -0.3);
                    next.control_l.position = Vec3::new(-0.5, 4.0, 1.0);
                    next.control_r.position = Vec3::new(-0.5, 4.0, 1.0);
                    next.control_l.orientation = Quaternion::rotation_x(PI / 2.0);
                    next.control_r.orientation = Quaternion::rotation_x(PI / 2.0);
                    next.weapon_l.position = Vec3::new(-12.0, -1.0, -15.0);
                    next.weapon_r.position = Vec3::new(12.0, -1.0, -15.0);

                    next.weapon_l.orientation = Quaternion::rotation_x(-PI / 2.0 - 0.1);
                    next.weapon_r.orientation = Quaternion::rotation_x(-PI / 2.0 - 0.1);

                    let twist = move1 * 0.8 + move3 * -0.8;
                    next.upper_torso.position =
                        Vec3::new(0.0, s_a.upper_torso.0, s_a.upper_torso.1);
                    next.upper_torso.orientation =
                        Quaternion::rotation_x(move1 * 0.8 + move2 * -1.1)
                            * Quaternion::rotation_z(twist * -0.2 + move1 * -0.1 + move2 * 0.3);

                    next.lower_torso.orientation =
                        Quaternion::rotation_x(move1 * -0.8 + move2 * 1.1)
                            * Quaternion::rotation_z(twist);

                    next.arm_control_r.orientation = Quaternion::rotation_x(move1 * PI / 2.0)
                        * Quaternion::rotation_y(move1 * -PI / 2.0 + move2 * 2.5);
                    //* Quaternion::rotation_y(move1 * -PI/2.0)
                    //* Quaternion::rotation_z(move1 * -PI/2.0);
                    next.arm_control_r.position = Vec3::new(0.0, move1 * 10.0 + move2 * -10.0, 0.0);
                },
                Some("common.abilities.custom.harvester.explodingpumpkin") => {
                    let (move1, move2, move3) = match stage_section {
                        Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                        Some(StageSection::Action) => (1.0, anim_time, 0.0),
                        Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                        _ => (0.0, 0.0, 0.0),
                    };
                    next.control_l.position = Vec3::new(1.0, 2.0, 8.0);
                    next.control_r.position = Vec3::new(1.0, 1.0, -2.0);

                    next.control.position =
                        Vec3::new(-7.0, 0.0 + s_a.grip.0 / 1.0, -s_a.grip.0 / 0.8);

                    next.control_l.orientation =
                        Quaternion::rotation_x(PI / 2.0) * Quaternion::rotation_z(PI);
                    next.control_r.orientation = Quaternion::rotation_x(PI / 2.0 + 0.2)
                        * Quaternion::rotation_y(-1.0)
                        * Quaternion::rotation_z(0.0);

                    next.control.orientation =
                        Quaternion::rotation_x(-1.4) * Quaternion::rotation_y(-2.8);

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
                            * Quaternion::rotation_z(twist * -0.2 + move1 * -0.1 + move2 * 0.3);

                    next.lower_torso.orientation =
                        Quaternion::rotation_x(move1 * -0.8 + move2 * 1.1)
                            * Quaternion::rotation_z(-twist + move1 * 0.4);

                    next.shoulder_l.position = Vec3::new(
                        -s_a.shoulder.0,
                        s_a.shoulder.1,
                        s_a.shoulder.2 - foothorir * 1.0,
                    );
                    next.shoulder_l.orientation = Quaternion::rotation_x(-0.4);

                    next.shoulder_r.position = Vec3::new(
                        s_a.shoulder.0 + move2 * -2.0,
                        s_a.shoulder.1,
                        s_a.shoulder.2,
                    );
                    next.shoulder_r.orientation = Quaternion::rotation_y(move1 * -PI / 2.0)
                        * Quaternion::rotation_x(move2 * 2.0)
                        * Quaternion::rotation_z(move1 * -PI / 2.0);

                    next.hand_r.position = Vec3::new(
                        -s_a.grip.1 + move1 * -2.0 + move2 * 8.0,
                        0.0 + move1 * 6.0,
                        s_a.grip.0 + move1 * 18.0 + move2 * -19.0,
                    );
                    next.hand_r.orientation = Quaternion::rotation_x(move1 * -3.0 + move2 * 3.0)
                        * Quaternion::rotation_y(move1 * 0.5 + move2 * -1.5)
                        * Quaternion::rotation_z(move1 * -1.5);

                    if speed == 0.0 {
                        next.leg_l.orientation = Quaternion::rotation_x(move1 * 0.8 + move2 * -0.8);

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
            },
            _ => {},
        }

        next
    }
}
