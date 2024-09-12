use std::f32::consts::PI;

use super::{
    super::{vek::*, Animation},
    QuadrupedLowSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct TailwhipAnimation;

impl Animation for TailwhipAnimation {
    type Dependency<'a> = (Option<&'a str>, f32, f32, Option<StageSection>, f32);
    type Skeleton = QuadrupedLowSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_low_tailwhip\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_low_tailwhip")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_id, _velocity, global_time, stage_section, timer): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        match ability_id {
            Some("common.abilities.custom.hydra.tail_swipe") => match stage_section {
                Some(StageSection::Charge) => {
                    let rotate_t = mirror * anim_time * 2.0 / (1.0 + anim_time * 2.0);
                    let shake = (anim_time * 15.0).sin() * 0.05;

                    next.chest.orientation = Quaternion::rotation_z(0.2 * rotate_t + shake * 0.1);

                    next.tail_front.orientation = Quaternion::rotation_z(0.7 * rotate_t + shake);

                    next.tail_rear.orientation = Quaternion::rotation_z(0.8 * rotate_t + shake);

                    next.foot_fl.orientation = Quaternion::rotation_x(-0.15 * rotate_t);
                    next.foot_bl.orientation = Quaternion::rotation_x(-0.25 * rotate_t);
                    next.foot_fr.orientation = Quaternion::rotation_x(0.15 * rotate_t);
                    next.foot_br.orientation = Quaternion::rotation_x(0.25 * rotate_t);
                },
                Some(StageSection::Action) => {
                    let rotate_t = mirror * -anim_time * 30.0 / (4.0 + anim_time * 30.0) * PI * 2.3;

                    next.chest.orientation = Quaternion::rotation_z(rotate_t);
                },
                Some(StageSection::Recover) => {},
                _ => {},
            },
            _ => {
                let (movement1base, movement2base, movement3, twitch1, twitch2) =
                    match stage_section {
                        Some(StageSection::Charge) => {
                            (anim_time.min(1.2), 0.0, 0.0, (anim_time * 15.0).sin(), 0.0)
                        },
                        Some(StageSection::Action) => (1.0, anim_time.powi(4), 0.0, 1.0, 0.0),
                        Some(StageSection::Recover) => {
                            (1.0, 1.0, anim_time.powi(6), 1.0, (anim_time * 7.0).sin())
                        },
                        _ => (0.0, 0.0, 0.0, 0.0, 0.0),
                    };
                let pullback = 1.0 - movement3;
                let movement1 = mirror * movement1base * pullback;
                let movement2 = mirror * movement2base * pullback;
                let movement1abs = movement1base * pullback;
                let movement1nopull = mirror * movement1base;
                let movement2nopull = mirror * movement2base;

                // Center head
                next.head_c_upper.orientation =
                    Quaternion::rotation_z(movement1 * 0.6 + movement2 * -1.2);

                next.head_c_lower.orientation =
                    Quaternion::rotation_z(movement1 * 0.7 + movement2 * -1.6);

                next.jaw_c.orientation = Quaternion::rotation_x(movement1 * -0.1 + movement2 * 0.1);

                // Left head
                next.head_l_upper.orientation =
                    Quaternion::rotation_z(movement1 * 0.6 + movement2 * -1.2);

                next.head_l_lower.orientation =
                    Quaternion::rotation_z(movement1 * 0.7 + movement2 * -1.6);

                next.jaw_l.orientation = Quaternion::rotation_x(movement1 * -0.1 + movement2 * 0.1);

                // Right head
                next.head_r_upper.orientation =
                    Quaternion::rotation_z(movement1 * 0.6 + movement2 * -1.2);

                next.head_r_lower.orientation =
                    Quaternion::rotation_z(movement1 * 0.7 + movement2 * -1.6);

                next.jaw_r.orientation = Quaternion::rotation_x(movement1 * -0.1 + movement2 * 0.1);

                next.chest.orientation = Quaternion::rotation_z(
                    (mirror * twitch1 * 0.02 + movement1nopull * -0.4 + movement2nopull * 3.0)
                        + (movement3 * 4.0 * mirror)
                        + twitch2 * 0.1 * mirror,
                );

                next.tail_front.orientation = Quaternion::rotation_x(0.15 + (movement1abs * -0.4))
                    * Quaternion::rotation_z(
                        mirror * twitch1 * 0.15
                            + movement1 * -0.6
                            + movement2 * 0.9
                            + twitch2 * 0.3 * mirror,
                    );
                next.tail_rear.position = Vec3::new(0.0, s_a.tail_rear.0, s_a.tail_rear.1);

                next.tail_rear.orientation = Quaternion::rotation_x(-0.12 + (movement1abs * -0.45))
                    * Quaternion::rotation_z(
                        mirror * twitch1 * 0.2
                            + movement1 * -0.6
                            + movement2 * 0.7
                            + twitch2 * 0.3 * mirror,
                    );
            },
        }

        next
    }
}
