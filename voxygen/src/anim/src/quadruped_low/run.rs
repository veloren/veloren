use super::{super::Animation, QuadrupedLowSkeleton, SkeletonAttr};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (f32, f64);
    type Skeleton = QuadrupedLowSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_low_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_low_run")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, _global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab = 0.5;



        let center = (anim_time as f32 * lab as f32 + PI / 2.0).sin();
        let centeroffset = (anim_time as f32 * lab as f32 + PI * 1.5).sin();


        let short = (((5.0)
            / (3.6
                + 1.4 * ((anim_time as f32 *16.0* lab as f32+ PI * 0.25).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 *16.0* lab as f32+ PI * 0.25).sin());
        let shortalt = (anim_time as f32 *16.0* lab as f32 + PI * 0.25).sin();




        let foothoril = (((5.0)
            / (1.0
                + (4.0)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.45).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.45).sin());
        let foothorir = (((5.0)
            / (1.0
                + (4.0)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI *0.45).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.45).sin());
        let footvertl = (anim_time as f32 * 16.0 * lab as f32+PI*0.0).sin();
        let footvertr = (anim_time as f32 * 16.0 * lab as f32 + PI).sin();

        let footrotl = (((5.0)
            / (0.5
                + (4.5)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin());

        let footrotr = (((5.0)
            / (0.5
                + (4.5)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin());
//
        let foothorilb = (((5.0)
            / (1.0
                + (4.0)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI *1.25).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.25).sin());        
        let foothorirb = (((5.0)
            / (1.0
                + (4.0)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI *0.25).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.25).sin());   
        let footvertlb = (anim_time as f32 * 16.0 * lab as f32+PI*(-0.2)).sin();
        let footvertrb = (anim_time as f32 * 16.0 * lab as f32 + PI*0.8).sin();

        let footrotlb = (((5.0)
            / (0.5
                + (4.5)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.2).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.2).sin());

        let footrotrb = (((5.0)
            / (0.5
                + (4.5)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.2).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.2).sin());







        next.head_upper.offset =
            Vec3::new(0.0, skeleton_attr.head_upper.0, skeleton_attr.head_upper.1);
        next.head_upper.ori =
            Quaternion::rotation_x(0.0) * Quaternion::rotation_z(short*-0.06);
        next.head_upper.scale = Vec3::one();

        next.head_lower.offset =
            Vec3::new(0.0, skeleton_attr.head_lower.0, skeleton_attr.head_lower.1);
        next.head_lower.ori = Quaternion::rotation_x(0.0)* Quaternion::rotation_z(short*-0.15);
        next.head_lower.scale = Vec3::one();

        next.jaw.offset = Vec3::new(
            0.0,
            skeleton_attr.jaw.0,
            skeleton_attr.jaw.1,
        );
        next.jaw.ori = Quaternion::rotation_x(0.0);
        next.jaw.scale = Vec3::one()*0.98;

        next.tail_front.offset = Vec3::new(
            0.0,
            skeleton_attr.tail_front.0,
            skeleton_attr.tail_front.1,
        );
        next.tail_front.ori = Quaternion::rotation_z(shortalt*0.18)*Quaternion::rotation_y(shortalt*0.1)*Quaternion::rotation_x(0.06);
        next.tail_front.scale = Vec3::one();

        next.tail_rear.offset = Vec3::new(
            0.0,
            skeleton_attr.tail_rear.0,
            skeleton_attr.tail_rear.1 + centeroffset * 0.6,
        );
        next.tail_rear.ori = Quaternion::rotation_z(shortalt*0.25)*Quaternion::rotation_y(shortalt*0.1)*Quaternion::rotation_x(-0.04);
        next.tail_rear.scale = Vec3::one();

        next.chest.offset =
            Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1)/6.0;
        next.chest.ori = Quaternion::rotation_z(short*0.12)*Quaternion::rotation_y(shortalt*0.12);
        next.chest.scale = Vec3::one()/6.0;

        next.foot_fl.offset = Vec3::new(
            -skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1 + foothoril * -2.5,
            skeleton_attr.feet_f.2 + ((footvertl * -0.8).max(-0.0)),
        );
        next.foot_fl.ori = Quaternion::rotation_x(-0.2 + footrotl * -0.4)*Quaternion::rotation_z(footrotl * 0.55);
        next.foot_fl.scale = Vec3::one();

        next.foot_fr.offset = Vec3::new(
            skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1 + foothorir * -2.5,
            skeleton_attr.feet_f.2 + ((footvertr * -0.8).max(-0.0)),
        );
        next.foot_fr.ori = Quaternion::rotation_x(-0.2 + footrotr * -0.4)*Quaternion::rotation_z(footrotr * -0.55);
        next.foot_fr.scale = Vec3::one();

        next.foot_bl.offset = Vec3::new(
            -skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1 + foothorilb * -2.5,
            skeleton_attr.feet_b.2 + ((footvertlb * -0.6).max(-0.0)),
        );
        next.foot_bl.ori = Quaternion::rotation_x(-0.2 + footrotlb * -0.4)*Quaternion::rotation_z(footrotlb * 0.55);
        next.foot_bl.scale = Vec3::one();

        next.foot_br.offset = Vec3::new(
            skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1 + foothorirb * -2.5,
            skeleton_attr.feet_b.2 + ((footvertrb * -0.6).max(-0.0)),
        );
        next.foot_br.ori = Quaternion::rotation_x(-0.2 + footrotrb * -0.4)*Quaternion::rotation_z(footrotrb * -0.55);
        next.foot_br.scale = Vec3::one();

        next
    }
}
