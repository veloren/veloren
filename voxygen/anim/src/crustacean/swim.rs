use super::{
    super::{vek::*, Animation},
    CrustaceanSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct SwimAnimation;

impl Animation for SwimAnimation {
    type Dependency<'a> = (Vec3<f32>, Vec3<f32>, Vec3<f32>, f32, Vec3<f32>, f32);
    type Skeleton = CrustaceanSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"crustacean_swim\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "crustacean_swim")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, orientation, _last_ori, _global_time, avg_vel, acc_vel): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = (Vec2::<f32>::from(velocity).magnitude()).min(22.0);
        *rate = 1.0;

        let speednorm = speed / 13.0;

        let direction = velocity.y * 0.098 * orientation.y + velocity.x * 0.098 * orientation.x;

        let side =
            (velocity.x * -0.098 * orientation.y + velocity.y * 0.098 * orientation.x) * -1.0;
        let sideabs = side.abs();

        let mixed_vel = (acc_vel + anim_time * 6.0) * 0.8; //sets run frequency using speed, with anim_time setting a floor

        //create a mix between a sine and a square wave
        //(controllable with ratio variable)
        let ratio = 0.1;
        let wave1 = (mixed_vel).sin();
        let wave2 = (mixed_vel - PI / 2.0).sin();
        let wave3 = (mixed_vel + PI / 2.0).sin();
        let wave4 = (mixed_vel + PI).sin();
        let slow_wave = (mixed_vel / 10.0).sin();
        let foot1 = wave1.abs().powf(ratio) * wave1.signum();
        let foot2 = wave2.abs().powf(ratio) * wave2.signum();
        let foot3 = wave3.abs().powf(ratio) * wave3.signum();
        let foot4 = wave4.abs().powf(ratio) * wave4.signum();

        let x_tilt = avg_vel.z.atan2(avg_vel.xy().magnitude()) * speednorm;

        next.chest.scale = Vec3::one() * s_a.scaler;

        let up_rot = 0.3;
        let turnaround = PI;
        let swim = -0.3 * foot2;

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + x_tilt);
        if s_a.move_sideways {
            next.chest.orientation =
                Quaternion::rotation_x((mixed_vel).sin().max(0.0) * 0.06 + x_tilt)
                    * Quaternion::rotation_z(turnaround + ((mixed_vel + PI / 2.0).sin() * 0.06));

            next.arm_l.orientation = Quaternion::rotation_x(0.1 * foot3)
                * Quaternion::rotation_y(foot4.max(sideabs * -1.0) * up_rot)
                * Quaternion::rotation_z((PI / -5.0) + 0.3 - foot2 * 0.1 * direction);
            next.arm_r.orientation = Quaternion::rotation_x(0.1 * foot3)
                * Quaternion::rotation_y(-foot1.max(sideabs * -1.0) * up_rot)
                * Quaternion::rotation_z((PI / 5.0) + -0.2 - foot3 * 0.1 * direction);
            next.arm_l.position = Vec3::new(0.0, -1.0, 0.0);
            next.arm_r.position = Vec3::new(0.0, -1.0, 0.0);
        } else {
            next.arm_l.orientation = Quaternion::rotation_z(0.7 * -slow_wave);
            next.arm_r.orientation = Quaternion::rotation_z(0.7 * slow_wave);
        }

        next.leg_fl.position = Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2 + 1.0);
        next.leg_fr.position = Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2 + 1.0);
        next.leg_fl.orientation = Quaternion::rotation_y(foot4.max(sideabs * -0.5) * up_rot)
            * Quaternion::rotation_z(swim + s_a.leg_ori.0 + foot2 * 0.1 * -direction);
        next.leg_fr.orientation = Quaternion::rotation_y(-foot1.max(sideabs * -0.5) * up_rot)
            * Quaternion::rotation_z(-swim + -s_a.leg_ori.0 - foot3 * 0.1 * -direction);

        next.leg_cl.position = Vec3::new(-s_a.leg_c.0, s_a.leg_c.1, s_a.leg_c.2);
        next.leg_cr.position = Vec3::new(s_a.leg_c.0, s_a.leg_c.1, s_a.leg_c.2);
        next.leg_cl.orientation = Quaternion::rotation_x(foot4 * 0.1 * -direction)
            * Quaternion::rotation_y(foot1.max(sideabs * -0.5) * up_rot)
            * Quaternion::rotation_z(swim + s_a.leg_ori.1 + foot3 * 0.2 * -direction);
        next.leg_cr.orientation = Quaternion::rotation_x(foot1 * 0.1 * -direction)
            * Quaternion::rotation_y(foot1.min(sideabs * -0.5) * up_rot)
            * Quaternion::rotation_z(-swim + -s_a.leg_ori.1 - foot2 * 0.2 * -direction);

        next.leg_bl.position = Vec3::new(-s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_br.position = Vec3::new(s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_bl.orientation = Quaternion::rotation_x(foot4 * 0.2)
            * Quaternion::rotation_y(foot4.max(sideabs * -0.5) * up_rot)
            * Quaternion::rotation_z(swim + s_a.leg_ori.2 + foot3 * 0.2 * direction);
        next.leg_br.orientation = Quaternion::rotation_x(foot1 * 0.2 * -direction)
            * Quaternion::rotation_y(foot4.min(sideabs * -0.5) * up_rot)
            * Quaternion::rotation_z(-swim + -s_a.leg_ori.2 - foot2 * 0.2 * direction);

        next
    }
}
