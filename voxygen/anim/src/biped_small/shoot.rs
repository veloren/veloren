use super::{
    super::{vek::*, Animation},
    BipedSmallSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};
use std::f32::consts::PI;

pub struct ShootAnimation;

type ShootAnimationDependency<'a> = (
    Option<ToolKind>,
    Vec3<f32>,
    Vec3<f32>,
    Vec3<f32>,
    f32,
    Vec3<f32>,
    f32,
    Option<StageSection>,
    f32,
    Option<&'a str>,
);

impl Animation for ShootAnimation {
    type Dependency<'a> = ShootAnimationDependency<'a>;
    type Skeleton = BipedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_small_shoot\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_small_shoot")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            active_tool_kind,
            velocity,
            _orientation,
            _last_ori,
            _global_time,
            _avg_vel,
            _acc_vel,
            stage_section,
            _timer,
            ability_id,
        ): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();

        let fast = (anim_time * 10.0).sin();
        let fastalt = (anim_time * 10.0 + PI / 2.0).sin();

        let speednorm = speed / 9.4;
        let speednormcancel = 1.0 - speednorm;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1 + fast * -0.1 * speednormcancel);
        next.head.orientation = Quaternion::rotation_x(0.45 * speednorm)
            * Quaternion::rotation_y(fast * 0.07 * speednormcancel);
        next.chest.position = Vec3::new(
            0.0,
            s_a.chest.0,
            s_a.chest.1 + fastalt * 0.4 * speednormcancel + speednormcancel * -0.5,
        );

        next.pants.position = Vec3::new(0.0, s_a.pants.0, s_a.pants.1);

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.tail.orientation = Quaternion::rotation_x(0.05 * fastalt * speednormcancel)
            * Quaternion::rotation_z(fast * 0.15 * speednormcancel);

        match ability_id {
            Some("common.abilities.adlet.hunter.throw") => {
                next.main.position = Vec3::new(8.0, -7.0, 2.0);
                next.main.orientation = Quaternion::rotation_x(PI / -2.0);

                next.hand_l.position = Vec3::new(s_a.grip.0 * 4.0, 0.0, s_a.grip.2);
                next.hand_r.position = Vec3::new(5.0, -2.0, s_a.grip.2);

                next.hand_l.orientation = Quaternion::rotation_x(0.0);
                next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0);
            },
            Some("common.abilities.adlet.tracker.trap") => {
                next.main.position = Vec3::new(10.0, 0.0, 9.0);

                next.hand_l.position = Vec3::new(s_a.grip.0 * 4.0, 0.0, s_a.grip.2);
                next.hand_r.position = Vec3::new(5.0, -2.0, s_a.grip.2);

                next.hand_l.orientation = Quaternion::rotation_x(0.0);
                next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0);
            },
            _ => {
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);

                next.hand_l.position = Vec3::new(s_a.grip.0 * 4.0, 0.0, s_a.grip.2);
                next.hand_r.position = Vec3::new(-s_a.grip.0 * 4.0, 0.0, s_a.grip.2);

                next.hand_l.orientation = Quaternion::rotation_x(0.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.0);
            },
        };

        match active_tool_kind {
            Some(ToolKind::Bow) => match ability_id {
                Some("common.abilities.adlet.tracker.trap") => {
                    let (move1base, move2base, move3) = match stage_section {
                        Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                        Some(StageSection::Action) => (1.0, anim_time.powf(0.25), 0.0),
                        Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                        _ => (0.0, 0.0, 0.0),
                    };
                    let pullback = 1.0 - move3;
                    let move1abs = move1base * pullback;
                    let move2abs = move2base * pullback;
                    next.main.position = Vec3::new(2.0, -3.0, -3.0);

                    next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2 - 2.0);
                    next.hand_r.position = Vec3::new(
                        s_a.hand.0,
                        s_a.hand.1 + move2abs * 1.5,
                        s_a.hand.2 - 2.5 + move1abs * 2.5,
                    );

                    next.hand_r.orientation =
                        Quaternion::rotation_x(move1abs * 4.0 + move2abs * -0.7)
                            * Quaternion::rotation_y(0.0 + move1abs * -0.7);

                    next.hand_l.orientation = Quaternion::rotation_x(PI / 3.0 * move1abs)
                        * Quaternion::rotation_y(-0.7 * move1abs + move2abs * 0.1);
                },
                _ => {
                    let (move1base, move2base, move3) = match stage_section {
                        Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                        Some(StageSection::Action) => (1.0, anim_time.powf(0.25), 0.0),
                        Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                        _ => (0.0, 0.0, 0.0),
                    };
                    let pullback = 1.0 - move3;
                    let move1abs = move1base * pullback;
                    let move2abs = move2base * pullback;
                    next.control_l.position = Vec3::new(
                        -1.0 - s_a.grip.0 * 2.0 + move2abs * -4.0,
                        move2abs * -8.0,
                        0.0,
                    );
                    next.control_r.position = Vec3::new(1.0 + s_a.grip.0 * 2.0, 3.0, -2.0);

                    next.control.position = Vec3::new(
                        -1.0,
                        2.0 + move1abs * 3.0 + s_a.grip.2,
                        3.0 + move1abs * 7.0 - s_a.grip.2 / 2.5 + s_a.grip.0 * -2.0,
                    );

                    next.control_l.orientation =
                        Quaternion::rotation_x(PI / 2.0) * Quaternion::rotation_y(-0.3);
                    next.control_r.orientation =
                        Quaternion::rotation_x(PI / 2.0 + s_a.grip.0 * 0.2)
                            * Quaternion::rotation_y(0.5 + s_a.grip.0 * 0.2);

                    next.control.orientation = Quaternion::rotation_x(-0.3 + move1abs * 0.4)
                        * Quaternion::rotation_y(0.5 * speednorm);
                },
            },
            Some(ToolKind::Natural) => match ability_id {
                Some("common.abilities.custom.irrwurz.magicball") => {
                    let (move1base, move2base, move3) = match stage_section {
                        Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                        Some(StageSection::Action) => (1.0, anim_time.powf(0.25), 0.0),
                        Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                        _ => (0.0, 0.0, 0.0),
                    };
                    let pullback = 1.0 - move3;
                    let move1abs = move1base * pullback;
                    let move2abs = move2base * pullback;
                    next.main.position = Vec3::new(2.0, -3.0, -3.0);

                    next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2 - 2.0);
                    next.hand_r.position = Vec3::new(
                        s_a.hand.0,
                        s_a.hand.1 + move2abs * 1.5,
                        s_a.hand.2 - 2.5 + move1abs * 2.5,
                    );

                    next.hand_r.orientation =
                        Quaternion::rotation_x(move1abs * 4.0 + move2abs * -0.7);

                    next.hand_l.orientation = Quaternion::rotation_x(PI / 3.0 * move1abs)
                        * Quaternion::rotation_y(-0.7 * move1abs + move2abs * 0.1);
                },
                Some("common.abilities.custom.goblin_chucker.throw") => {
                    let (move1base, move2base, move3) = match stage_section {
                        Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                        Some(StageSection::Action) => (1.0, anim_time.powf(0.25), 0.0),
                        Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                        _ => (0.0, 0.0, 0.0),
                    };
                    let pullback = 1.0 - move3;
                    let move1abs = move1base * pullback;
                    let move2abs = move2base * pullback;

                    next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2 - 2.0);
                    next.hand_r.position = Vec3::new(
                        s_a.hand.0,
                        s_a.hand.1 + move2abs * 1.5,
                        s_a.hand.2 - 2.5 + move1abs * 2.5,
                    );

                    next.hand_r.orientation =
                        Quaternion::rotation_x(move1abs * 4.0 + move2abs * -0.7)
                            * Quaternion::rotation_y(0.0 + move1abs * -0.7);

                    next.hand_l.orientation = Quaternion::rotation_x(PI / 3.0 * move1abs)
                        * Quaternion::rotation_y(-0.7 * move1abs + move2abs * 0.1);
                },
                _ => {
                    let (move1base, move2base, move3) = match stage_section {
                        Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                        Some(StageSection::Action) => (1.0, anim_time.powf(0.25), 0.0),
                        Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                        _ => (0.0, 0.0, 0.0),
                    };
                    let pullback = 1.0 - move3;
                    let move1abs = move1base * pullback;
                    let move2abs = move2base * pullback;
                    next.control_l.position = Vec3::new(
                        1.0 - s_a.grip.0 * 2.0 + move2abs * -4.0,
                        move2abs * -8.0,
                        0.0,
                    );
                    next.control_r.position = Vec3::new(-1.0 + s_a.grip.0 * 2.0, 6.0, -2.0);

                    next.control.position = Vec3::new(
                        -1.0,
                        2.0 + move1abs * 3.0 + s_a.grip.2,
                        3.0 + move1abs * 7.0 - s_a.grip.2 / 2.5 + s_a.grip.0 * -2.0,
                    );

                    next.control_l.orientation =
                        Quaternion::rotation_x(PI / 2.0) * Quaternion::rotation_y(-0.3);
                    next.control_r.orientation =
                        Quaternion::rotation_x(PI / 2.0 + s_a.grip.0 * 0.2)
                            * Quaternion::rotation_y(0.5 + s_a.grip.0 * 0.2);

                    next.control.orientation = Quaternion::rotation_x(-0.3 + move1abs * 0.4)
                        * Quaternion::rotation_y(0.5 * speednorm);
                },
            },
            Some(ToolKind::Blowgun) => {
                let (move1base, move2base, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time.powf(0.25), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1abs = move1base * pullback;
                let move2abs = move2base * pullback;
                next.head.orientation = Quaternion::rotation_x(move1abs * 0.5 + move2abs * -0.7);
                next.control_l.position = Vec3::new(1.0 - s_a.grip.0 * 2.0, 0.0, 3.0);
                next.control_r.position = Vec3::new(-1.0 + s_a.grip.0 * 2.0, 0.0, 4.0);

                next.control.position = Vec3::new(
                    0.0,
                    move1abs * -2.0 + move2abs * 1.0 + s_a.grip.2,
                    move1abs * 3.5 + move2abs * -2.0 + 4.0 - s_a.grip.2 / 2.5 + s_a.grip.0 * -2.0,
                );

                next.control_l.orientation =
                    Quaternion::rotation_x(3.8 + move1abs * 0.4 + move2abs * -0.5)
                        * Quaternion::rotation_y(-0.3);
                next.control_r.orientation = Quaternion::rotation_x(
                    3.5 + move1abs * 0.4 + move2abs * -0.5 + s_a.grip.0 * 0.2,
                ) * Quaternion::rotation_y(0.5 + s_a.grip.0 * 0.2);

                next.control.orientation = Quaternion::rotation_x(
                    -2.2 + move1abs * 0.7 + move2abs * -0.2 + 0.5 * speednorm,
                );
            },
            Some(ToolKind::Staff) => match ability_id {
                Some(
                    "common.abilities.custom.dwarves.flamekeeper.mines"
                    | "common.abilities.custom.dwarves.flamekeeper.lava_mortar",
                ) => {
                    let (move1base, _move2base, move3) = match stage_section {
                        Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                        Some(StageSection::Action) => (1.0, anim_time.powf(0.25), 0.0),
                        Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                        _ => (0.0, 0.0, 0.0),
                    };
                    let pullback = 1.0 - move3;
                    let move1abs = move1base * pullback;
                    next.control_l.position = Vec3::new(2.0 - s_a.grip.0 * 2.0, 1.0, 3.0);
                    next.control_r.position = Vec3::new(
                        7.0 + s_a.grip.0 * 2.0 + move1abs * -8.0,
                        -4.0 + move1abs * 4.0,
                        3.0,
                    );

                    next.control.position = Vec3::new(
                        -5.0,
                        -1.0 + s_a.grip.2,
                        -2.0 + -s_a.grip.2 / 2.5 + s_a.grip.0 * -2.0 + move1abs * 5.0,
                    );

                    next.control_l.orientation = Quaternion::rotation_x(PI / 2.0 + move1abs * 0.8)
                        * Quaternion::rotation_y(-0.3)
                        * Quaternion::rotation_z(-0.3);
                    next.control_r.orientation =
                        Quaternion::rotation_x(PI / 2.0 + s_a.grip.0 * 0.2 + move1abs * 0.8)
                            * Quaternion::rotation_y(-0.4 + s_a.grip.0 * 0.2 + move1abs * 0.8)
                            * Quaternion::rotation_z(-0.0);

                    next.control.orientation = Quaternion::rotation_x(-0.3 + move1abs * -0.3)
                        * Quaternion::rotation_y(-0.2 * speednorm + move1abs * 0.3)
                        * Quaternion::rotation_z(0.5 + move1abs * 0.6);
                },
                _ => {
                    let (move1base, _move2base, move3) = match stage_section {
                        Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                        Some(StageSection::Action) => (1.0, anim_time.powf(0.25), 0.0),
                        Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                        _ => (0.0, 0.0, 0.0),
                    };
                    let pullback = 1.0 - move3;
                    let move1abs = move1base * pullback;
                    next.control_l.position = Vec3::new(2.0 - s_a.grip.0 * 2.0, 1.0, 3.0);
                    next.control_r.position = Vec3::new(
                        7.0 + s_a.grip.0 * 2.0 + move1abs * -8.0,
                        -4.0 + move1abs * 4.0,
                        3.0,
                    );

                    next.control.position = Vec3::new(
                        -5.0,
                        -1.0 + s_a.grip.2,
                        -2.0 + -s_a.grip.2 / 2.5 + s_a.grip.0 * -2.0 + move1abs * 5.0,
                    );

                    next.control_l.orientation = Quaternion::rotation_x(PI / 2.0 + move1abs * 0.8)
                        * Quaternion::rotation_y(-0.3)
                        * Quaternion::rotation_z(-0.3);
                    next.control_r.orientation =
                        Quaternion::rotation_x(PI / 2.0 + s_a.grip.0 * 0.2 + move1abs * 0.8)
                            * Quaternion::rotation_y(-0.4 + s_a.grip.0 * 0.2 + move1abs * 0.8)
                            * Quaternion::rotation_z(-0.0);

                    next.control.orientation = Quaternion::rotation_x(-0.3 + move1abs * -0.6)
                        * Quaternion::rotation_y(-0.2 * speednorm)
                        * Quaternion::rotation_z(0.5 + move1abs * 0.6);
                },
            },
            Some(ToolKind::Spear) => {
                let (move1base, move2base, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time.powf(0.25), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1abs = move1base * pullback;
                let move2abs = move2base * pullback;
                next.control_l.position = Vec3::new(
                    0.0,
                    0.0 + move1abs * 4.0 + move2abs * -4.0,
                    -2.0 + move1abs * -8.0,
                );
                next.control_r.position = Vec3::new(
                    -1.0 + s_a.grip.0 * 2.0,
                    2.0 + move3 * -4.0,
                    -2.0 + move3 * -4.0,
                );

                next.control.position = Vec3::new(
                    -1.0,
                    0.0 + move1abs * -8.0 + move2abs * 8.0 + s_a.grip.2,
                    3.0 + move1abs * 7.0 - s_a.grip.2 / 2.5 + s_a.grip.0 * -2.0,
                );

                next.control_l.orientation =
                    Quaternion::rotation_x(PI / 2.0) * Quaternion::rotation_y(-0.3);
                next.control_r.orientation = Quaternion::rotation_x(PI / 2.0 + s_a.grip.0 * 0.2)
                    * Quaternion::rotation_y(0.5 + s_a.grip.0 * 0.2 + move3 * -1.5);

                next.control.orientation =
                    Quaternion::rotation_x(-0.3 + move1abs * 0.5 + move2abs * -0.5 + move3 * 0.5)
                        * Quaternion::rotation_y(0.5 * speednorm);

                if matches!(stage_section, Some(StageSection::Recover)) {
                    if move3 < 0.05 {
                        next.main.position += Vec3::new(0.0, 100000.0, -10000000.0);
                    }
                    if move3 < 0.3 {
                        next.main.scale = Vec3::zero();
                    }
                }
            },
            Some(ToolKind::Dagger) => {
                let (move1base, move2base, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time.powf(0.25), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1abs = move1base * pullback;
                let move2abs = move2base * pullback;
                next.main.position = Vec3::new(0.0, 2.0, -5.0);
                next.control_l.position = Vec3::new(
                    0.0,
                    0.0 + move1abs * 1.0 + move2abs * -1.0,
                    -2.0 + move1abs * -2.0,
                );
                next.control_r.position = Vec3::new(
                    -1.0 + s_a.grip.0 * 2.0,
                    2.0 + move3 * -4.0,
                    -2.0 + move3 * -4.0,
                );

                next.control.position = Vec3::new(
                    -1.0,
                    0.0 + move1abs * -3.0 + move2abs * 3.0 + s_a.grip.2,
                    3.0 + move1abs * 3.0 - s_a.grip.2 / 2.5 + s_a.grip.0 * -2.0,
                );

                next.control_l.orientation =
                    Quaternion::rotation_x(PI / 2.0) * Quaternion::rotation_y(-0.3);
                next.control_r.orientation = Quaternion::rotation_x(PI / 2.0 + s_a.grip.0 * 0.2)
                    * Quaternion::rotation_y(0.5 + s_a.grip.0 * 0.2 + move3 * -1.5);

                next.control.orientation =
                    Quaternion::rotation_x(-0.3 + move1abs * 0.5 + move2abs * -0.5 + move3 * 0.5)
                        * Quaternion::rotation_y(0.5 * speednorm);

                if matches!(stage_section, Some(StageSection::Recover)) {
                    next.main.scale = Vec3::zero();
                }
            },
            _ => {},
        }

        next
    }
}
