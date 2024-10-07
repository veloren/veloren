use super::{
    super::{vek::*, Animation},
    CrustaceanSkeleton, SkeletonAttr,
};
use common::{states::utils::StageSection, util::Dir};

pub struct SummonAnimation;

type SummonAnimationDependency = (f32, Option<StageSection>, f32, Dir, bool);

impl Animation for SummonAnimation {
    type Dependency<'a> = SummonAnimationDependency;
    type Skeleton = CrustaceanSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"crustacean_summon\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "crustacean_summon")]
    fn update_skeleton_inner<'a>(
        skeleton: &Self::Skeleton,
        (global_time, stage_section, timer, _look_dir, _on_ground): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        _s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (movement1base, movement2base, movement3, twitch) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time.min(1.0).powf(0.1), 0.0, anim_time),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time.min(1.0).powi(2), 1.0),
            _ => (0.0, 0.0, 0.0, 0.0),
        };

        let pullback = 1.0 - movement3;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let twitch2 = mirror * (twitch * 20.0).sin() * pullback;

        let movement1abs = movement1base * pullback;
        let _movement2abs = movement2base * pullback;

        let _wave_slow_cos = (anim_time * 4.5).cos();
        next.chest.position = Vec3::new(0.0, 6.0, -4.0);
        next.chest.orientation = Quaternion::rotation_x(twitch2 * 0.1)
            * Quaternion::rotation_y(twitch2 * -0.1)
            * Quaternion::rotation_y(twitch2 * 0.1);

        next.arm_r.orientation = Quaternion::rotation_z(movement1abs * -0.3);
        next.arm_l.orientation = Quaternion::rotation_z(movement1abs * 0.3);
        next
    }
}
