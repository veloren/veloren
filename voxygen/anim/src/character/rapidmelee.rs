use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::states::utils::{AbilityInfo, StageSection};
use core::f32::consts::{PI, TAU};
use std::ops::{Mul, Sub};

pub struct RapidMeleeAnimation;
impl Animation for RapidMeleeAnimation {
    type Dependency<'a> = (
        Option<&'a str>,
        Option<StageSection>,
        (u32, Option<u32>),
        Option<AbilityInfo>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_rapid_melee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_rapid_melee")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_id, stage_section, (current_strike, _max_strikes), _ability_info): Self::Dependency<
            '_,
        >,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_z(0.0);
        next.main_weapon_trail = true;
        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation = Quaternion::rotation_z(0.0);
        next.off_weapon_trail = true;

        match ability_id {
            Some(
                "common.abilities.sword.cleaving_whirlwind_slice"
                | "common.abilities.sword.cleaving_bladestorm",
            ) => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;

                let move2_no_pullback = move2 + current_strike as f32;
                let move2 = if current_strike == 0 { move2 } else { 1.0 };
                let move2_pre = move2.min(0.3) * 10.0 / 3.0;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;
                let move2_pre = move2_pre * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position = Vec3::new(-s_a.sc.0 + -6.0, -1.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(1.4);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation =
                    Quaternion::rotation_x(s_a.sc.3) * Quaternion::rotation_z(move1 * PI);

                if move2 < f32::EPSILON {
                    next.main_weapon_trail = false;
                    next.off_weapon_trail = false;
                }
                next.chest.orientation = Quaternion::rotation_z(move1 * 1.2);
                next.head.orientation = Quaternion::rotation_z(move1 * -0.7);
                next.belt.orientation = Quaternion::rotation_z(move1 * -0.3);
                next.shorts.orientation = Quaternion::rotation_z(move1 * -0.8);
                next.control.orientation.rotate_x(move1 * 0.2);
                next.foot_r
                    .orientation
                    .rotate_x(move1 * -0.4 + move2_pre * 0.4);
                next.foot_r.orientation.rotate_z(move1 * 1.4);

                next.control.orientation.rotate_y(move2_pre * -1.6);
                next.control.position += Vec3::new(0.0, 0.0, move2_pre * 4.0);
                next.torso.orientation.rotate_z(move2_no_pullback * TAU);
                next.chest.orientation.rotate_z(move2 * -2.0);
                next.head.orientation.rotate_z(move2 * 1.3);
                next.belt.orientation.rotate_z(move2 * 0.6);
                next.shorts.orientation.rotate_z(move2 * 1.5);
                next.foot_r.orientation.rotate_z(move2_pre * -1.7);
                next.control.orientation.rotate_z(move2 * -1.8);
                next.control.position += Vec3::new(move2 * 14.0, 0.0, 0.0);
            },
            Some(
                "common.abilities.sword.agile_perforate" | "common.abilities.sword.agile_flurry",
            ) => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                    Some(StageSection::Action) => (
                        1.0,
                        anim_time.min(0.5).mul(2.0).powi(2) - anim_time.max(0.5).sub(0.5).mul(2.0),
                        0.0,
                    ),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3);

                next.chest.orientation = Quaternion::rotation_z(move1 * 0.7);
                next.head.orientation = Quaternion::rotation_z(move1 * -0.4);
                next.shorts.orientation = Quaternion::rotation_z(move1 * -0.5);
                next.belt.orientation = Quaternion::rotation_z(move1 * -0.2);
                next.control.orientation.rotate_x(move1 * -1.1);
                next.control.orientation.rotate_z(move1 * -0.7);
                next.control.position += Vec3::new(move1 * 1.0, move1 * -1.0, move1 * 4.0);

                next.chest.orientation.rotate_z(move2 * -1.2);
                next.head.orientation.rotate_z(move2 * 0.6);
                next.belt.orientation.rotate_z(move2 * 0.3);
                next.shorts.orientation.rotate_z(move2 * 0.7);
                next.control.orientation.rotate_z(move2 * 1.2);
                next.control.position += Vec3::new(0.0, move2 * 12.0, 0.0);
            },
            Some("common.abilities.sword.agile_hundred_cuts") => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time.powf(0.25), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;
                let (move2a, move2b, move2c, move2d) = match current_strike % 4 {
                    0 => (move2, 0.0, 0.0, 0.0),
                    1 => (1.0, move2, 0.0, 0.0),
                    2 => (1.0, 1.0, move2, 0.0),
                    3 => (1.0, 1.0, 1.0, move2),
                    _ => (0.0, 0.0, 0.0, 0.0),
                };

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3);

                next.chest.orientation = Quaternion::rotation_z(move1 * 0.4);
                next.head.orientation = Quaternion::rotation_z(move1 * -0.2);
                next.shorts.orientation = Quaternion::rotation_z(move1 * -0.2);
                next.belt.orientation = Quaternion::rotation_z(move1 * -0.1);
                next.control.orientation.rotate_y(move1 * -1.2);
                next.control.position += Vec3::new(0.0, 0.0, move1 * 10.0);

                next.chest.orientation.rotate_z(move2a * -0.2);
                next.control.orientation.rotate_z(move2a * -2.0);
                next.control.position += Vec3::new(move2a * 12.0, 0.0, move2a * -6.0);

                next.chest.orientation.rotate_z(move2b * 0.2);
                next.control.orientation.rotate_z(move2b * 2.9);
                next.control.position += Vec3::new(move2b * -12.0, 0.0, 0.0);

                next.chest.orientation.rotate_z(move2c * -0.2);
                next.control.orientation.rotate_z(move2c * -2.3);
                next.control.position += Vec3::new(move2c * 12.0, 0.0, move2c * 12.0);

                next.chest.orientation.rotate_z(move2d * -0.2);
                next.control.orientation.rotate_z(move2d * -2.7);
                next.control.position += Vec3::new(move2d * 12.0, 0.0, move2a * -6.0);
            },

            Some("common.abilities.sword.crippling_mutilate") => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                    Some(StageSection::Action) => (
                        1.0,
                        if current_strike % 2 == 0 {
                            anim_time
                        } else {
                            1.0 - anim_time
                        },
                        0.0,
                    ),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3);

                next.chest.orientation = Quaternion::rotation_z(move1 * 0.7);
                next.head.orientation = Quaternion::rotation_z(move1 * -0.4);
                next.shorts.orientation = Quaternion::rotation_z(move1 * -0.5);
                next.belt.orientation = Quaternion::rotation_z(move1 * -0.2);
                next.control.orientation.rotate_x(move1 * -0.6);
                next.control.orientation.rotate_z(move1 * -0.7);
                next.control.position += Vec3::new(move1 * 1.0, move1 * -4.0, move1 * 4.0);

                next.chest.orientation.rotate_z(move2 * -1.2);
                next.head.orientation.rotate_z(move2 * 0.6);
                next.belt.orientation.rotate_z(move2 * 0.3);
                next.shorts.orientation.rotate_z(move2 * 0.7);
                next.control.orientation.rotate_z(move2 * 1.2);
                next.control.position += Vec3::new(0.0, move2 * 14.0, move2 * 12.0);
            },
            _ => {},
        }

        next
    }
}
