use super::{
    super::{vek::*, Animation},
    ArthropodSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency<'a> = (Vec3<f32>, Vec3<f32>, Vec3<f32>, f32, Vec3<f32>, f32);
    type Skeleton = ArthropodSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"arthropod_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "arthropod_run")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, _orientation, _last_ori, _global_time, avg_vel, acc_vel): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = (Vec2::<f32>::from(velocity).magnitude()).min(22.0);
        *rate = 1.0;

        let speednorm = speed / 13.0;

        let mixed_vel = (acc_vel + anim_time * 6.0) * 0.8; //sets run frequency using speed, with anim_time setting a floor

        //create a mix between a sine and a square wave
        //(controllable with ratio variable)
        let ratio = 0.1;
        let wave1 = (mixed_vel).sin();
        let wave2 = (mixed_vel - PI / 2.0).sin();
        let wave3 = (mixed_vel + PI / 2.0).sin();
        let wave4 = (mixed_vel + PI).sin();
        let foot1 = wave1.abs().powf(ratio) * wave1.signum();
        let foot2 = wave2.abs().powf(ratio) * wave2.signum();
        let foot3 = wave3.abs().powf(ratio) * wave3.signum();
        let foot4 = wave4.abs().powf(ratio) * wave4.signum();

        let x_tilt = avg_vel.z.atan2(avg_vel.xy().magnitude()) * speednorm;

        next.chest.scale = Vec3::one() / s_a.scaler;
        next.wing_bl.scale = Vec3::one() * 0.98;
        next.wing_br.scale = Vec3::one() * 0.98;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_x((mixed_vel).sin() * 0.1)
            * Quaternion::rotation_y((mixed_vel).sin().min(0.0) * 0.08)
            * Quaternion::rotation_z((mixed_vel + PI * 1.5).sin() * 0.08);

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + x_tilt);
        next.chest.orientation = Quaternion::rotation_x((mixed_vel).sin().max(0.0) * 0.06 + x_tilt)
            * Quaternion::rotation_z((mixed_vel + PI / 2.0).sin() * 0.06);

        next.mandible_l.position = Vec3::new(-s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);
        next.mandible_r.position = Vec3::new(s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);

        next.wing_fl.position = Vec3::new(-s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);
        next.wing_fr.position = Vec3::new(s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);

        next.wing_bl.position = Vec3::new(-s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);
        next.wing_br.position = Vec3::new(s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);

        next.leg_fl.position = Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fr.position = Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fl.orientation = Quaternion::rotation_x(foot4.max(0.0) * 0.7)
            * Quaternion::rotation_z(s_a.leg_ori.0 + foot2 * 0.4);
        next.leg_fr.orientation = Quaternion::rotation_x(foot1.max(0.0) * 0.7)
            * Quaternion::rotation_z(-s_a.leg_ori.0 - foot3 * 0.4);

        next.leg_fcl.position = Vec3::new(-s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);
        next.leg_fcr.position = Vec3::new(s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);
        next.leg_fcl.orientation = Quaternion::rotation_x(foot4 * 0.2)
            * Quaternion::rotation_y(foot1.max(0.0) * 0.7)
            * Quaternion::rotation_z(s_a.leg_ori.1 + foot3 * 0.2);
        next.leg_fcr.orientation = Quaternion::rotation_x(foot1 * 0.2)
            * Quaternion::rotation_y(foot1.min(0.0) * 0.7)
            * Quaternion::rotation_z(-s_a.leg_ori.1 - foot2 * 0.2);

        next.leg_bcl.position = Vec3::new(-s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);
        next.leg_bcr.position = Vec3::new(s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);
        next.leg_bcl.orientation = Quaternion::rotation_x(foot4 * 0.2)
            * Quaternion::rotation_y(foot4.max(0.0) * 0.7)
            * Quaternion::rotation_z(s_a.leg_ori.2 + foot2 * 0.3);
        next.leg_bcr.orientation = Quaternion::rotation_x(foot1 * 0.2)
            * Quaternion::rotation_y(foot4.min(0.0) * 0.7)
            * Quaternion::rotation_z(-s_a.leg_ori.2 - foot3 * 0.3);

        next.leg_bl.position = Vec3::new(-s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_br.position = Vec3::new(s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_bl.orientation = Quaternion::rotation_x(foot4 * 0.2)
            * Quaternion::rotation_y(foot1.max(0.0) * 0.7)
            * Quaternion::rotation_z(s_a.leg_ori.3 + foot3 * 0.2);
        next.leg_br.orientation = Quaternion::rotation_x(foot1 * 0.2)
            * Quaternion::rotation_y(foot1.min(0.0) * 0.7)
            * Quaternion::rotation_z(-s_a.leg_ori.3 - foot2 * 0.2);

        next
    }
}
