use super::{super::Animation, ArthropodSkeleton, SkeletonAttr};
//use std::f32::consts::PI;
use super::super::vek::*;

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Dependency<'a> = (f32, Vec3<f32>, Vec3<f32>, f32, Vec3<f32>);
    type Skeleton = ArthropodSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"arthropod_jump\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "arthropod_jump")]
    fn update_skeleton_inner<'a>(
        skeleton: &Self::Skeleton,
        (_velocity, _orientation, _last_ori, _global_time, _avg_vel): Self::Dependency<'a>,
        _anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

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
