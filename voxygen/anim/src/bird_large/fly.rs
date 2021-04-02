use super::{
    super::{vek::*, Animation},
    BirdLargeSkeleton, SkeletonAttr,
};

pub struct FlyAnimation;

impl Animation for FlyAnimation {
    type Dependency = (Vec3<f32>, Vec3<f32>, Vec3<f32>, f32, Vec3<f32>, f32);
    type Skeleton = BirdLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"bird_large_fly\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "bird_large_fly")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, orientation, last_ori, _global_time, avg_vel, _acc_vel): Self::Dependency,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let slow = (anim_time * 2.0).sin();
        let fast = (anim_time * 4.0).sin();

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
        let tilt = if ::vek::Vec2::new(ori, last_ori)
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
        let x_tilt = avg_vel.z.atan2(avg_vel.xy().magnitude());

        next.chest.scale = Vec3::one() * s_a.scaler / 4.0;

        next.neck.position = Vec3::new(0.0, s_a.neck.0, s_a.neck.1);
        next.neck.orientation = Quaternion::rotation_x(0.2);

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1 - 3.0);
        next.head.orientation = Quaternion::rotation_x(0.2 + fast * 0.05);

        next.beak.position = Vec3::new(0.0, s_a.beak.0, s_a.beak.1);

        dbg!(velocity);
        if velocity.z > 2.0 {
            next.chest.position =
                Vec3::new(0.0, s_a.chest.0, s_a.chest.1 - flap4 * 1.5) * s_a.scaler / 4.0;
            next.chest.orientation =
                Quaternion::rotation_x(-0.5 - flap1 * 0.2) * Quaternion::rotation_y(tilt * 1.8);

            next.wing_in_l.position = Vec3::new(-s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);
            next.wing_in_r.position = Vec3::new(s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);

            next.wing_in_l.orientation =
                Quaternion::rotation_y(-flap1 * 1.9 + 0.2) * Quaternion::rotation_x(0.4);
            next.wing_in_r.orientation =
                Quaternion::rotation_y(flap1 * 1.9 - 0.2) * Quaternion::rotation_x(0.4);

            next.wing_mid_l.position = Vec3::new(-s_a.wing_mid.0, s_a.wing_mid.1, s_a.wing_mid.2);
            next.wing_mid_r.position = Vec3::new(s_a.wing_mid.0, s_a.wing_mid.1, s_a.wing_mid.2);
            next.wing_mid_l.orientation = Quaternion::rotation_y(-flap2 * 1.4 - 0.2);
            next.wing_mid_r.orientation = Quaternion::rotation_y(flap2 * 1.4 + 0.2);

            next.wing_out_l.position = Vec3::new(-s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
            next.wing_out_r.position = Vec3::new(s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
            next.wing_out_l.orientation = Quaternion::rotation_y(-flap3 * 1.2 - 0.3);
            next.wing_out_r.orientation = Quaternion::rotation_y(flap3 * 1.2 + 0.3);

            next.tail_front.position = Vec3::new(0.0, s_a.tail_front.0, s_a.tail_front.1);
            next.tail_front.orientation =
                Quaternion::rotation_x(-flap2 * 0.2) * Quaternion::rotation_z(-tilt * 1.0);
            next.tail_rear.position = Vec3::new(0.0, s_a.tail_rear.0, s_a.tail_rear.1);
            next.tail_rear.orientation =
                Quaternion::rotation_x(-flap3 * 0.3) * Quaternion::rotation_z(-tilt * 1.0);
        } else {
            next.chest.position =
                Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + slow * 0.05) * s_a.scaler / 4.0;
            next.chest.orientation =
                Quaternion::rotation_x(-0.5 + slow * 0.05) * Quaternion::rotation_y(tilt * 1.8);

            next.wing_in_l.position = Vec3::new(-s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);
            next.wing_in_r.position = Vec3::new(s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);

            next.wing_in_l.orientation =
                Quaternion::rotation_y(0.1 + slow * 0.04) * Quaternion::rotation_x(0.6);
            next.wing_in_r.orientation =
                Quaternion::rotation_y(-0.1 + slow * -0.04) * Quaternion::rotation_x(0.6);

            next.wing_mid_l.position = Vec3::new(-s_a.wing_mid.0, s_a.wing_mid.1, s_a.wing_mid.2);
            next.wing_mid_r.position = Vec3::new(s_a.wing_mid.0, s_a.wing_mid.1, s_a.wing_mid.2);
            next.wing_mid_l.orientation = Quaternion::rotation_y(0.1 + slow * 0.04);
            next.wing_mid_r.orientation = Quaternion::rotation_y(-0.1 + slow * -0.04);

            next.wing_out_l.position = Vec3::new(-s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
            next.wing_out_r.position = Vec3::new(s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
            next.wing_out_l.orientation = Quaternion::rotation_y(0.1 + slow * 0.04);
            next.wing_out_r.orientation = Quaternion::rotation_y(-0.1 + slow * -0.04);

            next.tail_front.position = Vec3::new(0.0, s_a.tail_front.0, s_a.tail_front.1);
            next.tail_front.orientation =
                Quaternion::rotation_x(slow * 0.04) * Quaternion::rotation_z(-tilt * 1.0);
            next.tail_rear.position = Vec3::new(0.0, s_a.tail_rear.0, s_a.tail_rear.1);
            next.tail_rear.orientation =
                Quaternion::rotation_x(-0.2 + slow * 0.08) * Quaternion::rotation_z(-tilt * 1.0);
        }

        next.leg_l.position = Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2 + 3.0);
        next.leg_l.orientation = Quaternion::rotation_x(-1.0 - flap1 * 0.1);
        next.leg_r.position = Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2 + 3.0);
        next.leg_r.orientation = Quaternion::rotation_x(-1.0 - flap1 * 0.1);

        next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2);
        next.foot_l.orientation = Quaternion::rotation_x(-1.0 - flap1 * 0.1);
        next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2);
        next.foot_r.orientation = Quaternion::rotation_x(-1.0 - flap1 * 0.1);

        next
    }
}
