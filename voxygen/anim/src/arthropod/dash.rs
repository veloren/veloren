use super::{super::Animation, ArthropodSkeleton, SkeletonAttr};
//use std::{f32::consts::PI, ops::Mul};
use super::super::vek::*;
use common::states::utils::StageSection;
use std::f32::consts::PI;

pub struct DashAnimation;

impl Animation for DashAnimation {
    type Dependency<'a> = (f32, f32, Option<StageSection>, f32);
    type Skeleton = ArthropodSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"arthropod_dash\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "arthropod_dash")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, _global_time, stage_section, _timer): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (movement1base, chargemovementbase, movement2base, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powi(4), 0.0, 0.0, 0.0),
            Some(StageSection::Charge) => (1.0, 1.0, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, 1.0, anim_time.powi(6), 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - movement3;
        let movement1abs = movement1base * pullback;
        let movement2abs = movement2base * pullback;
        let shortalt = (anim_time * 200.0 + PI * 0.25).sin() * chargemovementbase * pullback;

        next.chest.scale = Vec3::one();

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_x(movement1abs * -0.4 + movement2abs * 1.4);

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);

        next.mandible_l.position = Vec3::new(-s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);
        next.mandible_r.position = Vec3::new(s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);
        next.mandible_l.orientation =
            Quaternion::rotation_z(movement1abs * 0.5 + movement2abs * -0.7);
        next.mandible_r.orientation =
            Quaternion::rotation_z(movement1abs * -0.5 + movement2abs * 0.7);

        next.wing_fl.position = Vec3::new(-s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);
        next.wing_fr.position = Vec3::new(s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);
        next.wing_fl.orientation =
            Quaternion::rotation_x(movement1abs * -0.4 + shortalt * 0.2 + movement2abs * -0.6)
                * Quaternion::rotation_y(movement1abs * -0.5 + movement2abs * -0.1)
                * Quaternion::rotation_z(movement1abs * -0.2);
        next.wing_fr.orientation =
            Quaternion::rotation_x(movement1abs * -0.4 + shortalt * 0.2 + movement2abs * -0.6)
                * Quaternion::rotation_y(movement1abs * 0.5 + movement2abs * 0.1)
                * Quaternion::rotation_z(movement1abs * 0.2);

        next.wing_bl.position = Vec3::new(-s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);
        next.wing_br.position = Vec3::new(s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);
        next.wing_bl.orientation =
            Quaternion::rotation_x(movement1abs * -0.2 + shortalt * 0.2 + movement2abs * -0.6)
                * Quaternion::rotation_y(movement1abs * -0.4 + movement2abs * -0.1);
        next.wing_br.orientation =
            Quaternion::rotation_x(movement1abs * -0.2 + shortalt * 0.2 + movement2abs * -0.6)
                * Quaternion::rotation_y(movement1abs * 0.4 + movement2abs * 0.1);

        next
    }
}
