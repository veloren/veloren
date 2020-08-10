use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use std::{f32::consts::PI, ops::Mul};

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (f32, f64);
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_run")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, _global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab = 0.55; //.65
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




            
        
        let foothoril2 = (((1.0)
            / (0.4
                + (0.6)
                    * ((anim_time as f32 * 16.0 * lab as f32 - PI * 0.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 - PI * 0.4).sin());
        let foothorir2 = (((1.0)
            / (0.4
                + (0.6)
                    * ((anim_time as f32 * 16.0 * lab as f32).cos()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32).cos());
        let footvertl2 = (anim_time as f32 * 16.0 * lab as f32 + 1.4 * PI).cos();
        let footvertr2 = (anim_time as f32 * 16.0 * lab as f32 + 1.4 * PI).sin();
        let handhoril2 = (anim_time as f32 * 16.0 * lab as f32).sin();
        let handhorir2 = (anim_time as f32 * 16.0 * lab as f32 + PI).sin();

        let footrotl2 = (((5.0)
            / (2.5
                + (2.5)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.1).cos()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.1).cos());

        let footrotr2 = (((5.0)
            / (2.5
                + (2.5)
                    * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin());


        
        let short = (anim_time as f32 * lab as f32 * 16.0).sin();

        let shortalt = (anim_time as f32 * lab as f32 * 16.0 + PI / 2.0).sin();

        if skeleton_attr.beast {
            next.head.position = Vec3::new(0.0, skeleton_attr.head.0 - 3.0, skeleton_attr.head.1 + 4.0) * 1.02;
            next.head.orientation = Quaternion::rotation_z(short * -0.18) * Quaternion::rotation_x(0.45 - short * 0.05);
            next.head.scale = Vec3::one() * 1.02;

            next.upper_torso.position = Vec3::new(
                0.0,
                skeleton_attr.upper_torso.0,
                skeleton_attr.upper_torso.1 + shortalt * -1.5,
            );
            next.upper_torso.orientation = Quaternion::rotation_x(-0.45) * Quaternion::rotation_z(short * 0.18);
            next.upper_torso.scale = Vec3::one();

            next.lower_torso.position = Vec3::new(
                0.0,
                skeleton_attr.lower_torso.0,
                skeleton_attr.lower_torso.1,
            );
            next.lower_torso.orientation = Quaternion::rotation_z(short * 0.15) * Quaternion::rotation_x(0.14);
            next.lower_torso.scale = Vec3::one() * 1.02;

            next.jaw.position = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
            next.jaw.orientation = Quaternion::rotation_x(0.0);
            next.jaw.scale = Vec3::one() * 1.02;

            next.tail.position = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
            next.tail.orientation =
                Quaternion::rotation_x(shortalt * 0.1 + 0.45);
            next.tail.scale = Vec3::one();

            next.second.position = Vec3::new(0.0, 0.0, 0.0);
            next.second.orientation =
                Quaternion::rotation_x(PI) * Quaternion::rotation_y(0.0) * Quaternion::rotation_z(0.0);
            next.second.scale = Vec3::one() * 0.0;

            next.control.position = Vec3::new(0.0, 0.0, 0.0);
            next.control.orientation = Quaternion::rotation_z(0.0);
            next.control.scale = Vec3::one();

            next.main.position = Vec3::new(-5.0, -7.0, 7.0);
            next.main.orientation =
                Quaternion::rotation_x(PI) * Quaternion::rotation_y(0.6) * Quaternion::rotation_z(1.57);
            next.main.scale = Vec3::one() * 1.02;

            next.shoulder_l.position = Vec3::new(
                -skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1 + foothoril2 * -3.0,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_l.orientation = Quaternion::rotation_x(footrotl2 * -0.36 + 0.45)
                * Quaternion::rotation_y(0.1)
                * Quaternion::rotation_z(footrotl2 * -0.3);
            next.shoulder_l.scale = Vec3::one();

            next.shoulder_r.position = Vec3::new(
                skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1 + foothorir2 * -3.0,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_r.orientation = Quaternion::rotation_x(footrotr2 * -0.36 + 0.45)
                * Quaternion::rotation_y(-0.1)
                * Quaternion::rotation_z(footrotr2 * -0.3);
            next.shoulder_r.scale = Vec3::one();

            next.hand_l.position = Vec3::new(
                -skeleton_attr.hand.0,
                skeleton_attr.hand.1 + foothoril2 * -4.0 + 2.0,
                skeleton_attr.hand.2 + foothoril2 * 0.5 + 2.0,
            );
            next.hand_l.orientation = Quaternion::rotation_x(0.15 + (handhoril2 * -1.2).max(-0.3) + 0.45)
                * Quaternion::rotation_y(handhoril2 * 0.1);
            next.hand_l.scale = Vec3::one() * 1.02;

            next.hand_r.position = Vec3::new(
                skeleton_attr.hand.0,
                skeleton_attr.hand.1 + foothorir2 * -4.0 + 2.0,
                skeleton_attr.hand.2 + foothorir2 * 0.5 + 2.0,
            );
            next.hand_r.orientation = Quaternion::rotation_x(0.15 + (handhorir2 * -1.2).max(-0.3) + 0.45)
                * Quaternion::rotation_y(handhorir2 * -0.1);
            next.hand_r.scale = Vec3::one() * 1.02;

            next.leg_l.position = Vec3::new(
                -skeleton_attr.leg.0,
                skeleton_attr.leg.1,
                skeleton_attr.leg.2,
            ) * 0.98;
            next.leg_l.orientation =
                Quaternion::rotation_z(short * 0.18) * Quaternion::rotation_x(foothoril2 * 0.6 - 0.45);
            next.leg_l.scale = Vec3::one() * 0.98;

            next.leg_r.position = Vec3::new(
                skeleton_attr.leg.0,
                skeleton_attr.leg.1,
                skeleton_attr.leg.2,
            ) * 0.98;
            next.leg_r.orientation =
                Quaternion::rotation_z(short * 0.18) * Quaternion::rotation_x(foothorir2 * 0.6 - 0.45);
            next.leg_r.scale = Vec3::one() * 0.98;

            next.foot_l.position = Vec3::new(
                -skeleton_attr.foot.0,
                4.0 + skeleton_attr.foot.1 + foothoril2 * 4.0 - 11.0,
                skeleton_attr.foot.2 + ((footvertl2 * 6.0).max(0.0)),
            ) / 8.0;
            next.foot_l.orientation =
                Quaternion::rotation_x(-0.5 + footrotl2 * 0.85);
            next.foot_l.scale = Vec3::one() / 8.0;

            next.foot_r.position = Vec3::new(
                skeleton_attr.foot.0,
                4.0 + skeleton_attr.foot.1 + foothorir2 * 4.0 - 11.0,
                skeleton_attr.foot.2 + ((footvertr2 * 6.0).max(0.0)),
            ) / 8.0;
            next.foot_r.orientation =
                Quaternion::rotation_x(-0.5 + footrotr2 * 0.85);
            next.foot_r.scale = Vec3::one() / 8.0;

            next.torso.position = Vec3::new(0.0, 0.0, 0.0) / 8.0;
            next.torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.25);
            next.torso.scale = Vec3::one() / 8.0;
        } else {
            next.head.position = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1) * 1.02;
            next.head.orientation = Quaternion::rotation_z(short * -0.18) * Quaternion::rotation_x(-0.05);
            next.head.scale = Vec3::one() * 1.02;
    
            next.upper_torso.position = Vec3::new(
                0.0,
                skeleton_attr.upper_torso.0,
                skeleton_attr.upper_torso.1 + shortalt * -1.5,
            );
            next.upper_torso.orientation = Quaternion::rotation_z(short * 0.18);
            next.upper_torso.scale = Vec3::one();
    
            next.lower_torso.position = Vec3::new(
                0.0,
                skeleton_attr.lower_torso.0,
                skeleton_attr.lower_torso.1,
            );
            next.lower_torso.orientation = Quaternion::rotation_z(short * 0.15) * Quaternion::rotation_x(0.14);
            next.lower_torso.scale = Vec3::one() * 1.02;
    
            next.jaw.position = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
            next.jaw.orientation = Quaternion::rotation_x(0.0);
            next.jaw.scale = Vec3::one() * 1.02;
    
            next.tail.position = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
            next.tail.orientation =
                Quaternion::rotation_x(shortalt * 0.3);
            next.tail.scale = Vec3::one();
    
            next.second.position = Vec3::new(0.0, 0.0, 0.0);
            next.second.orientation =
                Quaternion::rotation_x(PI) * Quaternion::rotation_y(0.0) * Quaternion::rotation_z(0.0);
            next.second.scale = Vec3::one() * 0.0;
    
            next.control.position = Vec3::new(0.0, 0.0, 0.0);
            next.control.orientation = Quaternion::rotation_z(0.0);
            next.control.scale = Vec3::one();
    
            next.main.position = Vec3::new(-5.0, -7.0, 7.0);
            next.main.orientation =
                Quaternion::rotation_x(PI) * Quaternion::rotation_y(0.6) * Quaternion::rotation_z(1.57);
            next.main.scale = Vec3::one() * 1.02;
    
            next.shoulder_l.position = Vec3::new(
                -skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1 + foothoril * -3.0,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_l.orientation = Quaternion::rotation_x(footrotl * -0.36)
                * Quaternion::rotation_y(0.1)
                * Quaternion::rotation_z(footrotl * 0.3);
            next.shoulder_l.scale = Vec3::one();
    
            next.shoulder_r.position = Vec3::new(
                skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1 + foothorir * -3.0,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_r.orientation = Quaternion::rotation_x(footrotr * -0.36)
                * Quaternion::rotation_y(-0.1)
                * Quaternion::rotation_z(footrotr * -0.3);
            next.shoulder_r.scale = Vec3::one();
    
            next.hand_l.position = Vec3::new(
                -1.0 + -skeleton_attr.hand.0,
                skeleton_attr.hand.1 + foothoril * -4.0,
                skeleton_attr.hand.2 + foothoril * 1.0,
            );
            next.hand_l.orientation = Quaternion::rotation_x(0.15 + (handhoril * -1.2).max(-0.3))
                * Quaternion::rotation_y(handhoril * -0.1);
            next.hand_l.scale = Vec3::one() * 1.02;
    
            next.hand_r.position = Vec3::new(
                1.0 + skeleton_attr.hand.0,
                skeleton_attr.hand.1 + foothorir * -4.0,
                skeleton_attr.hand.2 + foothorir * 1.0,
            );
            next.hand_r.orientation = Quaternion::rotation_x(0.15 + (handhorir * -1.2).max(-0.3))
                * Quaternion::rotation_y(handhorir * 0.1);
            next.hand_r.scale = Vec3::one() * 1.02;
    
            next.leg_l.position = Vec3::new(
                -skeleton_attr.leg.0,
                skeleton_attr.leg.1,
                skeleton_attr.leg.2,
            ) * 0.98;
            next.leg_l.orientation =
                Quaternion::rotation_z(short * 0.18) * Quaternion::rotation_x(foothoril * 0.3);
            next.leg_l.scale = Vec3::one() * 0.98;
    
            next.leg_r.position = Vec3::new(
                skeleton_attr.leg.0,
                skeleton_attr.leg.1,
                skeleton_attr.leg.2,
            ) * 0.98;
    
            next.leg_r.orientation =
                Quaternion::rotation_z(short * 0.18) * Quaternion::rotation_x(foothorir * 0.3);
            next.leg_r.scale = Vec3::one() * 0.98;
    
            next.foot_l.position = Vec3::new(
                -skeleton_attr.foot.0,
                4.0 + skeleton_attr.foot.1 + foothoril * 8.5,
                skeleton_attr.foot.2 + ((footvertl * 6.5).max(0.0)),
            ) / 8.0;
            next.foot_l.orientation =
                Quaternion::rotation_x(-0.5 + footrotl * 0.85) * Quaternion::rotation_y(0.0);
            next.foot_l.scale = Vec3::one() / 8.0;
    
            next.foot_r.position = Vec3::new(
                skeleton_attr.foot.0,
                4.0 + skeleton_attr.foot.1 + foothorir * 8.5,
                skeleton_attr.foot.2 + ((footvertr * 6.5).max(0.0)),
            ) / 8.0;
            next.foot_r.orientation =
                Quaternion::rotation_x(-0.5 + footrotr * 0.85) * Quaternion::rotation_y(0.0);
            next.foot_r.scale = Vec3::one() / 8.0;
    
            next.torso.position = Vec3::new(0.0, 0.0, 0.0) / 8.0;
            next.torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.25);
            next.torso.scale = Vec3::one() / 8.0;
        }
        next
    }
}
