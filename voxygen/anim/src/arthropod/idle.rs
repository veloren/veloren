use super::{super::Animation, ArthropodSkeleton, SkeletonAttr};
//use std::{f32::consts::PI, ops::Mul};
use super::super::vek::*;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency<'a> = f32;
    type Skeleton = ArthropodSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"arthropod_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "arthropod_idle")]
    fn update_skeleton_inner<'a>(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency<'a>,
        _anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        next.chest.scale = Vec3::one();

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);

        next.mandible_l.position = Vec3::new(-s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);
        next.mandible_r.position = Vec3::new(s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);

        next.wing_fl.position = Vec3::new(-s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);
        next.wing_fr.position = Vec3::new(s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);

        next.wing_bl.position = Vec3::new(-s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);
        next.wing_br.position = Vec3::new(s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);

        next.leg_fl.position = Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fr.position = Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fl.orientation = Quaternion::rotation_z(s_a.leg_ori.0);
        next.leg_fr.orientation = Quaternion::rotation_z(-s_a.leg_ori.0);

        next.leg_fcl.position = Vec3::new(-s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);
        next.leg_fcr.position = Vec3::new(s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);
        next.leg_fcl.orientation = Quaternion::rotation_z(s_a.leg_ori.1);
        next.leg_fcr.orientation = Quaternion::rotation_z(-s_a.leg_ori.1);

        next.leg_bcl.position = Vec3::new(-s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);
        next.leg_bcr.position = Vec3::new(s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);
        next.leg_bcl.orientation = Quaternion::rotation_z(s_a.leg_ori.2);
        next.leg_bcr.orientation = Quaternion::rotation_z(-s_a.leg_ori.2);

        next.leg_bl.position = Vec3::new(-s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_br.position = Vec3::new(s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_bl.orientation = Quaternion::rotation_z(s_a.leg_ori.3);
        next.leg_br.orientation = Quaternion::rotation_z(-s_a.leg_ori.3);

        next
    }
}
