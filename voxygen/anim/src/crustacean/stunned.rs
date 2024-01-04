use super::{
    super::{vek::*, Animation},
    CrustaceanSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct StunnedAnimation;

impl Animation for StunnedAnimation {
    type Dependency<'a> = (f32, f32, Option<StageSection>, f32);
    type Skeleton = CrustaceanSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"crustacean_stunned\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "crustacean_stunned")]
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

        let pullback = (1.0 - movement2) * 0.1;

        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let twitch1 = mirror * (twitch * 5.0).cos() * pullback;
        let twitch2 = mirror * (twitch * 5.0).sin() * pullback;

        next.chest.scale = Vec3::one() * s_a.scaler;

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);

        next.arm_l.orientation = Quaternion::rotation_x(-twitch2 * 1.2);
        next.arm_r.orientation = Quaternion::rotation_x(twitch2 * 1.2);
        next.pincer_l1.orientation = Quaternion::rotation_z(0.17);
        next.pincer_r1.orientation = Quaternion::rotation_z(-0.17);

        next.leg_fl.position = Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fr.position = Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fl.orientation =
            Quaternion::rotation_z(s_a.leg_ori.0) * Quaternion::rotation_x(twitch1 * 0.8 + 0.4);
        next.leg_fr.orientation =
            Quaternion::rotation_z(-s_a.leg_ori.0) * Quaternion::rotation_x(-twitch1 * 0.8 - 0.4);

        next.leg_cl.position = Vec3::new(-s_a.leg_c.0, s_a.leg_c.1, s_a.leg_c.2);
        next.leg_cr.position = Vec3::new(s_a.leg_c.0, s_a.leg_c.1, s_a.leg_c.2);
        next.leg_cl.orientation =
            Quaternion::rotation_z(s_a.leg_ori.1) * Quaternion::rotation_y(twitch2 * 0.4 + 0.4);
        next.leg_cr.orientation =
            Quaternion::rotation_z(-s_a.leg_ori.1) * Quaternion::rotation_y(-twitch2 * 0.4 - 0.4);

        next.leg_bl.position = Vec3::new(-s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_br.position = Vec3::new(s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_bl.orientation =
            Quaternion::rotation_z(s_a.leg_ori.2) * Quaternion::rotation_y(twitch2 * 0.4 + 0.4);
        next.leg_br.orientation =
            Quaternion::rotation_z(-s_a.leg_ori.2) * Quaternion::rotation_y(-twitch2 * 0.4 - 0.4);

        next
    }
}
