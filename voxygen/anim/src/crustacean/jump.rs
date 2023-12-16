use super::{
    super::{vek::*, Animation},
    CrustaceanSkeleton, SkeletonAttr,
};

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Dependency<'a> = (f32, Vec3<f32>, Vec3<f32>, f32, Vec3<f32>);
    type Skeleton = CrustaceanSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"crustacean_jump\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "crustacean_jump")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, _orientation, _last_ori, _global_time, _avg_vel): Self::Dependency<'_>,
        _anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        next.chest.scale = Vec3::one() * s_a.scaler;

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);

        let up_rot = 0.2;

        next.leg_fl.position = Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fr.position = Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fl.orientation =
            Quaternion::rotation_z(s_a.leg_ori.0) * Quaternion::rotation_y(up_rot);
        next.leg_fr.orientation =
            Quaternion::rotation_z(-s_a.leg_ori.0) * Quaternion::rotation_y(-up_rot);

        next.leg_cl.position = Vec3::new(-s_a.leg_c.0, s_a.leg_c.1, s_a.leg_c.2);
        next.leg_cr.position = Vec3::new(s_a.leg_c.0, s_a.leg_c.1, s_a.leg_c.2);
        next.leg_cl.orientation =
            Quaternion::rotation_z(s_a.leg_ori.1) * Quaternion::rotation_y(up_rot);
        next.leg_cr.orientation =
            Quaternion::rotation_z(-s_a.leg_ori.1) * Quaternion::rotation_y(-up_rot);

        next.leg_bl.position = Vec3::new(-s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_br.position = Vec3::new(s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_bl.orientation =
            Quaternion::rotation_z(s_a.leg_ori.2) * Quaternion::rotation_y(up_rot);
        next.leg_br.orientation =
            Quaternion::rotation_z(-s_a.leg_ori.2) * Quaternion::rotation_y(-up_rot);

        next
    }
}
