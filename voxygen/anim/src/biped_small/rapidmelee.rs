use super::{
    super::Animation, BipedSmallSkeleton, SkeletonAttr, biped_small_alpha_spear,
    biped_small_wield_spear, init_biped_small_alpha,
};
use common::states::utils::StageSection;

pub struct RapidMeleeAnimation;
impl Animation for RapidMeleeAnimation {
    type Dependency<'a> = (Option<&'a str>, StageSection, (u32, Option<u32>));
    type Skeleton = BipedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_small_rapid_melee\0";

    #[cfg_attr(
        feature = "be-dyn-lib",
        unsafe(export_name = "biped_small_rapid_melee")
    )]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_id, stage_section, (_current_strike, _max_strikes)): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        init_biped_small_alpha(&mut next, s_a);

        match ability_id {
            Some("common.abilities.haniwa.guard.flurry") => {
                biped_small_wield_spear(&mut next, s_a, anim_time, 0.0, 0.0);

                let (move1, move2, move3) = match stage_section {
                    StageSection::Buildup => (anim_time.powf(0.25), 0.0, 0.0),
                    StageSection::Action => (1.0, anim_time, 0.0),
                    StageSection::Recover => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = ((move2 - 0.5).abs() * -2.0 + 1.0) * 0.75;
                let move2 = move2 * pullback;

                biped_small_alpha_spear(&mut next, s_a, move1, move2, anim_time, 0.0);
            },
            _ => {},
        }

        next
    }
}
