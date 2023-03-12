use super::{
    super::{vek::*, Animation},
    BirdMediumSkeleton, SkeletonAttr,
};

pub struct FlyAnimation;

impl Animation for FlyAnimation {
    type Dependency<'a> = (Vec3<f32>, Vec3<f32>, Vec3<f32>);
    type Skeleton = BirdMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"bird_medium_fly\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "bird_medium_fly")]
    fn update_skeleton_inner<'a>(
        skeleton: &Self::Skeleton,
        (velocity, orientation, last_ori): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let slow = (anim_time * 2.0).sin();
        let fast = (anim_time * 4.0).sin();

        // Harmonic series hack to get a sine/saw mix
        let freq = 8.0;
        let off1 = 0.0;
        let off2 = -1.7;
        let off3 = -2.0;
        let off4 = -2.4;
        let flap1 = 7.0 / 16.0 * (freq * anim_time + off1).sin()
            + 7.0 / 64.0 * (freq * 2.0 * anim_time + off1).sin()
            + 1.0 / 48.0 * (freq * 3.0 * anim_time + off1).sin();
        let flap2 = 7.0 / 16.0 * (freq * anim_time + off2).sin()
            + 7.0 / 64.0 * (freq * 2.0 * anim_time + off2).sin()
            + 1.0 / 48.0 * (freq * 3.0 * anim_time + off2).sin();
        let flap3 = 7.0 / 16.0 * (freq * anim_time + off3).sin()
            + 7.0 / 64.0 * (freq * 2.0 * anim_time + off3).sin()
            + 1.0 / 48.0 * (freq * 3.0 * anim_time + off3).sin();
        let flap4 = 7.0 / 16.0 * (freq * anim_time + off4).sin()
            + 7.0 / 64.0 * (freq * 2.0 * anim_time + off4).sin()
            + 1.0 / 48.0 * (freq * 3.0 * anim_time + off4).sin();

        let ori: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if vek::Vec2::new(ori, last_ori)
            .map(|o| o.magnitude_squared())
            .map(|m| m > 0.001 && m.is_finite())
            .reduce_and()
            && ori.angle_between(last_ori).is_finite()
        {
            ori.angle_between(last_ori).min(0.2)
                * last_ori.determine_side(Vec2::zero(), ori).signum()
        } else {
            0.0
        } * 1.3;

        next.head.scale = Vec3::one() * 0.99;
        next.leg_l.scale = Vec3::one() * s_a.scaler * 0.99;
        next.leg_r.scale = Vec3::one() * s_a.scaler * 0.99;
        next.chest.scale = Vec3::one() * s_a.scaler * 0.99;
        next.tail.scale = Vec3::one() * 1.01;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

        next.head.orientation = Quaternion::rotation_x(
            (-0.5 + 0.2 * velocity.xy().magnitude() / 5.0).min(-0.3) + fast * 0.05,
        );

        if velocity.z > 2.0 || velocity.xy().magnitude() < 12.0 {
            next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 - flap4 * 1.5);
            next.chest.orientation = Quaternion::rotation_x(
                (0.2 - 0.2 * velocity.xy().magnitude() / 5.0).max(-0.2) - flap1 * 0.2,
            ) * Quaternion::rotation_y(tilt * 1.8 + fast * 0.01);

            next.wing_in_l.position = Vec3::new(-s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);
            next.wing_in_r.position = Vec3::new(s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);

            next.wing_in_l.orientation =
                Quaternion::rotation_y(-flap1 * 1.9 + 0.2) * Quaternion::rotation_x(0.4);
            next.wing_in_r.orientation =
                Quaternion::rotation_y(flap1 * 1.9 - 0.2) * Quaternion::rotation_x(0.4);

            next.wing_out_l.position = Vec3::new(-s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
            next.wing_out_r.position = Vec3::new(s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);

            next.wing_out_l.orientation = Quaternion::rotation_y(-flap3 * 1.2 - 0.3);
            next.wing_out_r.orientation = Quaternion::rotation_y(flap3 * 1.2 + 0.3);

            next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
            next.tail.orientation =
                Quaternion::rotation_x(-flap2 * 0.2 + 0.1) * Quaternion::rotation_z(tilt * 1.0);

            next.leg_l.position = Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2 - flap4 * 1.5);
            next.leg_l.orientation = Quaternion::rotation_x(
                (-1.0 * velocity.xy().magnitude() / 5.0).max(-1.2) + flap1 * -0.1,
            ) * Quaternion::rotation_y(tilt * 1.6 + fast * 0.01);
            next.leg_r.position = Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2 - flap4 * 1.5);
            next.leg_r.orientation = Quaternion::rotation_x(
                (-1.0 * velocity.xy().magnitude() / 5.0).max(-1.2) + flap1 * -0.1,
            ) * Quaternion::rotation_y(tilt * 1.6 + fast * 0.01);
        } else {
            next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + slow * 0.05);
            next.chest.orientation =
                Quaternion::rotation_x(-0.2 + slow * 0.05 + (0.8 * velocity.z / 80.0).min(0.8))
                    * Quaternion::rotation_y(tilt * 1.8 + fast * 0.01);

            next.wing_in_l.position = Vec3::new(-s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);
            next.wing_in_r.position = Vec3::new(s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);

            next.wing_in_l.orientation =
                Quaternion::rotation_y(0.1 + slow * 0.04 + (0.8 * velocity.z / 80.0).min(0.8))
                    * Quaternion::rotation_x(0.4);
            next.wing_in_r.orientation =
                Quaternion::rotation_y(-0.1 + slow * -0.04 - (0.8 * velocity.z / 80.0).min(0.8))
                    * Quaternion::rotation_x(0.4);

            next.wing_out_l.position = Vec3::new(-s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
            next.wing_out_r.position = Vec3::new(s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
            next.wing_out_l.orientation =
                Quaternion::rotation_y(0.1 + slow * 0.04 + (0.4 * velocity.z / 80.0).min(0.2));
            next.wing_out_r.orientation =
                Quaternion::rotation_y(-0.1 + slow * -0.04 - (0.4 * velocity.z / 80.0).min(0.2));

            next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
            next.tail.orientation =
                Quaternion::rotation_x(0.04 - slow * 0.04) * Quaternion::rotation_z(tilt * 1.0);

            next.leg_l.position = Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2 + 1.0 + slow * 0.05);
            next.leg_l.orientation = Quaternion::rotation_x(-1.2 + slow * -0.05)
                * Quaternion::rotation_y(tilt * 1.6 + fast * 0.01);
            next.leg_r.position = Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2 + 1.0 + slow * 0.05);
            next.leg_r.orientation = Quaternion::rotation_x(-1.2 + slow * -0.05)
                * Quaternion::rotation_y(tilt * 1.6 + fast * 0.01);
        }

        next
    }
}
