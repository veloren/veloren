use super::{
    super::{vek::*, Animation},
    ArthropodSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct StunnedAnimation;

impl Animation for StunnedAnimation {
    type Dependency<'a> = (f32, f32, Option<StageSection>, f32);
    type Skeleton = ArthropodSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"arthropod_stunned\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "arthropod_stunned")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, global_time, stage_section, timer): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (_movement1base, movement2, twitch) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.1), 0.0, anim_time),
            Some(StageSection::Recover) => (1.0, anim_time.powf(4.0), 1.0),
            _ => (0.0, 0.0, 0.0),
        };

        let pullback = 1.0 - movement2;

        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let twitch1 = mirror * (twitch * 20.0).cos() * pullback;
        let twitch2 = mirror * (twitch * 20.0).sin() * pullback;

        next.chest.scale = Vec3::one() / s_a.scaler;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_z(twitch2 * 0.6);

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);

        next.mandible_l.position = Vec3::new(-s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);
        next.mandible_r.position = Vec3::new(s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);

        next.wing_fl.position = Vec3::new(-s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);
        next.wing_fr.position = Vec3::new(s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);

        next.wing_bl.position = Vec3::new(-s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);
        next.wing_br.position = Vec3::new(s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);

        next.leg_fl.position = Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fr.position = Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fl.orientation =
            Quaternion::rotation_z(s_a.leg_ori.0) * Quaternion::rotation_x(twitch1 * 0.8 + 0.4);
        next.leg_fr.orientation =
            Quaternion::rotation_z(-s_a.leg_ori.0) * Quaternion::rotation_x(-twitch1 * 0.8 - 0.4);

        next.leg_fcl.position = Vec3::new(-s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);
        next.leg_fcr.position = Vec3::new(s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);
        next.leg_fcl.orientation =
            Quaternion::rotation_z(s_a.leg_ori.1) * Quaternion::rotation_y(twitch2 * 0.8 + 0.4);
        next.leg_fcr.orientation =
            Quaternion::rotation_z(-s_a.leg_ori.1) * Quaternion::rotation_y(-twitch2 * 0.8 - 0.4);

        next.leg_bcl.position = Vec3::new(-s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);
        next.leg_bcr.position = Vec3::new(s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);
        next.leg_bcl.orientation =
            Quaternion::rotation_z(s_a.leg_ori.2) * Quaternion::rotation_y(twitch1 * 0.8 + 0.4);
        next.leg_bcr.orientation =
            Quaternion::rotation_z(-s_a.leg_ori.2) * Quaternion::rotation_y(-twitch1 * 0.8 - 0.4);

        next.leg_bl.position = Vec3::new(-s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_br.position = Vec3::new(s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_bl.orientation =
            Quaternion::rotation_z(s_a.leg_ori.3) * Quaternion::rotation_y(twitch2 * 0.8 + 0.4);
        next.leg_br.orientation =
            Quaternion::rotation_z(-s_a.leg_ori.3) * Quaternion::rotation_y(-twitch2 * 0.8 - 0.4);

        next
    }
}
