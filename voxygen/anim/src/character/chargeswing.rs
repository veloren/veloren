use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
use core::f32::consts::{PI, TAU};

pub struct ChargeswingAnimation;

type ChargeswingAnimationDependency<'a> = (Option<&'a str>, StageSection);

impl Animation for ChargeswingAnimation {
    type Dependency<'a> = ChargeswingAnimationDependency<'a>;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_chargeswing\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_chargeswing")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_id, stage_section): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_z(0.0);
        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation = Quaternion::rotation_z(0.0);
        if matches!(stage_section, StageSection::Action) {
            next.main_weapon_trail = true;
            next.off_weapon_trail = true;
        }

        match ability_id {
            Some(
                "common.abilities.sword.basic_thrust"
                | "common.abilities.sword.defensive_vital_jab",
            ) => {
                let (move1, move2, move3, tension) = match stage_section {
                    StageSection::Charge => (
                        anim_time.powf(0.25).min(1.0),
                        0.0,
                        0.0,
                        (anim_time * 20.0).sin() - 0.5,
                    ),
                    StageSection::Action => (1.0, anim_time.powi(2), 0.0, 0.0),
                    StageSection::Recover => (1.0, 1.0, anim_time.powi(4), 0.0),
                    _ => (0.0, 0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(PI * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2 + move2 * 5.0);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3 + move1 * -0.9)
                    * Quaternion::rotation_y(move1 * 1.0 + move2 * -1.0)
                    * Quaternion::rotation_z(move1 * 1.3 + move2 * -1.3);

                next.chest.orientation =
                    Quaternion::rotation_z(move1 * 1.0 + tension * 0.02 + move2 * -1.2);
                next.head.orientation = Quaternion::rotation_z(move1 * -0.4 + move2 * 0.3);
                next.belt.orientation = Quaternion::rotation_z(move1 * -0.25 + move2 * 0.2);
                next.shorts.orientation = Quaternion::rotation_z(move1 * -0.5 + move2 * 0.4);
            },
            Some("common.abilities.sword.heavy_slam") => {
                let (move1, move2, move3, tension) = match stage_section {
                    StageSection::Charge => (
                        anim_time.powf(0.25).min(1.0),
                        0.0,
                        0.0,
                        (anim_time * 20.0).sin(),
                    ),
                    StageSection::Action => (1.0, anim_time.powi(2), 0.0, 0.0),
                    StageSection::Recover => (1.0, 1.0, anim_time.powi(4), 0.0),
                    _ => (0.0, 0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3)
                    * Quaternion::rotation_z(move1 * 0.3 + move2 * -0.7);

                next.control
                    .orientation
                    .rotate_x(move1 * 1.4 + tension / 50.0);
                next.control.position +=
                    Vec3::new(move1 * -1.0, move1 * 2.0, move1 * 8.0) + Vec3::one() * tension / 4.0;
                next.chest.orientation = Quaternion::rotation_z(move1 * 0.4 + tension / 50.0);

                if move2 < f32::EPSILON {
                    next.main_weapon_trail = false;
                    next.off_weapon_trail = false;
                }
                next.control.orientation.rotate_x(move2 * -3.0);
                next.control.orientation.rotate_z(move2 * -0.4);
                next.control.position += Vec3::new(move2 * 10.0, 0.0, move2 * -10.0);
                next.chest.orientation.rotate_z(move2 * -0.6);
            },
            Some("common.abilities.sword.crippling_deep_rend") => {
                let (move1, move2, tension, move3, move4) = match stage_section {
                    StageSection::Buildup => (anim_time, 0.0, 0.0, 0.0, 0.0),
                    StageSection::Charge => {
                        (1.0, anim_time.min(1.0), (anim_time * 20.0).sin(), 0.0, 0.0)
                    },
                    StageSection::Action => (1.0, 1.0, 0.0, anim_time, 0.0),
                    StageSection::Recover => (1.0, 1.0, 0.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0, 0.0, 0.0),
                };
                let move1pre = move1.min(0.5) * 2.0;
                let move1post = move1.max(0.5) * 2.0 - 1.0;
                let pullback = 1.0 - move4;
                let move1pre = move1pre * pullback;
                let move1post = move1post * pullback;
                let move2 = move2 * pullback;
                let move3 = move3 * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1pre * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation =
                    Quaternion::rotation_x(s_a.sc.3) * Quaternion::rotation_z(move1pre * PI / 2.0);

                next.foot_r.position += Vec3::new(0.0, move1pre * -3.0, 0.0);
                next.foot_r.orientation.rotate_z(move1pre * -1.2);
                next.chest.orientation = Quaternion::rotation_z(move1pre * -1.3);
                next.head.orientation = Quaternion::rotation_z(move1pre * 0.7);
                next.belt.orientation = Quaternion::rotation_z(move1pre * 0.4);
                next.shorts.orientation = Quaternion::rotation_z(move1pre * 0.8);
                next.control.orientation.rotate_y(move1pre * -1.5);
                next.control.orientation.rotate_z(move1pre * 0.0);
                next.control.position += Vec3::new(move1pre * 12.0, 0.0, 0.0);

                next.chest.orientation.rotate_z(move1post * 1.2);
                next.head.orientation.rotate_z(move1post * -0.7);
                next.belt.orientation.rotate_z(move1post * -0.3);
                next.shorts.orientation.rotate_z(move1post * -0.8);
                next.foot_r.orientation.rotate_z(move1post * 1.2);
                next.foot_r.orientation.rotate_x(move1post * -0.6);
                next.control.orientation.rotate_z(move1post * -1.2);
                next.control.position += Vec3::new(0.0, move1post * 4.0, move1post * 3.0);

                next.control
                    .orientation
                    .rotate_y(move2 * -2.0 + tension / 10.0);
                next.chest.orientation.rotate_z(move2 * -0.4 + move3 * -1.4);
                next.control
                    .orientation
                    .rotate_z(move2 * 0.3 + move3 * -1.2);
                next.head.orientation.rotate_z(move2 * 0.2 + move3 * 0.7);
                next.belt.orientation.rotate_z(move3 * 0.3);
                next.shorts.orientation.rotate_z(move2 * 0.2 + move3 * 0.7);
                next.chest
                    .orientation
                    .rotate_y(move2 * -0.3 - tension / 100.0);
                next.foot_r.orientation.rotate_z(move3 * -1.5);
            },
            Some(
                "common.abilities.sword.cleaving_spiral_slash"
                | "common.abilities.sword.cleaving_dual_spiral_slash",
            ) => {
                let (move1, tension, move2, move3) = match stage_section {
                    StageSection::Charge => (
                        anim_time.powf(0.25).min(1.0),
                        (anim_time * 15.0).sin(),
                        0.0,
                        0.0,
                    ),
                    StageSection::Action => (1.0, 0.0, anim_time, 0.0),
                    StageSection::Recover => (1.0, 0.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;

                let move2_no_pullback = move2;
                let move2_pre = move2.min(0.3) * 10.0 / 3.0;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;
                let move2_pre = move2_pre * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3);

                next.chest.orientation = Quaternion::rotation_z(move1 * 1.2 + tension / 50.0);
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
                next.torso.orientation.rotate_z(move2_no_pullback * -TAU);
                next.chest.orientation.rotate_z(move2 * -2.0);
                next.head.orientation.rotate_z(move2 * 1.3);
                next.belt.orientation.rotate_z(move2 * 0.6);
                next.shorts.orientation.rotate_z(move2 * 1.5);
                next.foot_r.orientation.rotate_z(move2_pre * -1.7);
                next.control.orientation.rotate_z(move2 * -1.8);
                next.control.position += Vec3::new(move2 * 14.0, 0.0, 0.0);
            },
            Some("common.abilities.axe.cleave") => {
                let (move1, move2, move3, tension) = match stage_section {
                    StageSection::Charge => {
                        (anim_time.min(1.0), 0.0, 0.0, (anim_time * 20.0).sin())
                    },
                    StageSection::Action => (1.0, anim_time.powi(2), 0.0, 0.0),
                    StageSection::Recover => (1.0, 1.0, anim_time, 0.0),
                    _ => (0.0, 0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;

                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(
                    s_a.ac.0 + move1 * 7.0,
                    s_a.ac.1 + move1 * -4.0,
                    s_a.ac.2 + move1 * 18.0 + tension / 5.0,
                );
                next.control.orientation =
                    Quaternion::rotation_x(s_a.ac.3 + move1 * -1.0 + tension / 30.0)
                        * Quaternion::rotation_y(s_a.ac.4)
                        * Quaternion::rotation_z(s_a.ac.5 - move1 * PI);

                next.control.orientation.rotate_x(move2 * -3.0);
                next.control.position += Vec3::new(0.0, move2 * 8.0, move2 * -30.0);
            },
            _ => {},
        }

        next
    }
}
