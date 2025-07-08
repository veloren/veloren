use super::{
    super::{Animation, vek::*},
    BipedSmallSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};
use std::f32::consts::PI;

pub struct SummonAnimation;

type SummonAnimationDependency<'a> = (
    Option<&'a str>,
    Option<ToolKind>,
    Vec3<f32>,
    Vec3<f32>,
    Vec3<f32>,
    f32,
    Vec3<f32>,
    f32,
    Option<StageSection>,
    f32,
);

impl Animation for SummonAnimation {
    type Dependency<'a> = SummonAnimationDependency<'a>;
    type Skeleton = BipedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_small_summon\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "biped_small_summon"))]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            ability_id,
            active_tool_kind,
            _velocity,
            _orientation,
            _last_ori,
            global_time,
            _avg_vel,
            _acc_vel,
            stage_section,
            timer,
        ): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let anim_time = anim_time.min(1.0);
        let (move1base, twitch, twitch2, move2base, move3) = match stage_section {
            Some(StageSection::Buildup) => {
                (anim_time.sqrt(), (anim_time * 5.0).sin(), 0.0, 0.0, 0.0)
            },
            Some(StageSection::Action) => {
                (1.0, 1.0, (anim_time * 10.0).sin(), anim_time.powi(4), 0.0)
            },
            Some(StageSection::Recover) => (1.0, 1.0, 1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - move3;
        let twitch = twitch * pullback;
        let twitch2 = twitch2 * pullback;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let move1 = move1base * pullback * mirror;
        let move2 = move2base * pullback * mirror;
        let move1abs = move1base * pullback;
        let move2abs = move2base * pullback;
        next.hand_l.position = Vec3::new(s_a.grip.0 * 4.0, 0.0, s_a.grip.2);
        next.hand_r.position = Vec3::new(-s_a.grip.0 * 4.0, 0.0, s_a.grip.2);
        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_x(0.0);
        next.hand_l.orientation = Quaternion::rotation_x(0.0);
        next.hand_r.orientation = Quaternion::rotation_x(0.0);
        match active_tool_kind {
            Some(ToolKind::Staff) => match ability_id {
                Some("common.abilities.custom.dwarves.flamekeeper.summon_lavathrower") => {
                    next.control_l.position = Vec3::new(2.0 - s_a.grip.0 * 2.0, 3.0, 3.0);
                    next.control_r.position = Vec3::new(
                        12.0 + s_a.grip.0 * 2.0,
                        -4.0 + move1abs * -20.0,
                        3.0 + twitch * 2.0 + twitch2 * 2.0,
                    );

                    next.control.position = Vec3::new(
                        -5.0 + move1abs * -5.0,
                        -1.0 + s_a.grip.2 + move1abs * -8.0,
                        -2.0 + -s_a.grip.2 / 2.5 + s_a.grip.0 * -2.0 + twitch2 * -2.0,
                    );
                    next.chest.position =
                        Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + twitch2 + move2abs * 2.0);
                    next.control_l.orientation = Quaternion::rotation_x(PI / 2.0)
                        * Quaternion::rotation_y(-0.3)
                        * Quaternion::rotation_z(-0.3);
                    next.control_r.orientation = Quaternion::rotation_x(
                        PI / 2.0 + s_a.grip.0 * 0.2 + twitch * 0.2 + twitch2 * 0.2,
                    ) * Quaternion::rotation_y(
                        -0.4 + s_a.grip.0 * 0.2 + move1abs * -2.0,
                    ) * Quaternion::rotation_z(move1abs * 0.5);

                    next.control.orientation = Quaternion::rotation_x(-0.3)
                        * Quaternion::rotation_y(0.0)
                        * Quaternion::rotation_z(0.5 + move1abs * 1.0);
                    next.chest.orientation =
                        Quaternion::rotation_x(0.0) * Quaternion::rotation_z(0.0);
                    next.head.orientation =
                        Quaternion::rotation_z(twitch * 0.2 + twitch2 * 0.4 + move2abs * -0.5)
                            * Quaternion::rotation_x(move2abs * 0.7);
                },
                Some("common.abilities.custom.ashen_warrior.staff.flame_wall") => {
                    let slow = (global_time * 4.0).sin();
                    let (move1base, move2base, move3base) = match stage_section {
                        Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                        Some(StageSection::Action) => (1.0, anim_time, 0.0),
                        Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                        _ => (0.0, 0.0, 0.0),
                    };

                    let pullback = 1.0 - move3base;
                    let move1 = move1base * pullback;

                    next.control.position +=
                        Vec3::new(-8.0, 8.0, 10.0) * move1 + Vec3::new(0.5, 0.5, 2.0) * slow;
                    next.control.orientation.rotate_z(0.15 * slow);
                    next.control.orientation.rotate_x(-PI / 10.0 * move1);
                    next.control_l.position += Vec3::new(7.0, -2.0, 0.0) * move1;
                    next.control_l.orientation.rotate_x(PI / 1.3 * move1);
                    next.control_l.orientation.rotate_z(-PI / 4.0 * move1);
                    next.control_r.position += Vec3::new(4.0, -2.0, 0.0) * move1;
                    next.control_r.orientation.rotate_x(PI / 1.3 * move1);
                    next.control_r.orientation.rotate_z(PI / 2.0 * move1);

                    next.chest.orientation.rotate_z(2.0 * PI * move2base);
                    next.foot_l.position += Vec3::new(
                        (2.0 * PI * move2base + PI).cos() + 1.0,
                        (2.0 * PI * move2base + PI).sin(),
                        0.0,
                    ) * 3.0;
                    next.foot_l.orientation.rotate_z(2.0 * PI * move2base);
                    next.foot_r.position += Vec3::new(
                        (2.0 * PI * move2base).cos() - 1.0,
                        (2.0 * PI * move2base).sin(),
                        0.0,
                    ) * 3.0;
                    next.foot_r.orientation.rotate_z(2.0 * PI * move2base);
                },
                Some("common.abilities.custom.ashen_warrior.staff.summon_crux") => {
                    let slow = (global_time * 4.0).sin();
                    let (move1base, _, move3base) = match stage_section {
                        Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                        Some(StageSection::Action) => (1.0, anim_time, 0.0),
                        Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                        _ => (0.0, 0.0, 0.0),
                    };

                    let pullback = 1.0 - move3base;
                    let move1 = move1base * pullback;

                    next.main.position = Vec3::new(-10.0, 5.0, 0.0);
                    next.main.orientation = Quaternion::rotation_x(0.0);
                    next.hand_l.position = Vec3::new(s_a.grip.0 * 5.0, 0.0, 2.0);
                    next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0);
                    next.hand_r.position = Vec3::new(-s_a.grip.0 * 5.0, 0.0, 2.0);
                    next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0);

                    next.head.position += Vec3::new(0.0, -4.0, 0.0) * move1;
                    next.head.orientation.rotate_x(PI / 10.0 * move1);
                    next.chest.orientation.rotate_x(PI / 10.0 * move1);
                    next.hand_l.orientation.rotate_x(PI / 10.0 * move1);
                    next.hand_l.orientation.rotate_y(PI / 10.0 * move1);
                    next.hand_r.orientation.rotate_x(PI / 10.0 * move1);
                    next.hand_r.orientation.rotate_y(-PI / 10.0 * move1);
                    next.main.position += Vec3::new(0.0, 0.0, 2.0) * move1;
                    next.main.orientation.rotate_y(PI / 10.0 * move1);
                    next.control.position +=
                        Vec3::new(0.0, 4.0, 10.0) * move1 + Vec3::new(0.5, 0.5, 2.0) * slow;
                },
                _ => {
                    next.control_l.position = Vec3::new(2.0 - s_a.grip.0 * 2.0, 1.0, 3.0);
                    next.control_r.position = Vec3::new(
                        7.0 + s_a.grip.0 * 2.0,
                        -4.0 + move1abs * -14.0,
                        3.0 + twitch * 2.0 + twitch2 * 2.0,
                    );

                    next.control.position = Vec3::new(
                        -5.0 + move1abs * -5.0,
                        -1.0 + s_a.grip.2 + move1abs * -8.0,
                        -2.0 + -s_a.grip.2 / 2.5 + s_a.grip.0 * -2.0 + twitch2 * -2.0,
                    );
                    next.chest.position =
                        Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + twitch2 + move2abs * 3.0);
                    next.control_l.orientation = Quaternion::rotation_x(PI / 2.0)
                        * Quaternion::rotation_y(-0.3)
                        * Quaternion::rotation_z(-0.3);
                    next.control_r.orientation = Quaternion::rotation_x(
                        PI / 2.0 + s_a.grip.0 * 0.2 + twitch * 0.2 + twitch2 * 0.2,
                    ) * Quaternion::rotation_y(
                        -0.4 + s_a.grip.0 * 0.2 + move1abs * -2.0,
                    ) * Quaternion::rotation_z(move1abs * 0.5);

                    next.control.orientation = Quaternion::rotation_x(-0.3)
                        * Quaternion::rotation_y(0.0)
                        * Quaternion::rotation_z(0.5 + move1abs * 1.0);
                    next.chest.orientation =
                        Quaternion::rotation_x(0.0) * Quaternion::rotation_z(0.0);
                    next.head.orientation =
                        Quaternion::rotation_z(twitch * 0.2 + twitch2 * 0.4 + move2abs * -0.5)
                            * Quaternion::rotation_x(move2abs * 0.7);
                },
            },
            Some(ToolKind::Sword) => {
                next.control_l.position = Vec3::new(2.0 - s_a.grip.0 * 2.0, 3.0, 3.0);
                next.control_r.position =
                    Vec3::new(12.0 + s_a.grip.0 * 2.0, -4.0 + move1abs * -14.0, 3.0);

                next.control.position = Vec3::new(
                    -5.0 + move1abs * -5.0,
                    -1.0 + s_a.grip.2 + move1abs * -8.0,
                    s_a.grip.2 / 2.0,
                );
                next.chest.position =
                    Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + twitch2 + move2abs * 2.0);
                next.control_l.orientation = Quaternion::rotation_x(PI / 2.0)
                    * Quaternion::rotation_y(-0.3)
                    * Quaternion::rotation_z(-0.3);
                next.control_r.orientation = Quaternion::rotation_x(
                    PI / 2.0 + s_a.grip.0 * 0.2 + twitch * 0.2 + twitch2 * 0.2,
                ) * Quaternion::rotation_y(
                    -0.4 + s_a.grip.0 * 0.2 + move1abs * -2.0,
                ) * Quaternion::rotation_z(move1abs * 0.5);

                next.chest.position = Vec3::new(
                    0.0,
                    s_a.chest.0,
                    s_a.chest.1 + move1abs * 6.0 + twitch2 + move2abs * 3.0,
                );
                next.control_l.orientation = Quaternion::rotation_x(PI / 2.0)
                    * Quaternion::rotation_y(-0.3)
                    * Quaternion::rotation_z(-0.3);
                next.control_l.orientation = Quaternion::rotation_x(PI / 2.0)
                    * Quaternion::rotation_y(-0.3)
                    * Quaternion::rotation_z(0.3);
                next.control.orientation = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.5 + move1abs * 1.0);
                next.chest.orientation = Quaternion::rotation_x(0.0) * Quaternion::rotation_z(0.0);
                next.head.orientation =
                    Quaternion::rotation_z(twitch * 0.2 + twitch2 * 0.4 + move2abs * -0.5)
                        * Quaternion::rotation_x(move2abs * 0.4);
            },
            _ => {
                next.chest.orientation = Quaternion::rotation_x(move2abs * -1.0)
                    * Quaternion::rotation_z(move1 * 1.2 + move2 * -1.8);
                next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2);
                next.hand_l.orientation = Quaternion::rotation_x(1.2);
                next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2);
                next.hand_r.orientation = Quaternion::rotation_x(1.2);
            },
        }
        next
    }
}
