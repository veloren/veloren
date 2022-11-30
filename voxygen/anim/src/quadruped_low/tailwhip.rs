use super::{
    super::{vek::*, Animation},
    QuadrupedLowSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct TailwhipAnimation;

impl Animation for TailwhipAnimation {
    type Dependency<'a> = (f32, f32, Option<StageSection>, f32);
    type Skeleton = QuadrupedLowSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_low_tailwhip\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_low_tailwhip")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, global_time, stage_section, timer): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (movement1base, movement2base, movement3, twitch1, twitch2) = match stage_section {
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
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let movement1 = mirror * movement1base * pullback;
        let movement2 = mirror * movement2base * pullback;
        let movement1abs = movement1base * pullback;
        let movement1nopull = mirror * movement1base;
        let movement2nopull = mirror * movement2base;
        next.head_upper.orientation = Quaternion::rotation_z(movement1 * 0.6 + movement2 * -1.2);

        next.head_lower.orientation = Quaternion::rotation_z(movement1 * 0.7 + movement2 * -1.6);

        next.chest.orientation = Quaternion::rotation_z(
            (mirror * twitch1 * 0.02 + movement1nopull * -0.4 + movement2nopull * 3.0)
                + (movement3 * 4.0 * mirror)
                + twitch2 * 0.1 * mirror,
        );

        next.jaw.orientation = Quaternion::rotation_x(movement1 * -0.1 + movement2 * 0.1);

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
        next
    }
}
