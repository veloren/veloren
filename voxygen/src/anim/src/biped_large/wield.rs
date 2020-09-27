use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use std::{f32::consts::PI, ops::Mul};

pub struct WieldAnimation;

impl Animation for WieldAnimation {
    type Dependency = (Option<ToolKind>, Option<ToolKind>, f32, f64);
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_wield\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_wield")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, velocity, global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab = 0.55;
        let breathe = (anim_time as f32 + 1.5 * PI).sin();
        let test = (anim_time as f32 + 36.0 * PI).sin();

        let slower = (anim_time as f32 * 1.0 + PI).sin();
        let slow = (anim_time as f32 * 3.5 + PI).sin();
        let u_slow = (anim_time as f32 * 1.0 + PI).sin();
        let u_slowalt = (anim_time as f32 * 3.0 + PI).cos();
        
        let tailmove = Vec2::new(
            ((global_time + anim_time) as f32 / 2.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.25,
            ((global_time + anim_time) as f32 / 2.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.125,
        );

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

        next.hand_l.position = Vec3::new(
            -skeleton_attr.hand.0 - 7.0,
            skeleton_attr.hand.1 - 7.0,
            skeleton_attr.hand.2 + 10.0,
        );
        next.hand_l.orientation = Quaternion::rotation_x(0.57) * Quaternion::rotation_z(1.57);
        next.hand_l.scale = Vec3::one() * 1.02;

        next.hand_r.position = Vec3::new(
            skeleton_attr.hand.0 - 7.0,
            skeleton_attr.hand.1 - 7.0,
            skeleton_attr.hand.2 + 10.0,
        );
        next.hand_r.orientation = Quaternion::rotation_x(0.57) * Quaternion::rotation_z(1.57);
        next.hand_r.scale = Vec3::one() * 1.02;
        next.hand_r.orientation = Quaternion::rotation_x(0.57) * Quaternion::rotation_z(1.57);
        next.hand_r.scale = Vec3::one() * 1.02;
        
        next.control.position = Vec3::new(7.0, 9.0, -10.0);
        next.control.orientation = Quaternion::rotation_x(test * 0.02)
            * Quaternion::rotation_y(test * 0.02)
            * Quaternion::rotation_z(test * 0.02);
        next.control.scale = Vec3::one();

        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_x(0.0)
            * Quaternion::rotation_y(-1.57)
            * Quaternion::rotation_z(1.0);
        next.main.scale = Vec3::one() * 1.02;

        if velocity > 0.5 {
            next.head.position = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1) * 1.02;
            next.head.orientation =
                Quaternion::rotation_z(short * -0.18) * Quaternion::rotation_x(-0.05);
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
            next.lower_torso.orientation =
                Quaternion::rotation_z(short * 0.15) * Quaternion::rotation_x(0.14);
            next.lower_torso.scale = Vec3::one() * 1.02;

            next.jaw.position = Vec3::new(
                0.0,
                skeleton_attr.jaw.0 - slower * 0.12,
                skeleton_attr.jaw.1 + slow * 0.2,
            );
            next.jaw.orientation = Quaternion::rotation_x(slow * 0.05);
            next.jaw.scale = Vec3::one() * 1.02;
    
            next.tail.position = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
            next.tail.orientation =
                Quaternion::rotation_z(0.0 + slow * 0.2 + tailmove.x) * Quaternion::rotation_x(0.0);
            next.tail.scale = Vec3::one();

            next.shoulder_l.position = Vec3::new(
                -skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1 + foothoril * -1.0,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_l.orientation = Quaternion::rotation_x(0.5 + footrotl * -0.16)
                * Quaternion::rotation_y(0.1)
                * Quaternion::rotation_z(footrotl * 0.1);
            next.shoulder_l.scale = Vec3::one();

            next.shoulder_r.position = Vec3::new(
                skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1 + foothorir * -1.0,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_r.orientation = Quaternion::rotation_x(0.5 + footrotr * -0.16)
                * Quaternion::rotation_y(-0.1)
                * Quaternion::rotation_z(footrotr * -0.1);
            next.shoulder_r.scale = Vec3::one();

            next.torso.position = Vec3::new(0.0, 0.0, 0.0) / 8.0;
            next.torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.25);
            next.torso.scale = Vec3::one() / 8.0;
        } else {
            next.head.position = Vec3::new(
                0.0,
                skeleton_attr.head.0,
                skeleton_attr.head.1 + breathe * 0.2,
            ) * 1.02;
            next.head.orientation =
                Quaternion::rotation_z(look.x * 0.6) * Quaternion::rotation_x(look.y * 0.6);
            next.head.scale = Vec3::one() * 1.02;

            next.upper_torso.position = Vec3::new(
                0.0,
                skeleton_attr.upper_torso.0,
                skeleton_attr.upper_torso.1 + breathe * 0.5,
            );
            next.upper_torso.orientation =
                Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
            next.upper_torso.scale = Vec3::one();

            next.lower_torso.position = Vec3::new(
                0.0,
                skeleton_attr.lower_torso.0,
                skeleton_attr.lower_torso.1 + breathe * 0.15,
            );
            next.lower_torso.orientation =
                Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
            next.lower_torso.scale = Vec3::one() * 1.02;

            next.jaw.position = Vec3::new(
                0.0,
                skeleton_attr.jaw.0 - slower * 0.12,
                skeleton_attr.jaw.1 + slow * 0.2,
            );
            next.jaw.orientation = Quaternion::rotation_x(slow * 0.05);
            next.jaw.scale = Vec3::one() * 1.02;
    
            next.tail.position = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
            next.tail.orientation =
                Quaternion::rotation_z(0.0 + slow * 0.2 + tailmove.x) * Quaternion::rotation_x(0.0);
            next.tail.scale = Vec3::one();

            next.shoulder_l.position = Vec3::new(
                -skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
            next.shoulder_l.scale = Vec3::one();

            next.shoulder_r.position = Vec3::new(
                skeleton_attr.shoulder.0,
                skeleton_attr.shoulder.1,
                skeleton_attr.shoulder.2,
            );
            next.shoulder_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
            next.shoulder_r.scale = Vec3::one();

            next.leg_l.position = Vec3::new(
                -skeleton_attr.leg.0,
                skeleton_attr.leg.1,
                skeleton_attr.leg.2 + breathe * 0.2,
            ) * 1.02;
            next.leg_l.orientation = Quaternion::rotation_z(0.0);
            next.leg_l.scale = Vec3::one() * 1.02;

            next.leg_r.position = Vec3::new(
                skeleton_attr.leg.0,
                skeleton_attr.leg.1,
                skeleton_attr.leg.2 + breathe * 0.2,
            ) * 1.02;
            next.leg_r.orientation = Quaternion::rotation_z(0.0);
            next.leg_r.scale = Vec3::one() * 1.02;

            next.foot_l.position = Vec3::new(
                -skeleton_attr.foot.0,
                skeleton_attr.foot.1,
                skeleton_attr.foot.2,
            ) / 8.0;
            next.foot_l.orientation = Quaternion::rotation_z(0.0);
            next.foot_l.scale = Vec3::one() / 8.0;

            next.foot_r.position = Vec3::new(
                skeleton_attr.foot.0,
                skeleton_attr.foot.1,
                skeleton_attr.foot.2,
            ) / 8.0;
            next.foot_r.orientation = Quaternion::rotation_z(0.0);
            next.foot_r.scale = Vec3::one() / 8.0;

            next.torso.position = Vec3::new(0.0, 0.0, 0.0) / 8.0;
            next.torso.orientation = Quaternion::rotation_z(test * 0.0);
            next.torso.scale = Vec3::one() / 8.0;

            next.control.position = Vec3::new(7.0, 9.0, -10.0);
            next.control.orientation = Quaternion::rotation_x(test * 0.02)
                * Quaternion::rotation_y(test * 0.02)
                * Quaternion::rotation_z(test * 0.02);
            next.control.scale = Vec3::one();
        }
        match active_tool_kind {
            Some(ToolKind::Bow(_)) => {
                next.hand_l.position = Vec3::new(2.0, 1.5, 0.0);
                next.hand_l.orientation = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6)
                    * Quaternion::rotation_z(-0.3);
                next.hand_l.scale = Vec3::one() * 1.05;
                next.hand_r.position = Vec3::new(5.9, 4.5, -5.0);
                next.hand_r.orientation = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6)
                    * Quaternion::rotation_z(-0.3);
                next.hand_r.scale = Vec3::one() * 1.05;
                next.main.position = Vec3::new(8.0, 8.0, -13.0);
                next.main.orientation = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.3)
                    * Quaternion::rotation_z(-0.6);

                next.hold.position = Vec3::new(1.2, -1.0, -5.2);
                next.hold.orientation = Quaternion::rotation_x(-1.7)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(-0.1);
                next.hold.scale = Vec3::one() * 1.0;

                next.control.position = Vec3::new(-7.0, 3.0, -6.0);
                next.control.orientation =
                    Quaternion::rotation_x(u_slow * 0.2) * Quaternion::rotation_z(u_slowalt * 0.1);
                next.control.scale = Vec3::one();
            },
            Some(ToolKind::Hammer(_)) => {
                next.hand_l.position = Vec3::new(
                    -skeleton_attr.hand.0 - 7.0,
                    skeleton_attr.hand.1 - 7.0,
                    skeleton_attr.hand.2 + 10.0,
                );
                next.hand_l.orientation = Quaternion::rotation_x(0.57) * Quaternion::rotation_z(1.57);
                next.hand_l.scale = Vec3::one() * 1.02;
        
                next.hand_r.position = Vec3::new(
                    skeleton_attr.hand.0 - 7.0,
                    skeleton_attr.hand.1 - 7.0,
                    skeleton_attr.hand.2 + 10.0,
                );
                next.hand_r.orientation = Quaternion::rotation_x(0.57) * Quaternion::rotation_z(1.57);
                next.hand_r.scale = Vec3::one() * 1.02;
                
                next.control.position = Vec3::new(7.0, 9.0, -10.0);
                next.control.orientation = Quaternion::rotation_x(test * 0.02)
                    * Quaternion::rotation_y(test * 0.02)
                    * Quaternion::rotation_z(test * 0.02);
                next.control.scale = Vec3::one();

                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(-1.57)
                    * Quaternion::rotation_z(1.0);
                next.main.scale = Vec3::one() * 1.02;
            },
            Some(ToolKind::Staff(_)) => {
                next.hand_l.position = Vec3::new(
                    -skeleton_attr.hand.0 - 7.0,
                    skeleton_attr.hand.1 - 7.0,
                    skeleton_attr.hand.2 + 10.0,
                );
                next.hand_l.orientation = Quaternion::rotation_x(0.57) * Quaternion::rotation_z(1.57);
                next.hand_l.scale = Vec3::one() * 1.02;
        
                next.hand_r.position = Vec3::new(
                    skeleton_attr.hand.0 - 7.0,
                    skeleton_attr.hand.1 - 7.0,
                    skeleton_attr.hand.2 + 10.0,
                );
                next.hand_r.orientation = Quaternion::rotation_x(0.57) * Quaternion::rotation_z(1.57);
                next.hand_r.scale = Vec3::one() * 1.02;
                next.hand_r.orientation = Quaternion::rotation_x(0.57) * Quaternion::rotation_z(1.57);
                next.hand_r.scale = Vec3::one() * 1.02;
                
                next.control.position = Vec3::new(7.0, 9.0, -10.0);
                next.control.orientation = Quaternion::rotation_x(test * 0.02)
                    * Quaternion::rotation_y(test * 0.02)
                    * Quaternion::rotation_z(test * 0.02);
                next.control.scale = Vec3::one();

                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(-1.57)
                    * Quaternion::rotation_z(1.0);
                next.main.scale = Vec3::one() * 1.02;
            },
            Some(ToolKind::Sword(_)) => {
                next.hand_l.position = Vec3::new(
                    -skeleton_attr.hand.0 - 7.0,
                    skeleton_attr.hand.1 - 7.0,
                    skeleton_attr.hand.2 + 10.0,
                );
                next.hand_l.orientation = Quaternion::rotation_x(0.57) * Quaternion::rotation_z(1.57);
                next.hand_l.scale = Vec3::one() * 1.02;
        
                next.hand_r.position = Vec3::new(
                    skeleton_attr.hand.0 - 7.0,
                    skeleton_attr.hand.1 - 7.0,
                    skeleton_attr.hand.2 + 10.0,
                );
                next.hand_r.orientation = Quaternion::rotation_x(0.57) * Quaternion::rotation_z(1.57);
                next.hand_r.scale = Vec3::one() * 1.02;
                next.hand_r.orientation = Quaternion::rotation_x(0.57) * Quaternion::rotation_z(1.57);
                next.hand_r.scale = Vec3::one() * 1.02;
                
                next.control.position = Vec3::new(7.0, 9.0, -10.0);
                next.control.orientation = Quaternion::rotation_x(test * 0.02)
                    * Quaternion::rotation_y(test * 0.02)
                    * Quaternion::rotation_z(test * 0.02);
                next.control.scale = Vec3::one();

                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(-1.57)
                    * Quaternion::rotation_z(1.0);
                next.main.scale = Vec3::one() * 1.02;
            },
            _ => {},
        }

        next
    }
}