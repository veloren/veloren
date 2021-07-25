use super::{
    super::{vek::*, Animation},
    ArthropodSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct AlphaAnimation;

impl Animation for AlphaAnimation {
    type Dependency<'a> = (f32, f32, Option<StageSection>, f32);
    type Skeleton = ArthropodSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"arthropod_alpha\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "arthropod_alpha")]
    fn update_skeleton_inner<'a>(
        skeleton: &Self::Skeleton,
        (_velocity, global_time, stage_section, timer): Self::Dependency<'a>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (movement1base, movement2base, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powi(2), 0.0, 0.0),
            Some(StageSection::Swing) => (1.0, anim_time.powi(4), 0.0),
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

        next.chest.scale = Vec3::one() / s_a.scaler;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);

        next.mandible_l.position = Vec3::new(-s_a.mandible_l.0, s_a.mandible_l.1, s_a.mandible_l.2);
        next.mandible_r.position = Vec3::new(s_a.mandible_r.0, s_a.mandible_r.1, s_a.mandible_r.2);

        next.wing_fl.position = Vec3::new(-s_a.wing_fl.0, s_a.wing_fl.1, s_a.wing_fl.2);
        next.wing_fr.position = Vec3::new(s_a.wing_fr.0, s_a.wing_fr.1, s_a.wing_fr.2);

        next.wing_bl.position = Vec3::new(-s_a.wing_bl.0, s_a.wing_bl.1, s_a.wing_bl.2);
        next.wing_br.position = Vec3::new(s_a.wing_br.0, s_a.wing_br.1, s_a.wing_br.2);

        next.leg_fl.position = Vec3::new(-s_a.leg_fl.0, s_a.leg_fl.1, s_a.leg_fl.2);
        next.leg_fr.position = Vec3::new(s_a.leg_fr.0, s_a.leg_fr.1, s_a.leg_fr.2);

        next.leg_fcl.position = Vec3::new(-s_a.leg_fcl.0, s_a.leg_fcl.1, s_a.leg_fcl.2);
        next.leg_fcr.position = Vec3::new(s_a.leg_fcr.0, s_a.leg_fcr.1, s_a.leg_fcr.2);

        next.leg_bcl.position = Vec3::new(-s_a.leg_bcl.0, s_a.leg_bcl.1, s_a.leg_bcl.2);
        next.leg_bcr.position = Vec3::new(s_a.leg_bcr.0, s_a.leg_bcr.1, s_a.leg_bcr.2);

        next.leg_bl.position = Vec3::new(-s_a.leg_bl.0, s_a.leg_bl.1, s_a.leg_bl.2);
        next.leg_br.position = Vec3::new(s_a.leg_br.0, s_a.leg_br.1, s_a.leg_br.2);

        next
    }
}
