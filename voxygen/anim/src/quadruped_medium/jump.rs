use super::{
    super::{vek::*, Animation},
    QuadrupedMediumSkeleton, SkeletonAttr,
};

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Dependency<'a> = (f32, Vec3<f32>, Vec3<f32>);
    type Skeleton = QuadrupedMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_medium_jump\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_medium_jump")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_global_time, velocity, avg_vel): Self::Dependency<'_>,
        _anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        next.neck.scale = Vec3::one() * 1.02;
        next.jaw.scale = Vec3::one() * 1.02;
        next.leg_fl.scale = Vec3::one() * 1.02;
        next.leg_fr.scale = Vec3::one() * 1.02;
        next.leg_bl.scale = Vec3::one() * 1.02;
        next.leg_br.scale = Vec3::one() * 1.02;
        next.foot_fl.scale = Vec3::one() * 0.96;
        next.foot_fr.scale = Vec3::one() * 0.96;
        next.foot_bl.scale = Vec3::one() * 0.96;
        next.foot_br.scale = Vec3::one() * 0.96;
        next.ears.scale = Vec3::one() * 1.02;
        let speed = Vec2::<f32>::from(velocity).magnitude();
        let velocityalt = speed.max(3.0);
        let normalize = velocityalt / 22.0;

        let x_tilt = (avg_vel.z.atan2(avg_vel.xy().magnitude()) * normalize).max(-0.28);
        let x_tilt = if velocityalt < 3.5 {
            x_tilt.abs()
        } else {
            x_tilt
        };

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_x(x_tilt * -0.5);

        next.neck.position = Vec3::new(0.0, s_a.neck.0, s_a.neck.1);
        next.neck.orientation = Quaternion::rotation_x(x_tilt * -1.0);

        next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
        next.jaw.orientation = Quaternion::rotation_x(0.0);

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.tail.orientation = Quaternion::rotation_x(-0.6 * normalize + x_tilt * 2.0);

        next.torso_front.position = Vec3::new(0.0, s_a.torso_front.0, s_a.torso_front.1);
        next.torso_front.orientation = Quaternion::rotation_x(x_tilt * 3.5);

        next.torso_back.position = Vec3::new(0.0, s_a.torso_back.0, s_a.torso_back.1);
        next.torso_back.orientation = Quaternion::rotation_x(x_tilt * -1.2);

        next.ears.position = Vec3::new(0.0, s_a.ears.0, s_a.ears.1);
        next.ears.orientation = Quaternion::rotation_x(x_tilt * 1.5);

        next.leg_fl.position = Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fl.orientation = Quaternion::rotation_x(1.2 * normalize + x_tilt.abs() * 0.8);

        next.leg_fr.position = Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fr.orientation = Quaternion::rotation_x(1.2 * normalize + x_tilt.abs() * 0.8);

        next.leg_bl.position = Vec3::new(-s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_bl.orientation = Quaternion::rotation_x(-0.8 * normalize + x_tilt * -0.8);

        next.leg_br.position = Vec3::new(s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_br.orientation = Quaternion::rotation_x(-0.8 * normalize + x_tilt * -0.8);

        next.foot_fl.position = Vec3::new(-s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2);
        next.foot_fl.orientation = Quaternion::rotation_x(-0.4 * normalize + x_tilt * -0.4);

        next.foot_fr.position = Vec3::new(s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2);
        next.foot_fr.orientation = Quaternion::rotation_x(-0.4 * normalize + x_tilt * -0.4);

        next.foot_bl.position = Vec3::new(-s_a.feet_b.0, s_a.feet_b.1, s_a.feet_b.2);
        next.foot_bl.orientation = Quaternion::rotation_x(-0.4 * normalize + x_tilt * -0.4);

        next.foot_br.position = Vec3::new(s_a.feet_b.0, s_a.feet_b.1, s_a.feet_b.2);
        next.foot_br.orientation = Quaternion::rotation_x(-0.4 * normalize + x_tilt * -0.4);

        next
    }
}
