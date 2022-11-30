use super::{
    super::{vek::*, Animation},
    DragonSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency<'a> = (f32, Vec3<f32>, Vec3<f32>, f32, Vec3<f32>);
    type Skeleton = DragonSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"dragon_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "dragon_run")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, orientation, last_ori, _global_time, avg_vel): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab: f32 = 0.6; //6

        let short = ((1.0 / (0.72 + 0.28 * ((anim_time * 16.0 * lab + PI * 1.0).sin()).powi(2)))
            .sqrt())
            * ((anim_time * 16.0 * lab + PI * 1.0).sin());

        //

        let shortalt = (anim_time * 16.0 * lab + PI * 0.5).sin();

        //
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
        let x_tilt = avg_vel.z.atan2(avg_vel.xy().magnitude());

        let lab: f32 = 14.0;

        let wave_ultra_slow_cos = (anim_time * 3.0 + PI).cos();
        let wave_slow = (anim_time * 4.5).sin();

        let vertlf = (anim_time * lab + PI * 1.8).sin().max(0.15);
        let vertrfoffset = (anim_time * lab + PI * 0.80).sin().max(0.15);
        let vertlboffset = (anim_time * lab).sin().max(0.15);
        let vertrb = (anim_time * lab + PI).sin().max(0.15);

        let horilf = (anim_time * lab + PI * 1.2).sin();
        let horirfoffset = (anim_time * lab + PI * 0.20).sin();
        let horilboffset = (anim_time * lab + PI * 1.4).sin();
        let horirb = (anim_time * lab + PI * 0.4).sin();

        let center = (anim_time * lab + PI / 2.0).sin();
        let centeroffset = (anim_time * lab + PI * 1.5).sin();

        next.head_lower.scale = Vec3::one() * 1.02;
        next.jaw.scale = Vec3::one() * 1.05;
        next.tail_front.scale = Vec3::one() * 0.98;
        next.tail_rear.scale = Vec3::one() * 0.98;

        next.head_upper.position = Vec3::new(0.0, s_a.head_upper.0, s_a.head_upper.1);
        next.head_upper.orientation = Quaternion::rotation_x(short * -0.03 - 0.1)
            * Quaternion::rotation_z(tilt * -1.2)
            * Quaternion::rotation_y(tilt * 0.8);

        next.head_lower.position = Vec3::new(0.0, s_a.head_lower.0, s_a.head_lower.1);
        next.head_lower.orientation = Quaternion::rotation_z(tilt * -0.8)
            * Quaternion::rotation_x(short * -0.05)
            * Quaternion::rotation_y(tilt * 0.3);

        next.jaw.position = Vec3::new(
            0.0,
            s_a.jaw.0 - wave_ultra_slow_cos * 0.12,
            s_a.jaw.1 + wave_slow * 0.2,
        );
        next.jaw.orientation = Quaternion::rotation_x(wave_slow * 0.03);

        next.tail_front.position =
            Vec3::new(0.0, s_a.tail_front.0, s_a.tail_front.1 + centeroffset * 0.6);
        next.tail_front.orientation =
            Quaternion::rotation_x(center * 0.03) * Quaternion::rotation_z(tilt * 1.5);

        next.tail_rear.position =
            Vec3::new(0.0, s_a.tail_rear.0, s_a.tail_rear.1 + centeroffset * 0.6);
        next.tail_rear.orientation =
            Quaternion::rotation_x(center * 0.03) * Quaternion::rotation_z(tilt * 1.5);

        next.chest_front.position = Vec3::new(
            0.0,
            s_a.chest_front.0,
            s_a.chest_front.1 + shortalt * 2.5 + x_tilt * 10.0,
        );
        next.chest_front.orientation = Quaternion::rotation_x(short * 0.13 + x_tilt)
            * Quaternion::rotation_y(0.0)
            * Quaternion::rotation_z(tilt * -1.5);

        next.chest_rear.position =
            Vec3::new(0.0, s_a.chest_rear.0, s_a.chest_rear.1 + shortalt * 0.2);
        next.chest_rear.orientation = Quaternion::rotation_x(short * 0.1)
            * Quaternion::rotation_y(0.0)
            * Quaternion::rotation_z(tilt * 1.8);

        next.foot_fl.position = Vec3::new(
            -s_a.feet_f.0,
            s_a.feet_f.1 + horilf * 2.5,
            s_a.feet_f.2 + vertlf * 5.0 * s_a.height - 0.5,
        );
        next.foot_fl.orientation = Quaternion::rotation_x(horilf * 0.6);

        next.foot_fr.position = Vec3::new(
            s_a.feet_f.0,
            s_a.feet_f.1 + horirfoffset * 2.5,
            s_a.feet_f.2 + vertrfoffset * 5.0 * s_a.height - 0.5,
        );
        next.foot_fr.orientation = Quaternion::rotation_x(horirb * 0.6);

        next.foot_bl.position = Vec3::new(
            -s_a.feet_b.0,
            s_a.feet_b.1 + horilboffset * 3.0,
            s_a.feet_b.2 + vertlboffset * 5.0 * s_a.height - 0.5,
        );
        next.foot_bl.orientation = Quaternion::rotation_x(horilf * 0.55);

        next.foot_br.position = Vec3::new(
            s_a.feet_b.0,
            s_a.feet_b.1 + horirb * 3.0,
            s_a.feet_b.2 + vertrb * 5.0 * s_a.height - 0.5,
        );
        next.foot_br.orientation = Quaternion::rotation_x(horirb * 0.55);

        next.wing_in_l.position = Vec3::new(-s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);
        next.wing_in_l.orientation = Quaternion::rotation_y(0.8 + tilt * 1.0);

        next.wing_in_r.position = Vec3::new(s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);
        next.wing_in_r.orientation = Quaternion::rotation_y(-0.8 + tilt * 1.0);

        next.wing_out_l.position = Vec3::new(-s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
        next.wing_out_l.orientation = Quaternion::rotation_y(-2.0 + tilt * 1.0);

        next.wing_out_r.position = Vec3::new(s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
        next.wing_out_r.orientation = Quaternion::rotation_y(2.0 + tilt * 1.0);

        next
    }
}
