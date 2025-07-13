use super::{
    super::{Animation, vek::*},
    QuadrupedMediumSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
use std::f32::consts::PI;

pub struct RapidMeleeAnimation;

impl Animation for RapidMeleeAnimation {
    type Dependency<'a> = (Option<&'a str>, Option<StageSection>);
    type Skeleton = QuadrupedMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_medium_rapidmelee\0";

    #[cfg_attr(
        feature = "be-dyn-lib",
        unsafe(export_name = "quadruped_medium_rapidmelee")
    )]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_id, stage_section): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        _s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (buildup, _action, recover) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.sqrt(), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time.min(1.0), 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(3)),
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - recover;
        let buildup = buildup * pullback;
        let buildup_overshoot = (1.5 - (2.5 * (buildup - 0.6)).abs()) * pullback;

        match ability_id {
            Some("common.abilities.custom.elephant.vacuum") => {
                next.head.orientation.rotate_x(PI / 6.0 * buildup);
                next.jaw.orientation.rotate_x(PI / 5.0 * buildup_overshoot);
                next.ears.position += Vec3::new(0.0, -4.0, -2.0) * buildup_overshoot;
                next.ears.orientation.rotate_x(PI / 4.0 * buildup_overshoot);
            },
            _ => {},
        }

        next
    }
}
