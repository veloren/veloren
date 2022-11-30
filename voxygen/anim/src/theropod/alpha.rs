use super::{
    super::{vek::*, Animation},
    SkeletonAttr, TheropodSkeleton,
};
use common::states::utils::StageSection;

pub struct AlphaAnimation;

impl Animation for AlphaAnimation {
    type Dependency<'a> = (f32, f32, Option<StageSection>, f32);
    type Skeleton = TheropodSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"theropod_alpha\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "theropod_alpha")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, global_time, stage_section, timer): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        _s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (movement1base, movement2base, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powi(2), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time.powi(4), 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - movement3;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let movement1 = mirror * movement1base * pullback;
        let movement2 = mirror * movement2base * pullback;
        let movement1abs = movement1base * pullback;
        let movement2abs = movement2base * pullback;

        next.head.orientation = Quaternion::rotation_x(movement1abs * 0.2)
            * Quaternion::rotation_y(movement1 * 0.1 + movement2 * 0.2);
        next.neck.orientation = Quaternion::rotation_x(movement1abs * -0.3)
            * Quaternion::rotation_y(movement1 * 0.1 + movement2 * 0.1);

        next.jaw.orientation = Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * 0.5);

        next.chest_front.orientation = Quaternion::rotation_x(movement1abs * -0.2);
        next.chest_back.orientation = Quaternion::rotation_x(movement1abs * 0.2);

        next.leg_l.orientation = Quaternion::rotation_x(movement1abs * -0.1);

        next.leg_r.orientation = Quaternion::rotation_x(movement1abs * -0.1);
        next.foot_l.orientation = Quaternion::rotation_x(movement1abs * -0.3);
        next.foot_r.orientation = Quaternion::rotation_x(movement1abs * -0.3);

        next.tail_front.orientation =
            Quaternion::rotation_x(0.1 + movement1abs * -0.1 + movement2abs * -0.3)
                * Quaternion::rotation_z(movement1 * -0.1 + movement2 * -0.2);

        next.tail_back.orientation =
            Quaternion::rotation_x(0.1 + movement1abs * -0.1 + movement2abs * -0.3)
                * Quaternion::rotation_z(movement1 * -0.1 + movement2 * -0.2);
        next
    }
}
