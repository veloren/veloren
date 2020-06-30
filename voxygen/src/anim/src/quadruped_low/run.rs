use super::{super::Animation, QuadrupedLowSkeleton, SkeletonAttr};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (f32, Vec3<f32>, Vec3<f32>, f64);
    type Skeleton = QuadrupedLowSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_low_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_low_run")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, orientation, last_ori, _global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab = 0.7 * (skeleton_attr.lean.0 + 1.0)*(1.0/skeleton_attr.scaler);

        let center = (anim_time as f32 * lab as f32 + PI / 2.0).sin();
        let centeroffset = (anim_time as f32 * lab as f32 + PI * 1.5).sin();

        let short = (((1.0)
            / (0.72
                + 0.28
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.25).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.25).sin());
        let shortalt = (anim_time as f32 * 16.0 * lab as f32 + PI * 0.25).sin();

        let foothoril = (((1.0)
            / (0.4
                + (0.6)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.45).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.45).sin());
        let foothorir = (((1.0)
            / (0.4
                + (0.6)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.45).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.45).sin());
        let footvertl = (anim_time as f32 * 16.0 * lab as f32 + PI * 0.0).sin();
        let footvertr = (anim_time as f32 * 16.0 * lab as f32 + PI).sin();

        let footrotl = (((1.0)
            / (0.4
                + (0.6)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin());

        let footrotr = (((1.0)
            / (0.4
                + (0.6)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin());
        //
        let foothorilb = (((1.0)
            / (0.4
                + (0.6)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.25).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.25).sin());
        let foothorirb = (((1.0)
            / (0.4
                + (0.6)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.25).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.25).sin());
        let footvertlb = (anim_time as f32 * 16.0 * lab as f32 + PI * (-0.2)).sin();
        let footvertrb = (anim_time as f32 * 16.0 * lab as f32 + PI * 0.8).sin();

        let footrotlb = (((1.0)
            / (0.4
                + (0.6)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.2).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.2).sin());

        let footrotrb = (((1.0)
            / (0.4
                + (0.6)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.2).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.2).sin());
        let ori = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if Vec2::new(ori, last_ori)
            .map(|o| Vec2::<f32>::from(o).magnitude_squared())
            .map(|m| m > 0.001 && m.is_finite())
            .reduce_and()
            && ori.angle_between(last_ori).is_finite()
        {
            ori.angle_between(last_ori).min(0.2)
                * last_ori.determine_side(Vec2::zero(), ori).signum()
        } else {
            0.0
        } * 1.3;
//let tilt = 0.0;
        next.head_upper.offset =
            Vec3::new(0.0, skeleton_attr.head_upper.0, skeleton_attr.head_upper.1);
        next.head_upper.ori =
            Quaternion::rotation_x(-skeleton_attr.lean.0) * Quaternion::rotation_z(short * -0.06 + tilt * -1.5);
        next.head_upper.scale = Vec3::one();

        next.head_lower.offset =
            Vec3::new(0.0, skeleton_attr.head_lower.0, skeleton_attr.head_lower.1);
        next.head_lower.ori = Quaternion::rotation_x(0.0) * Quaternion::rotation_z(short * -0.15 + tilt * -0.8);
        next.head_lower.scale = Vec3::one();

        next.jaw.offset = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
        next.jaw.ori = Quaternion::rotation_x(0.0);
        next.jaw.scale = Vec3::one() * 0.98;

        next.tail_front.offset = Vec3::new(
            0.0,
            skeleton_attr.tail_front.0 + skeleton_attr.lean.0 * 2.0,
            skeleton_attr.tail_front.1 + skeleton_attr.lean.0 * 2.0,
        );
        next.tail_front.ori = Quaternion::rotation_z(shortalt * 0.18 + tilt * 1.8)
            * Quaternion::rotation_y(shortalt * 0.1)
            * Quaternion::rotation_x(0.06 - skeleton_attr.lean.0 * 1.2);
        next.tail_front.scale = Vec3::one();

        next.tail_rear.offset = Vec3::new(
            0.0,
            skeleton_attr.tail_rear.0,
            skeleton_attr.tail_rear.1 + centeroffset * 0.6,
        );
        next.tail_rear.ori = Quaternion::rotation_z(shortalt * 0.25 + tilt * 1.6)
            * Quaternion::rotation_y(shortalt * 0.1)
            * Quaternion::rotation_x(-0.04);
        next.tail_rear.scale = Vec3::one();

        next.chest.offset = Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1) *skeleton_attr.scaler/11.0;
        next.chest.ori = Quaternion::rotation_z(short * 0.13 + tilt * -1.9)
            * Quaternion::rotation_y(shortalt * 0.12)
            * Quaternion::rotation_x(skeleton_attr.lean.0);
        next.chest.scale = Vec3::one() *skeleton_attr.scaler/11.0;

        next.foot_fl.offset = Vec3::new(
            -skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1 + foothoril * -1.0,
            skeleton_attr.feet_f.2 + 1.0 + ((footvertl * -0.8).max(-0.0)),
        );
        next.foot_fl.ori =
            Quaternion::rotation_x(footrotl * -0.25 * skeleton_attr.lean.1 - skeleton_attr.lean.0)
                * Quaternion::rotation_z(footrotl * 0.4 * skeleton_attr.lean.1);
        next.foot_fl.scale = Vec3::one();

        next.foot_fr.offset = Vec3::new(
            skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1 + foothorir * -1.0,
            skeleton_attr.feet_f.2 + 1.0 + ((footvertr * -0.8).max(-0.0)),
        );
        next.foot_fr.ori =
            Quaternion::rotation_x(footrotr * -0.25 * skeleton_attr.lean.1 - skeleton_attr.lean.0)
                * Quaternion::rotation_z(footrotr * -0.4 * skeleton_attr.lean.1 + tilt * 3.5);
        next.foot_fr.scale = Vec3::one();

        next.foot_bl.offset = Vec3::new(
            -skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1 + foothorilb * -1.0,
            skeleton_attr.feet_b.2 + 1.0 + ((footvertlb * -0.6).max(-0.0)),
        );
        next.foot_bl.ori = Quaternion::rotation_x(footrotlb * -0.25 - skeleton_attr.lean.0)
            * Quaternion::rotation_z(footrotlb * 0.4);
        next.foot_bl.scale = Vec3::one();

        next.foot_br.offset = Vec3::new(
            skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1 + foothorirb * -1.0,
            skeleton_attr.feet_b.2 + 1.0 + ((footvertrb * -0.6).max(-0.0)),
        );
        next.foot_br.ori = Quaternion::rotation_x(footrotrb * -0.25 - skeleton_attr.lean.0)
            * Quaternion::rotation_z(footrotrb * -0.4);
        next.foot_br.scale = Vec3::one();

        next
    }
}
