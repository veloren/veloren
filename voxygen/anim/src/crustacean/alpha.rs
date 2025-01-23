use super::{
    super::{Animation, vek::*},
    CrustaceanSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct AlphaAnimation;

impl Animation for AlphaAnimation {
    type Dependency<'a> = (f32, f32, Option<StageSection>, f32);
    type Skeleton = CrustaceanSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"crustacean_alpha\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "crustacean_alpha")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, global_time, stage_section, timer): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (movement1, movement2, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powi(2), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time.powi(4), 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - movement3;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let _mirror = (check - 0.5).signum();
        let _movement1abs = movement1 * pullback;
        let _movement2abs = movement2 * pullback;
        let _movement3abs = movement3 * pullback;

        next.arm_l.orientation = Quaternion::rotation_x(anim_time);

        next.chest.scale = Vec3::one() * s_a.scaler;

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);

        next.leg_fl.position = Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fr.position = Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);

        next.leg_cl.position = Vec3::new(-s_a.leg_c.0, s_a.leg_c.1, s_a.leg_c.2);
        next.leg_cr.position = Vec3::new(s_a.leg_c.0, s_a.leg_c.1, s_a.leg_c.2);

        next.leg_bl.position = Vec3::new(-s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_br.position = Vec3::new(s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next
    }
}
