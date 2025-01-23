use super::{
    super::{Animation, vek::*},
    QuadrupedSmallSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
//use std::ops::Rem;

pub struct ShockwaveAnimation;

impl Animation for ShockwaveAnimation {
    type Dependency<'a> = (f32, f32, Option<StageSection>, f32);
    type Skeleton = QuadrupedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_small_shockwave\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_small_shockwave")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, global_time, stage_section, timer): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        _s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (movement1base, movement2base, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.sqrt(), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time.powi(4), 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - movement3;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let _movement1 = mirror * movement1base * pullback;
        let _movement2 = mirror * movement2base * pullback;
        let _movement1abs = movement1base * pullback;
        let movement2abs = movement2base * pullback;
        // used by treant_sapling
        next.chest.position = Vec3::new(0.0, 0.0, -2.0 + movement2abs * 16.0);

        next
    }
}
