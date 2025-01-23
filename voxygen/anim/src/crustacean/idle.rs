use super::{
    super::{Animation, vek::*},
    CrustaceanSkeleton, SkeletonAttr,
};

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency<'a> = f32;
    type Skeleton = CrustaceanSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"crustacean_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "crustacean_idle")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        next.chest.scale = Vec3::one() * s_a.scaler;

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);

        let arm = (anim_time * 0.2).sin() * 0.03;
        let clasp_l =
            (((anim_time * 0.1).fract() * 2.0 - 1.0) * (anim_time * 2.0).sin().powi(2)).powi(16);
        let clasp_r = (((anim_time * 0.105 + 33.0).fract() * 2.0 - 1.0)
            * (anim_time * 1.95 + 20.0).sin().powi(2))
        .powi(16);

        next.arm_l.position = Vec3::zero();
        next.arm_l.orientation = Quaternion::rotation_x(arm);
        next.arm_r.position = Vec3::zero();
        next.arm_r.orientation = Quaternion::rotation_x(arm);

        next.pincer_l0.position = Vec3::zero();
        next.pincer_l1.position = Vec3::zero();
        next.pincer_l1.orientation = Quaternion::rotation_z(clasp_l * 0.15);
        next.pincer_r0.position = Vec3::zero();
        next.pincer_r1.position = Vec3::zero();
        next.pincer_r1.orientation = Quaternion::rotation_z(-clasp_r * 0.15);

        next.leg_fl.position = Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fr.position = Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fl.orientation = Quaternion::rotation_z(s_a.leg_ori.0);
        next.leg_fr.orientation = Quaternion::rotation_z(-s_a.leg_ori.0);

        next.leg_cl.position = Vec3::new(-s_a.leg_c.0, s_a.leg_c.1, s_a.leg_c.2);
        next.leg_cr.position = Vec3::new(s_a.leg_c.0, s_a.leg_c.1, s_a.leg_c.2);
        next.leg_cl.orientation = Quaternion::rotation_z(s_a.leg_ori.1);
        next.leg_cr.orientation = Quaternion::rotation_z(-s_a.leg_ori.1);

        next.leg_bl.position = Vec3::new(-s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_br.position = Vec3::new(s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_bl.orientation = Quaternion::rotation_z(s_a.leg_ori.2);
        next.leg_br.orientation = Quaternion::rotation_z(-s_a.leg_ori.2);

        next
    }
}
