use super::{super::Animation, BipedLargeSkeleton, SkeletonAttr};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct WieldAnimation;

impl Animation for WieldAnimation {
    type Dependency = (f32, f64);
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_wield\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_wield")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab = 0.55;
        let breathe = (anim_time as f32 + 1.5 * PI).sin();
        let test = (anim_time as f32 + 36.0 * PI).sin();

        let look = Vec2::new(
            ((global_time + anim_time) as f32 / 8.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.5,
            ((global_time + anim_time) as f32 / 8.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.25,
        );

        let foothoril = (((1.0)
            / (0.4
                + (0.6)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin());
        let foothorir = (((1.0)
            / (0.4
                + (0.6)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin());
        let footvertl = (anim_time as f32 * 16.0 * lab as f32).sin();
        let footvertr = (anim_time as f32 * 16.0 * lab as f32 + PI).sin();
        let handhoril = (anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin();
        let handhorir = (anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin();

        let footrotl = (((5.0)
            / (2.5
                + (2.5)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin());

        let footrotr = (((5.0)
            / (1.0
                + (4.0)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin());

        let short = (anim_time as f32 * lab as f32 * 16.0).sin();

        let shortalt = (anim_time as f32 * lab as f32 * 16.0 + PI / 2.0).sin();

        next.main.offset = Vec3::new(0.0, 0.0, 0.0);
        next.main.ori = Quaternion::rotation_x(0.0)
            * Quaternion::rotation_y(-1.57)
            * Quaternion::rotation_z(1.0);
        next.main.scale = Vec3::one() * 1.02;

        next.hand_l.offset = Vec3::new(
            -skeleton_attr.hand.0 - 7.0,
            skeleton_attr.hand.1 - 7.0,
            skeleton_attr.hand.2 + 10.0,
        );
        next.hand_l.ori = Quaternion::rotation_x(0.57) * Quaternion::rotation_z(1.57);
        next.hand_l.scale = Vec3::one() * 1.02;

        next.hand_r.offset = Vec3::new(
            skeleton_attr.hand.0 - 7.0,
            skeleton_attr.hand.1 - 7.0,
            skeleton_attr.hand.2 + 10.0,
        );
        next.hand_r.ori = Quaternion::rotation_x(0.57) * Quaternion::rotation_z(1.57);
        next.hand_r.scale = Vec3::one() * 1.02;

        if velocity < 0.5 {
            next.head.offset = Vec3::new(
                0.0,
                skeleton_attr.head.0,
                skeleton_attr.head.1 + breathe * 0.2,
            ) * 1.02;
            next.head.ori =
                Quaternion::rotation_z(look.x * 0.6) * Quaternion::rotation_x(look.y * 0.6);
            next.head.scale = Vec3::one() * 1.02;

            next.upper_torso.offset = Vec3::new(
                0.0,
                skeleton_attr.upper_torso.0,
                skeleton_attr.upper_torso.1 + breathe * 0.5,
            );
            next.upper_torso.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
            next.upper_torso.scale = Vec3::one();

            next.lower_torso.offset = Vec3::new(
                0.0,
                skeleton_attr.lower_torso.0,
                skeleton_attr.lower_torso.1 + breathe * 0.15,
            );
            next.lower_torso.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
            next.lower_torso.scale = Vec3::one() * 1.02;

            next.shoulder_l.offset = Vec3::new(
                -skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_l.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
            next.shoulder_l.scale = Vec3::one();

            next.shoulder_r.offset = Vec3::new(
                skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_r.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
            next.shoulder_r.scale = Vec3::one();

            next.leg_l.offset = Vec3::new(
                -skeleton_attr.leg.0,
                skeleton_attr.leg.1,
                skeleton_attr.leg.2 + breathe * 0.2,
            ) * 1.02;
            next.leg_l.ori = Quaternion::rotation_z(0.0);
            next.leg_l.scale = Vec3::one() * 1.02;

            next.leg_r.offset = Vec3::new(
                skeleton_attr.leg.0,
                skeleton_attr.leg.1,
                skeleton_attr.leg.2 + breathe * 0.2,
            ) * 1.02;
            next.leg_r.ori = Quaternion::rotation_z(0.0);
            next.leg_r.scale = Vec3::one() * 1.02;

            next.foot_l.offset = Vec3::new(
                -skeleton_attr.foot.0,
                skeleton_attr.foot.1,
                skeleton_attr.foot.2,
            ) / 8.0;
            next.foot_l.ori = Quaternion::rotation_z(0.0);
            next.foot_l.scale = Vec3::one() / 8.0;

            next.foot_r.offset = Vec3::new(
                skeleton_attr.foot.0,
                skeleton_attr.foot.1,
                skeleton_attr.foot.2,
            ) / 8.0;
            next.foot_r.ori = Quaternion::rotation_z(0.0);
            next.foot_r.scale = Vec3::one() / 8.0;

            next.torso.offset = Vec3::new(0.0, 0.0, 0.0) / 8.0;
            next.torso.ori = Quaternion::rotation_z(test * 0.0);
            next.torso.scale = Vec3::one() / 8.0;

            next.control.offset = Vec3::new(7.0, 9.0, -10.0);
            next.control.ori = Quaternion::rotation_x(test * 0.02)
                * Quaternion::rotation_y(test * 0.02)
                * Quaternion::rotation_z(test * 0.02);
            next.control.scale = Vec3::one();
        } else {
            next.head.offset = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1) * 1.02;
            next.head.ori = Quaternion::rotation_z(short * -0.18) * Quaternion::rotation_x(-0.05);
            next.head.scale = Vec3::one() * 1.02;

            next.upper_torso.offset = Vec3::new(
                0.0,
                skeleton_attr.upper_torso.0,
                skeleton_attr.upper_torso.1 + shortalt * -1.5,
            );
            next.upper_torso.ori = Quaternion::rotation_z(short * 0.18);
            next.upper_torso.scale = Vec3::one();

            next.lower_torso.offset = Vec3::new(
                0.0,
                skeleton_attr.lower_torso.0,
                skeleton_attr.lower_torso.1,
            );
            next.lower_torso.ori =
                Quaternion::rotation_z(short * 0.15) * Quaternion::rotation_x(0.14);
            next.lower_torso.scale = Vec3::one() * 1.02;

            next.shoulder_l.offset = Vec3::new(
                -skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1 + foothoril * -1.0,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_l.ori = Quaternion::rotation_x(0.5 + footrotl * -0.16)
                * Quaternion::rotation_y(0.1)
                * Quaternion::rotation_z(footrotl * 0.1);
            next.shoulder_l.scale = Vec3::one();

            next.shoulder_r.offset = Vec3::new(
                skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1 + foothorir * -1.0,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_r.ori = Quaternion::rotation_x(0.5 + footrotr * -0.16)
                * Quaternion::rotation_y(-0.1)
                * Quaternion::rotation_z(footrotr * -0.1);
            next.shoulder_r.scale = Vec3::one();

            next.leg_l.offset = Vec3::new(
                -skeleton_attr.leg.0,
                skeleton_attr.leg.1,
                skeleton_attr.leg.2,
            ) * 0.98;
            next.leg_l.ori =
                Quaternion::rotation_z(short * 0.18) * Quaternion::rotation_x(foothoril * 0.3);
            next.leg_l.scale = Vec3::one() * 0.98;

            next.leg_r.offset = Vec3::new(
                skeleton_attr.leg.0,
                skeleton_attr.leg.1,
                skeleton_attr.leg.2,
            ) * 0.98;

            next.leg_r.ori =
                Quaternion::rotation_z(short * 0.18) * Quaternion::rotation_x(foothorir * 0.3);
            next.leg_r.scale = Vec3::one() * 0.98;

            next.foot_l.offset = Vec3::new(
                -skeleton_attr.foot.0,
                4.0 + skeleton_attr.foot.1 + foothoril * 8.5,
                skeleton_attr.foot.2 + ((footvertl * 6.5).max(0.0)),
            ) / 8.0;
            next.foot_l.ori =
                Quaternion::rotation_x(-0.5 + footrotl * 0.85) * Quaternion::rotation_y(0.0);
            next.foot_l.scale = Vec3::one() / 8.0;

            next.foot_r.offset = Vec3::new(
                skeleton_attr.foot.0,
                4.0 + skeleton_attr.foot.1 + foothorir * 8.5,
                skeleton_attr.foot.2 + ((footvertr * 6.5).max(0.0)),
            ) / 8.0;
            next.foot_r.ori =
                Quaternion::rotation_x(-0.5 + footrotr * 0.85) * Quaternion::rotation_y(0.0);
            next.foot_r.scale = Vec3::one() / 8.0;

            next.torso.offset = Vec3::new(0.0, 0.0, 0.0) / 8.0;
            next.torso.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.25);
            next.torso.scale = Vec3::one() / 8.0;

            next.control.offset = Vec3::new(7.0, 9.0, -10.0);
            next.control.ori = Quaternion::rotation_x(test * 0.02)
                * Quaternion::rotation_y(test * 0.02)
                * Quaternion::rotation_z(test * 0.02);
            next.control.scale = Vec3::one();
        }

        next
    }
}
