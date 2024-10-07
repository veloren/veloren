use super::{
    super::{vek::*, Animation},
    CrustaceanSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct RiposteMeleeAnimation;
impl Animation for RiposteMeleeAnimation {
    type Dependency<'a> = (Option<&'a str>, StageSection);
    type Skeleton = CrustaceanSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"crustacean_riposte_melee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "crustacean_riposte_melee")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_ability_id, stage_section): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        _s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let _slow = (anim_time * 2.0).sin();

        let (move1, move2, move3) = match stage_section {
            StageSection::Buildup => (anim_time.powf(0.25), 0.0, 0.0),
            StageSection::Action => (1.0, anim_time, 0.0),
            StageSection::Recover => (1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - move3;
        let move1 = move1 * pullback;
        let move2 = move2 * pullback;
        let _move2fast = move2.max(0.001).powf(0.25) * pullback;
        let _move2slow = move2.powi(4) * pullback;

        next.chest.position = Vec3::new(0.0, 0.0, 0.0 - move1 * 3.0);

        next.arm_r.orientation = Quaternion::rotation_z(move1 * -1.5 + move2 * 1.6);
        next.arm_r.position = Vec3::new(0.0 - move1 * 4.0, -4.0, 0.0);
        next.pincer_r1.position = Vec3::new(0.0, -3.0 * move1 + 4.0 * move2, 4.0);
        next.pincer_r1.orientation = Quaternion::rotation_x(move1 * -0.4 + move2 * 0.3);

        next.arm_l.orientation = Quaternion::rotation_z(move1 * 0.5 - move2 * 0.5);
        next.pincer_l1.orientation = Quaternion::rotation_x(move1 * -0.4 + move2 * 0.3);
        next.pincer_l1.position = Vec3::new(0.0, -3.0 * move1 + 4.0 * move2, 4.0);
        next.pincer_l0.orientation = Quaternion::rotation_x(move1 * 0.4 + move2 * -0.6);

        next
    }
}
