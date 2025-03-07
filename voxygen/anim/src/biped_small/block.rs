use super::{
    super::{Animation, vek::*},
    BipedSmallSkeleton, SkeletonAttr, biped_small_wield_sword, init_biped_small_alpha,
};
use common::states::utils::StageSection;

pub struct BlockAnimation;

type BlockAnimationDependency<'a> = (Option<&'a str>, StageSection);

impl Animation for BlockAnimation {
    type Dependency<'a> = BlockAnimationDependency<'a>;
    type Skeleton = BipedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_small_block\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "biped_small_block"))]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_id, stage_section): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        init_biped_small_alpha(&mut next, s_a);

        match ability_id {
            Some("common.abilities.haniwa.soldier.guard") => {
                let slow = (anim_time * 2.0).sin();
                biped_small_wield_sword(&mut next, s_a, 0.0, slow);

                let (move1, move2, move3) = match stage_section {
                    StageSection::Buildup => (anim_time, 0.0, 0.0),
                    StageSection::Action => (1.0, anim_time, 0.0),
                    StageSection::Recover => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let _move2 = move2 * pullback;

                next.detach_right = true;
                // For some reason there's a discontinuity when using detach_right, two offsets
                // below seem to help
                next.control_r.position += next.control.position
                    + Vec3::new(0.0, -2.0, 1.0)
                    + Vec3::new(2.0 * move3, -1.0 * move3, 2.0 * move3);
                next.control_r.orientation = next.control.orientation * next.control_r.orientation;

                next.control.orientation.rotate_x(move1 * -0.7);
                next.control.orientation.rotate_z(move1 * -1.4);
                next.control_r.orientation.rotate_x(move1 * 1.2);
                next.control_r.position += Vec3::new(0.0, 5.0 * move1, 6.0 * move1);
            },
            _ => {},
        }

        next
    }
}
