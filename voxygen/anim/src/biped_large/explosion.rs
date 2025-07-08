use super::{
    super::{Animation, vek::*},
    BipedLargeSkeleton, SkeletonAttr, init_biped_large_alpha, init_gigas_fire,
};
use common::states::utils::StageSection;
use core::f32::consts::PI;

pub struct ExplosionAnimation;

impl Animation for ExplosionAnimation {
    type Dependency<'a> = (Vec3<f32>, f32, Option<StageSection>, Option<&'a str>);
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_explosion\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "biped_large_explosion"))]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, acc_vel, stage_section, ability_id): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (move1base, move2base, move3base) = match stage_section {
            Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - move3base;

        let speed = Vec2::<f32>::from(velocity).magnitude();

        match ability_id {
            Some("common.abilities.custom.gigas_fire.explosive_strike") => {
                let (pre_move1base, move1base) = if move1base < 0.2 {
                    (5.0 * move1base, 0.0)
                } else {
                    (1.0, 1.25 * (move1base - 0.2))
                };

                let pre_move1 = pre_move1base * pullback;
                let move2 = move2base * pullback;

                init_biped_large_alpha(&mut next, s_a, speed, acc_vel, pre_move1);
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
            _ => {},
        }

        next
    }
}
