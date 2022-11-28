use super::{
    super::{vek::*, Animation},
    QuadrupedLowSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
//use std::ops::Rem;
use std::f32::consts::PI;

pub struct DashAnimation;

impl Animation for DashAnimation {
    type Dependency<'a> = (f32, f32, Option<StageSection>, f32);
    type Skeleton = QuadrupedLowSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_low_dash\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_low_dash")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, global_time, stage_section, timer): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        _s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (movement1base, chargemovementbase, movement2base, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.sqrt(), 0.0, 0.0, 0.0),
            Some(StageSection::Charge) => (1.0, 1.0, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, 1.0, anim_time.powi(4), 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - movement3;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let twitch1 = (mirror * movement1base * 9.5).sin();
        let twitch1fast = (mirror * movement1base * 25.0).sin();
        //let twitch3 = (mirror * movement3 * 4.0).sin();
        //let movement1 = mirror * movement1base * pullback;
        //let movement2 = mirror * movement2base * pullback;
        let movement1abs = movement1base * pullback;
        let movement2abs = movement2base * pullback;
        let short = ((1.0 / (0.72 + 0.28 * ((anim_time * 16.0_f32 + PI * 0.25).sin()).powi(2)))
            .sqrt())
            * ((anim_time * 16.0_f32 + PI * 0.25).sin())
            * chargemovementbase
            * pullback;
        let shortalt = (anim_time * 16.0_f32 + PI * 0.25).sin() * chargemovementbase * pullback;

        next.head_upper.orientation =
            Quaternion::rotation_x(movement1abs * 0.4 + movement2abs * 0.3)
                * Quaternion::rotation_z(short * -0.06 + twitch1 * -0.3);

        next.head_lower.orientation =
            Quaternion::rotation_x(movement1abs * -0.4 + movement2abs * -0.5)
                * Quaternion::rotation_z(short * 0.15 + twitch1 * 0.3);

        next.jaw.orientation = Quaternion::rotation_x(
            twitch1fast * 0.2
                + movement1abs * -0.3
                + movement2abs * 1.2
                + chargemovementbase * -0.5,
        );
        next.chest.orientation =
            Quaternion::rotation_z(twitch1 * 0.06) * Quaternion::rotation_y(short * 0.06);

        next.tail_front.orientation = Quaternion::rotation_x(
            0.15 + movement1abs * -0.4 + movement2abs * 0.2 + chargemovementbase * 0.2,
        ) * Quaternion::rotation_z(shortalt * 0.15);

        next.tail_rear.orientation =
            Quaternion::rotation_x(
                -0.12 + movement1abs * -0.4 + movement2abs * 0.2 + chargemovementbase * 0.2,
            ) * Quaternion::rotation_z(shortalt * 0.15 + twitch1fast * 0.3);

        next
    }
}
