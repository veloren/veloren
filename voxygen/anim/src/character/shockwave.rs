use super::{
    super::{vek::*, Animation},
    hammer_start, twist_back, twist_forward, CharacterSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
use std::f32::consts::PI;

pub struct Input {
    pub attack: bool,
}
pub struct ShockwaveAnimation;

impl Animation for ShockwaveAnimation {
    type Dependency<'a> = (Option<&'a str>, f32, f32, Option<StageSection>);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_shockwave\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_shockwave")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_id, _global_time, velocity, stage_section): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        if matches!(stage_section, Some(StageSection::Action)) {
            next.main_weapon_trail = true;
            next.off_weapon_trail = true;
        }

        let (move1, move2, move3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0),
        };

        match ability_id {
            Some(
                "common.abilities.staff.fireshockwave"
                | "common.abilities.sceptre.healingaura"
                | "common.abilities.sceptre.wardingaura",
            ) => {
                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

                next.hand_l.position = Vec3::new(s_a.sthl.0, s_a.sthl.1, s_a.sthl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.sthl.3) * Quaternion::rotation_y(s_a.sthl.4);
                next.hand_r.position = Vec3::new(s_a.sthr.0, s_a.sthr.1, s_a.sthr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.sthr.3) * Quaternion::rotation_y(s_a.sthr.4);
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);

                next.control.position = Vec3::new(s_a.stc.0, s_a.stc.1, s_a.stc.2);
                next.control.orientation =
                    Quaternion::rotation_x(s_a.stc.3) * Quaternion::rotation_y(s_a.stc.4);

                let twist = move1 * 0.8;

                next.control.position = Vec3::new(
                    s_a.stc.0 + (move1 * 5.0) * (1.0 - move3),
                    s_a.stc.1 + (move1 * 5.0) * (1.0 - move3),
                    s_a.stc.2 + (move1 * 10.0 + move2 * -10.0) * (1.0 - move3),
                );
                next.control.orientation =
                    Quaternion::rotation_x(s_a.stc.3 + (move1 * 0.8) * (1.0 - move3))
                        * Quaternion::rotation_y(
                            s_a.stc.4 + (move1 * -0.15 + move2 * -0.15) * (1.0 - move3),
                        )
                        * Quaternion::rotation_z((move1 * 0.8 + move2 * -0.8) * (1.0 - move3));

                next.head.orientation = Quaternion::rotation_x((move1 * 0.4) * (1.0 - move3))
                    * Quaternion::rotation_z((twist * 0.2 + move2 * -0.8) * (1.0 - move3));

                next.chest.position = Vec3::new(
                    0.0,
                    s_a.chest.0,
                    s_a.chest.1 + (move1 * 2.0 + move2 * -4.0) * (1.0 - move3),
                );
                next.chest.orientation = Quaternion::rotation_x((move2 * -0.8) * (1.0 - move3))
                    * Quaternion::rotation_z(twist * -0.2 + move2 * -0.1 + (1.0 - move3));

                next.belt.orientation = Quaternion::rotation_x((move2 * 0.2) * (1.0 - move3))
                    * Quaternion::rotation_z((twist * 0.6 + move2 * -0.48) * (1.0 - move3));

                next.shorts.orientation = Quaternion::rotation_x((move2 * 0.3) * (1.0 - move3))
                    * Quaternion::rotation_z((twist + move2 * -0.8) * (1.0 - move3));

                if velocity < 0.5 {
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
                }
            },
            Some("common.abilities.hammer.tremor") => {
                hammer_start(&mut next, s_a);
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;

                twist_back(&mut next, move1, 1.4, 0.7, 0.5, 0.9);
                next.foot_l.orientation.rotate_z(move1 * 1.4);
                next.foot_l.position += Vec3::new(-1.0, -3.0, 0.0) * move1;
                next.control.orientation.rotate_x(move1 * 2.6);
                next.control.orientation.rotate_y(move1 * 0.8);

                twist_forward(&mut next, move2, 2.1, 1.2, 0.9, 1.6);
                next.foot_l.orientation.rotate_z(move2 * -1.4);
                next.foot_l.position += Vec3::new(2.0, 7.0, 0.0) * move2;
                next.control.orientation.rotate_z(move2 * 2.1);
                next.control.orientation.rotate_x(move2 * -2.0);
                next.control.orientation.rotate_z(move2 * 1.2);
                next.control.position += Vec3::new(-16.0, 0.0, 0.0) * move2;
                next.chest.orientation.rotate_x(-0.8 * move2);
            },
            Some("common.abilities.hammer.rampart") => {
                hammer_start(&mut next, s_a);
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;

                next.control.orientation.rotate_x(move1 * 0.6);
                next.control.orientation.rotate_y(move1 * -PI / 2.0);
                next.hand_l.orientation.rotate_y(move1 * -PI);
                next.hand_r.orientation.rotate_y(move1 * -PI);
                next.control.position += Vec3::new(-5.0, 0.0, 30.0) * move1;

                next.control.position += Vec3::new(0.0, 0.0, -10.0) * move2;
                next.torso.orientation.rotate_x(move2 * -0.6);
                next.control.orientation.rotate_x(move2 * 0.6);
            },
            Some("common.abilities.hammer.seismic_shock") => {
                hammer_start(&mut next, s_a);
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;

                next.control.orientation.rotate_x(move1 * 2.5);
                next.control.position += Vec3::new(0.0, 0.0, 28.0) * move1;
                next.head.orientation.rotate_x(move1 * 0.3);
                next.chest.orientation.rotate_x(move1 * 0.3);
                next.belt.orientation.rotate_x(move1 * -0.2);
                next.shorts.orientation.rotate_x(move1 * -0.3);

                next.control.orientation.rotate_z(move2 * 2.0);
                next.control.orientation.rotate_x(move2 * -4.0);
                next.control.position += Vec3::new(-6.0, 0.0, -30.0) * move2;
                next.head.orientation.rotate_x(move2 * -0.9);
                next.chest.orientation.rotate_x(move2 * -0.5);
                next.belt.orientation.rotate_x(move2 * 0.2);
                next.shorts.orientation.rotate_x(move2 * 0.4);
            },
            _ => {},
        }

        next
    }
}
