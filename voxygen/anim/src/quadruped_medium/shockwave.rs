use super::{
    super::{Animation, vek::*},
    QuadrupedMediumSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
//use std::ops::Rem;
use std::f32::consts::PI;

pub struct ShockwaveAnimation;

impl Animation for ShockwaveAnimation {
    type Dependency<'a> = (Option<&'a str>, f32, f32, Option<StageSection>, f32);
    type Skeleton = QuadrupedMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_medium_shockwave\0";

    #[cfg_attr(
        feature = "be-dyn-lib",
        unsafe(export_name = "quadruped_medium_shockwave")
    )]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_id, _velocity, global_time, stage_section, timer): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        _s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (buildup_base, charge_base, action_base, recover_base, legtell) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.sqrt(), 0.0, 0.0, 0.0, anim_time),
            Some(StageSection::Charge) => (1.0, 1.0, 0.0, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, 1.0, anim_time.powi(4), 0.0, 0.2),
            Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time, 0.0),
            _ => (0.0, 0.0, 0.0, 0.0, 0.0),
        };

        match ability_id {
            Some("common.abilities.custom.elephant.stomp") => {
                let buildup = buildup_base;
                let action = action_base;
                let action_inv = 1.0 - action_base;
                let recover_inv = 1.0 - recover_base;

                next.foot_br
                    .orientation
                    .rotate_x(-PI / 8.0 * buildup * action_inv);
                next.foot_bl
                    .orientation
                    .rotate_x(-PI / 8.0 * buildup * action_inv);
                next.leg_fr
                    .orientation
                    .rotate_x(PI / 8.0 * buildup * recover_inv);
                next.leg_fr
                    .orientation
                    .rotate_y(-PI / 12.0 * buildup * action_inv);
                next.foot_fr
                    .orientation
                    .rotate_x(PI / 4.0 * buildup * action_inv);
                next.leg_fl
                    .orientation
                    .rotate_x(PI / 8.0 * buildup * recover_inv);
                next.leg_fl
                    .orientation
                    .rotate_y(PI / 12.0 * buildup * action_inv);
                next.foot_fl
                    .orientation
                    .rotate_x(PI / 4.0 * buildup * action_inv);
                next.torso_back
                    .orientation
                    .rotate_x(-PI / 8.0 * buildup * action_inv + PI / 8.0 * action * recover_inv);
                next.torso_front.position += Vec3::new(0.0, -9.0, 11.0) * buildup * action_inv
                    + Vec3::new(0.0, 0.0, -4.0) * action * recover_inv;
                next.torso_front
                    .orientation
                    .rotate_x(PI / 4.0 * buildup * action_inv - PI / 8.0 * action * recover_inv);
                next.head
                    .orientation
                    .rotate_x(PI / 8.0 * buildup * action_inv - PI / 8.0 * action * recover_inv);
                next.jaw
                    .orientation
                    .rotate_x(PI / 6.0 * buildup * action_inv);
                next.ears.position += Vec3::new(0.0, -5.0, -2.0) * buildup * action_inv;
                next.ears
                    .orientation
                    .rotate_x(PI / 4.0 * buildup * recover_inv);
            },
            _ => {
                let pullback = 1.0 - recover_base;
                let subtract = global_time - timer;
                let check = subtract - subtract.trunc();
                let mirror = (check - 0.5).signum();
                let twitch1 = (mirror * buildup_base * 3.5).sin();
                let twitch1fast = (mirror * buildup_base * 45.0).sin();
                //let twitch3 = (mirror * movement3 * 4.0).sin();
                //let movement1 = mirror * movement1base * pullback;
                //let movement2 = mirror * movement2base * pullback;
                let movement1abs = buildup_base * pullback;
                let movement2abs = action_base * pullback;
                //let legtwitch = (legtell * 6.0).sin() * pullback;
                let legswing = legtell * pullback;
                let short = ((1.0
                    / (0.72 + 0.28 * ((anim_time * 16.0_f32 + PI * 0.25).sin()).powi(2)))
                .sqrt())
                    * ((anim_time * 16.0_f32 + PI * 0.25).sin())
                    * charge_base
                    * pullback;
                let shortalt = (anim_time * 16.0_f32 + PI * 0.25).sin() * charge_base * pullback;

                next.head.orientation =
                    Quaternion::rotation_x(movement1abs * -0.2 + movement2abs * 0.8)
                        * Quaternion::rotation_z(short * -0.06 + twitch1 * 0.2);

                next.neck.orientation =
                    Quaternion::rotation_x(movement1abs * -0.2 + movement2abs * 0.5)
                        * Quaternion::rotation_z(short * 0.15 + twitch1 * 0.2);

                next.jaw.orientation = Quaternion::rotation_x(
                    twitch1fast * 0.2
                        + movement1abs * -0.3
                        + movement2abs * 1.2
                        + charge_base * -0.5,
                );
                next.torso_front.orientation = Quaternion::rotation_z(twitch1 * 0.06)
                    * Quaternion::rotation_y(short * 0.06)
                    * Quaternion::rotation_x(legswing * 0.06);

                next.tail.orientation = Quaternion::rotation_x(
                    0.15 + movement1abs * -0.4 + movement2abs * 0.2 + charge_base * 0.2,
                ) * Quaternion::rotation_z(shortalt * 0.15);
                next.leg_fl.orientation = Quaternion::rotation_x(legswing * 1.4);

                next.foot_fl.orientation = Quaternion::rotation_x(legswing * -0.5);
                next.leg_bl.orientation = Quaternion::rotation_x(legswing * 0.3);

                next.foot_bl.orientation = Quaternion::rotation_x(legswing * -0.3);

                next.leg_fr.orientation = Quaternion::rotation_x(legswing * 1.4);

                next.foot_fr.orientation = Quaternion::rotation_x(legswing * -0.5);

                next.leg_br.orientation = Quaternion::rotation_x(legswing * 0.3);

                next.foot_br.orientation = Quaternion::rotation_x(legswing * -0.3);
            },
        }
        next
    }
}
