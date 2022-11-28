use super::{
    super::{vek::*, Animation},
    QuadrupedLowSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
//use std::ops::Rem;
use std::f32::consts::PI;

pub struct BreatheAnimation;

impl Animation for BreatheAnimation {
    type Dependency<'a> = (f32, f32, Option<StageSection>, f32);
    type Skeleton = QuadrupedLowSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_low_breathe\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_low_breathe")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, global_time, stage_section, timer): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        _s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = (Vec2::<f32>::from(velocity).magnitude()).min(24.0);

        let (movement1base, _movement2base, movement3, twitch) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.sqrt(), 0.0, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time.min(1.0), 0.0, anim_time),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time, 1.0),
            _ => (0.0, 0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - movement3;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let twitch2 = mirror * (twitch * 20.0).sin() * pullback;
        let twitch2alt = mirror * (twitch * 20.0 + PI / 2.0).sin() * pullback;

        let movement1abs = movement1base * pullback;

        next.head_upper.orientation =
            Quaternion::rotation_x(movement1abs * 0.3 + twitch2alt * 0.02);

        next.head_lower.orientation =
            Quaternion::rotation_x(movement1abs * -0.3) * Quaternion::rotation_y(twitch2 * 0.02);

        next.jaw.orientation = Quaternion::rotation_x(movement1abs * -0.7 + twitch2 * 0.1);
        next.chest.orientation =
            Quaternion::rotation_y(twitch2 * -0.02) * Quaternion::rotation_z(0.0);

        next.tail_front.orientation =
            Quaternion::rotation_x(0.15 + movement1abs * -0.15 + twitch2alt * 0.02)
                * Quaternion::rotation_z(0.0);

        next.tail_rear.orientation =
            Quaternion::rotation_x(-0.12 + movement1abs * -0.2 + twitch2alt * 0.08)
                * Quaternion::rotation_z(0.0);
        if speed < 0.5 {
            next.foot_fl.orientation = Quaternion::rotation_y(twitch2 * 0.02);

            next.foot_fr.orientation = Quaternion::rotation_y(twitch2 * 0.02);

            next.foot_bl.orientation = Quaternion::rotation_y(twitch2 * 0.02);

            next.foot_br.orientation = Quaternion::rotation_y(twitch2 * 0.02);
        } else {
        };
        next
    }
}
