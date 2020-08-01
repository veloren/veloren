use super::{super::Animation, DragonSkeleton, SkeletonAttr};
use std::f32::consts::PI;
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (f32, Vec3<f32>, Vec3<f32>, f64, Vec3<f32>);
    type Skeleton = DragonSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"dragon_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "dragon_run")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, orientation, last_ori, _global_time, avg_vel): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab = 0.6; //6

        let short = (((1.0)
            / (0.72
                + 0.28
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.0).sin());

        //

        let shortalt = (anim_time as f32 * 16.0 * lab as f32 + PI * 0.5).sin();

        //
        let ori: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if Vec2::new(ori, last_ori)
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

        let lab = 14;

        let wave_ultra_slow_cos = (anim_time as f32 * 3.0 + PI).cos();
        let wave_slow = (anim_time as f32 * 4.5).sin();

        let vertlf = (anim_time as f32 * lab as f32 + PI * 1.8).sin().max(0.15);
        let vertrfoffset = (anim_time as f32 * lab as f32 + PI * 0.80).sin().max(0.15);
        let vertlboffset = (anim_time as f32 * lab as f32).sin().max(0.15);
        let vertrb = (anim_time as f32 * lab as f32 + PI).sin().max(0.15);

        let horilf = (anim_time as f32 * lab as f32 + PI * 1.2).sin();
        let horirfoffset = (anim_time as f32 * lab as f32 + PI * 0.20).sin();
        let horilboffset = (anim_time as f32 * lab as f32 + PI * 1.4).sin();
        let horirb = (anim_time as f32 * lab as f32 + PI * 0.4).sin();

        let center = (anim_time as f32 * lab as f32 + PI / 2.0).sin();
        let centeroffset = (anim_time as f32 * lab as f32 + PI * 1.5).sin();

        next.head_upper.offset =
            Vec3::new(0.0, skeleton_attr.head_upper.0, skeleton_attr.head_upper.1);
        next.head_upper.ori = Quaternion::rotation_x(short * -0.03 - 0.1)
            * Quaternion::rotation_z(tilt * -1.2)
            * Quaternion::rotation_y(tilt * 0.8);
        next.head_upper.scale = Vec3::one();

        next.head_lower.offset =
            Vec3::new(0.0, skeleton_attr.head_lower.0, skeleton_attr.head_lower.1);
        next.head_lower.ori = Quaternion::rotation_z(tilt * -0.8)
            * Quaternion::rotation_x(short * -0.05)
            * Quaternion::rotation_y(tilt * 0.3);
        next.head_lower.scale = Vec3::one() * 1.02;

        next.jaw.offset = Vec3::new(
            0.0,
            skeleton_attr.jaw.0 - wave_ultra_slow_cos * 0.12,
            skeleton_attr.jaw.1 + wave_slow * 0.2,
        );
        next.jaw.ori = Quaternion::rotation_x(wave_slow * 0.03);
        next.jaw.scale = Vec3::one() * 1.05;

        next.tail_front.offset = Vec3::new(
            0.0,
            skeleton_attr.tail_front.0,
            skeleton_attr.tail_front.1 + centeroffset * 0.6,
        );
        next.tail_front.ori =
            Quaternion::rotation_x(center * 0.03) * Quaternion::rotation_z(tilt * 1.5);
        next.tail_front.scale = Vec3::one() * 0.98;

        next.tail_rear.offset = Vec3::new(
            0.0,
            skeleton_attr.tail_rear.0,
            skeleton_attr.tail_rear.1 + centeroffset * 0.6,
        );
        next.tail_rear.ori =
            Quaternion::rotation_x(center * 0.03) * Quaternion::rotation_z(tilt * 1.5);
        next.tail_rear.scale = Vec3::one() * 0.98;

        next.chest_front.offset = Vec3::new(
            0.0,
            skeleton_attr.chest_front.0,
            skeleton_attr.chest_front.1 + shortalt * 2.5 + x_tilt * 10.0,
        );
        next.chest_front.ori = Quaternion::rotation_x(short * 0.13 + x_tilt)
            * Quaternion::rotation_y(0.0)
            * Quaternion::rotation_z(tilt * -1.5);
        next.chest_front.scale = Vec3::one();

        next.chest_rear.offset = Vec3::new(
            0.0,
            skeleton_attr.chest_rear.0,
            skeleton_attr.chest_rear.1 + shortalt * 0.2,
        );
        next.chest_rear.ori = Quaternion::rotation_x(short * 0.1)
            * Quaternion::rotation_y(0.0)
            * Quaternion::rotation_z(tilt * 1.8);
        next.chest_rear.scale = Vec3::one();

        next.foot_fl.offset = Vec3::new(
            -skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1 + horilf * 2.5,
            skeleton_attr.feet_f.2 + vertlf * 5.0 * skeleton_attr.height - 0.5,
        );
        next.foot_fl.ori = Quaternion::rotation_x(horilf * 0.6);
        next.foot_fl.scale = Vec3::one();

        next.foot_fr.offset = Vec3::new(
            skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1 + horirfoffset * 2.5,
            skeleton_attr.feet_f.2 + vertrfoffset * 5.0 * skeleton_attr.height - 0.5,
        );
        next.foot_fr.ori = Quaternion::rotation_x(horirb * 0.6);
        next.foot_fr.scale = Vec3::one();

        next.foot_bl.offset = Vec3::new(
            -skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1 + horilboffset * 3.0,
            skeleton_attr.feet_b.2 + vertlboffset * 5.0 * skeleton_attr.height - 0.5,
        );
        next.foot_bl.ori = Quaternion::rotation_x(horilf * 0.55);
        next.foot_bl.scale = Vec3::one();

        next.foot_br.offset = Vec3::new(
            skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1 + horirb * 3.0,
            skeleton_attr.feet_b.2 + vertrb * 5.0 * skeleton_attr.height - 0.5,
        );
        next.foot_br.ori = Quaternion::rotation_x(horirb * 0.55);
        next.foot_br.scale = Vec3::one();

        next.wing_in_l.offset = Vec3::new(
            -skeleton_attr.wing_in.0,
            skeleton_attr.wing_in.1,
            skeleton_attr.wing_in.2,
        );
        next.wing_in_l.ori = Quaternion::rotation_y(0.8 + tilt * 1.0);
        next.wing_in_l.scale = Vec3::one();

        next.wing_in_r.offset = Vec3::new(
            skeleton_attr.wing_in.0,
            skeleton_attr.wing_in.1,
            skeleton_attr.wing_in.2,
        );
        next.wing_in_r.ori = Quaternion::rotation_y(-0.8 + tilt * 1.0);
        next.wing_in_r.scale = Vec3::one();

        next.wing_out_l.offset = Vec3::new(
            -skeleton_attr.wing_out.0,
            skeleton_attr.wing_out.1,
            skeleton_attr.wing_out.2,
        );
        next.wing_out_l.ori = Quaternion::rotation_y(-2.0 + tilt * 1.0);
        next.wing_out_l.scale = Vec3::one();

        next.wing_out_r.offset = Vec3::new(
            skeleton_attr.wing_out.0,
            skeleton_attr.wing_out.1,
            skeleton_attr.wing_out.2,
        );
        next.wing_out_r.ori = Quaternion::rotation_y(2.0 + tilt * 1.0);
        next.wing_out_r.scale = Vec3::one();

        next
    }
}
