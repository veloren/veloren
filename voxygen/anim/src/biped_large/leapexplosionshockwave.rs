use super::{
    super::{Animation, vek::*},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
use core::f32::consts::PI;

pub struct LeapExplosionShockAnimation;

impl Animation for LeapExplosionShockAnimation {
    type Dependency<'a> = (Option<StageSection>, Option<&'a str>);
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_leapexplosionshockwave\0";

    #[cfg_attr(
        feature = "be-dyn-lib",
        unsafe(export_name = "biped_large_leapexplosionshockwave")
    )]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (stage_section, ability_id): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (move1, move2, move3, move4) = match stage_section {
            Some(StageSection::Buildup) => (anim_time, 0.0, 0.0, 0.0),
            Some(StageSection::Movement) => (1.0, anim_time.powf(0.25), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, 1.0, anim_time.powf(0.25), 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0, 0.0),
        };

        match ability_id {
            Some("common.abilities.custom.gigas_fire.lava_leap") => {
                let move1 = (PI * move1).sin();
                let move2 = move2 * (1.0 - move3);
                let move3 = move3 * (1.0 - move4.powi(3));

                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3)
                    * Quaternion::rotation_y(s_a.sc.4)
                    * Quaternion::rotation_z(s_a.sc.5);
                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position = Vec3::new(s_a.shr.0, s_a.shr.1, s_a.shr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.shr.3) * Quaternion::rotation_y(s_a.shr.4);
                next.main.position = Vec3::new(1.0, 10.0, 0.0);
                next.main.orientation = Quaternion::rotation_y(0.0) * Quaternion::rotation_z(0.0);

                next.torso.position += Vec3::new(0.0, -3.0, -3.0) * move1;
                next.leg_l.position += Vec3::new(0.0, 1.5, 1.5) * move1;
                next.leg_l.orientation.rotate_x(PI / 3.0 * move1);
                next.foot_l.position += Vec3::new(0.0, 3.0, 3.0) * move1;
                next.leg_r.position += Vec3::new(0.0, 1.5, 1.5) * move1;
                next.leg_r.orientation.rotate_x(PI / 3.0 * move1);
                next.foot_r.position += Vec3::new(0.0, 3.0, 3.0) * move1;

                next.torso.orientation.rotate_x(PI / 5.0 * move2);
                next.torso.position += Vec3::new(0.0, 5.0, 0.0) * move2;
                next.foot_l.orientation.rotate_x(-PI / 8.0 * move2);
                next.leg_r.position += Vec3::new(0.0, 5.0, 0.0) * move2;
                next.leg_r.orientation.rotate_x(PI / 8.0 * move2);
                next.foot_r.position += Vec3::new(0.0, 2.0, 0.0) * move2;
                next.foot_r.orientation.rotate_x(-PI / 6.0 * move2);
                next.shoulder_l.orientation.rotate_x(PI / 2.5 * move2);
                next.shoulder_r.position += Vec3::new(-3.0, 7.0, 0.0) * move2;
                next.shoulder_r.orientation.rotate_x(PI / 1.5 * move2);
                next.shoulder_r.orientation.rotate_z(PI / 4.0 * move2);
                next.control.position += Vec3::new(-8.0, 0.0, 15.0) * move2;
                next.control.orientation.rotate_x(PI / 3.0 * move2);
                next.control.orientation.rotate_z(-PI / 10.0 * move2);
                next.control_r.position += Vec3::new(13.0, 4.0, -8.0) * move2;
                next.control_r.orientation.rotate_x(PI / 8.0 * move2);
                next.control_r.orientation.rotate_z(PI / 3.0 * move2);
                next.control_l.position += Vec3::new(0.0, 0.0, -7.0) * move2;
                next.control_l.orientation.rotate_x(PI / 8.0 * move2);

                next.torso.position += Vec3::new(0.0, -9.0, 0.0) * move3;
                next.torso.orientation.rotate_x(-PI / 8.0 * move3);
                next.lower_torso.position += Vec3::new(0.0, 0.0, 1.0) * move3;
                next.lower_torso.orientation.rotate_x(PI / 8.0 * move3);
                next.shoulder_r.position += Vec3::new(-3.0, 6.0, 0.0) * move3;
                next.shoulder_r.orientation.rotate_z(PI / 4.0 * move3);
                next.shoulder_l.position += Vec3::new(3.0, 6.0, 0.0) * move3;
                next.shoulder_l.orientation.rotate_z(-PI / 4.0 * move3);
                next.control.position += Vec3::new(0.0, 0.0, 0.0) * move3;
                next.control.orientation.rotate_x(-PI / 2.5 * move3);
            },
            _ => {},
        }

        next
    }
}
