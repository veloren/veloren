use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::comp::item::ToolKind;
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
        (active_tool_kind, _second_tool_kind, velocity, global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 3.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.2,
            ((global_time + anim_time) as f32 / 3.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.1,
        );

        let lab = 0.55;
        let breathe = 0.0;
        let test = (anim_time as f32 + 36.0 * PI).sin();
        let torso = (anim_time as f32 * lab as f32 + 1.5 * PI).sin();

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

        if skeleton_attr.beast {
            next.jaw.position = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
        } else {
            next.jaw.position = Vec3::new(
                0.0,
                skeleton_attr.jaw.0 - slower * 0.12,
                skeleton_attr.jaw.1 + slow * 0.2,
            );

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
                next.head.position =
                    Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1) * 1.02;
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

                next.jaw.orientation = Quaternion::rotation_x(slow * 0.05);
                next.jaw.scale = Vec3::one() * 0.98;

                next.tail.position = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
                next.tail.orientation = Quaternion::rotation_z(0.0 + slow * 0.2 + tailmove.x)
                    * Quaternion::rotation_x(0.0);
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
                next.torso.orientation =
                    Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.25);
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
                next.jaw.scale = Vec3::one() * 0.98;

                next.tail.position = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
                next.tail.orientation = Quaternion::rotation_z(0.0 + slow * 0.2 + tailmove.x)
                    * Quaternion::rotation_x(0.0);
                next.tail.scale = Vec3::one();

                next.shoulder_l.position = Vec3::new(
                    -skeleton_attr.shoulder.0,
                    skeleton_attr.shoulder.1,
                    skeleton_attr.shoulder.2,
                );
                next.shoulder_l.orientation =
                    Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
                next.shoulder_l.scale = Vec3::one();

                next.shoulder_r.position = Vec3::new(
                    skeleton_attr.shoulder.0,
                    skeleton_attr.shoulder.1,
                    skeleton_attr.shoulder.2,
                );
                next.shoulder_r.orientation =
                    Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
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
                );
                next.foot_l.orientation = Quaternion::rotation_z(0.0);
                next.foot_l.scale = Vec3::one();

                next.foot_r.position = Vec3::new(
                    skeleton_attr.foot.0,
                    skeleton_attr.foot.1,
                    skeleton_attr.foot.2,
                );
                next.foot_r.orientation = Quaternion::rotation_z(0.0);
                next.foot_r.scale = Vec3::one();

                next.torso.position = Vec3::new(0.0, 0.0, 0.0) / 8.0;
                next.torso.orientation = Quaternion::rotation_z(test * 0.0);
                next.torso.scale = Vec3::one() / 8.0;

                next.control.position = Vec3::new(7.0, 9.0, -10.0);
                next.control.orientation = Quaternion::rotation_x(test * 0.02)
                    * Quaternion::rotation_y(test * 0.02)
                    * Quaternion::rotation_z(test * 0.02);
                next.control.scale = Vec3::one();

                next.hold.scale = Vec3::one() * 0.0;
            }
            match active_tool_kind {
                Some(ToolKind::Sword(_)) => {
                    next.hand_l.position = Vec3::new(-4.75, -1.0, 2.5);
                    next.hand_l.orientation =
                        Quaternion::rotation_x(1.47) * Quaternion::rotation_y(-0.2);
                    next.hand_l.scale = Vec3::one() * 1.02;
                    next.hand_r.position = Vec3::new(3.75, -1.5, -0.5);
                    next.hand_r.orientation =
                        Quaternion::rotation_x(1.47) * Quaternion::rotation_y(0.3);
                    next.hand_r.scale = Vec3::one() * 1.02;
                    next.main.position = Vec3::new(3.0, 6.0, -5.0);
                    next.main.orientation = Quaternion::rotation_x(-0.1);

                    next.control.position = Vec3::new(-7.0, 7.0, -10.0);
                    next.control.orientation = Quaternion::rotation_x(u_slow * 0.15)
                        * Quaternion::rotation_z(u_slowalt * 0.08);
                    next.control.scale = Vec3::one();
                },
                Some(ToolKind::Dagger(_)) => {
                    // hands should be larger when holding a dagger grip,
                    // also reduce flicker with overlapping polygons
                    let hand_scale = 1.12;

                    next.control.position = Vec3::new(0.0, 0.0, 0.0);

                    next.hand_l.position = Vec3::new(0.0, 0.0, 0.0);
                    next.hand_l.orientation = Quaternion::rotation_x(0.0 * PI)
                        * Quaternion::rotation_y(0.0 * PI)
                        * Quaternion::rotation_z(0.0 * PI);
                    next.hand_l.scale = Vec3::one() * hand_scale;

                    next.main.position = Vec3::new(0.0, 0.0, 0.0);
                    next.main.orientation = Quaternion::rotation_x(0.0 * PI)
                        * Quaternion::rotation_y(0.0 * PI)
                        * Quaternion::rotation_z(0.0 * PI);

                    next.hand_r.position = Vec3::new(0.0, 0.0, 0.0);
                    next.hand_r.orientation = Quaternion::rotation_x(0.0 * PI)
                        * Quaternion::rotation_y(0.0 * PI)
                        * Quaternion::rotation_z(0.0 * PI);
                    next.hand_r.scale = Vec3::one() * hand_scale;

                    next.second.position = Vec3::new(0.0, 0.0, 0.0);
                    next.second.orientation = Quaternion::rotation_x(0.0 * PI)
                        * Quaternion::rotation_y(0.0 * PI)
                        * Quaternion::rotation_z(0.0 * PI);
                    next.second.scale = Vec3::one();
                },
                Some(ToolKind::Axe(_)) => {
                    if velocity < 0.5 {
                        next.head.position = Vec3::new(
                            0.0,
                            -3.5 + skeleton_attr.head.0,
                            skeleton_attr.head.1 + u_slow * 0.1,
                        );
                        next.head.orientation = Quaternion::rotation_z(head_look.x)
                            * Quaternion::rotation_x(0.35 + head_look.y.abs());
                        next.head.scale = Vec3::one() * 1.01;
                        next.upper_torso.orientation = Quaternion::rotation_x(-0.35)
                            * Quaternion::rotation_y(u_slowalt * 0.04)
                            * Quaternion::rotation_z(0.15);
                        next.lower_torso.position = Vec3::new(
                            0.0,
                            1.0 + skeleton_attr.lower_torso.0,
                            skeleton_attr.lower_torso.1,
                        );
                        next.lower_torso.orientation =
                            Quaternion::rotation_x(0.15) * Quaternion::rotation_z(0.25);
                        next.control.orientation = Quaternion::rotation_x(1.8)
                            * Quaternion::rotation_y(-0.5)
                            * Quaternion::rotation_z(PI - 0.2);
                        next.control.scale = Vec3::one();
                    } else {
                        next.control.orientation = Quaternion::rotation_x(2.1)
                            * Quaternion::rotation_y(-0.4)
                            * Quaternion::rotation_z(PI - 0.2);
                        next.control.scale = Vec3::one();
                    }
                    next.hand_l.position = Vec3::new(-0.5, 0.0, 4.0);
                    next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
                        * Quaternion::rotation_z(0.0)
                        * Quaternion::rotation_y(0.0);
                    next.hand_l.scale = Vec3::one() * 1.08;
                    next.hand_r.position = Vec3::new(0.5, 0.0, -2.5);
                    next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
                        * Quaternion::rotation_z(0.0)
                        * Quaternion::rotation_y(0.0);
                    next.hand_r.scale = Vec3::one() * 1.06;
                    next.main.position = Vec3::new(-0.0, -2.0, -1.0);
                    next.main.orientation = Quaternion::rotation_x(0.0)
                        * Quaternion::rotation_y(0.0)
                        * Quaternion::rotation_z(0.0);

                    next.control.position = Vec3::new(-3.0, 11.0, 3.0);
                },
                Some(ToolKind::Bow(_)) => {
                    next.hand_l.position = Vec3::new(3.0, 2.5, 0.0);
                    next.hand_l.orientation = Quaternion::rotation_x(1.20)
                        * Quaternion::rotation_y(-0.6)
                        * Quaternion::rotation_z(-0.3);
                    next.hand_l.scale = Vec3::one() * 1.05;
                    next.hand_r.position = Vec3::new(5.9, 5.5, -5.0);
                    next.hand_r.orientation = Quaternion::rotation_x(1.20)
                        * Quaternion::rotation_y(-0.6)
                        * Quaternion::rotation_z(-0.3);
                    next.hand_r.scale = Vec3::one() * 1.05;
                    next.main.position = Vec3::new(8.0, 8.0, -13.0);
                    next.main.orientation = Quaternion::rotation_x(-0.3)
                        * Quaternion::rotation_y(0.3)
                        * Quaternion::rotation_z(-0.6);

                    next.hold.position = Vec3::new(1.2, -1.0, -14.2);
                    next.hold.orientation = Quaternion::rotation_x(-1.7)
                        * Quaternion::rotation_y(0.0)
                        * Quaternion::rotation_z(-0.1);
                    next.hold.scale = Vec3::one() * 1.0;

                    next.control.position = Vec3::new(-7.0, 3.0, -8.0);
                    next.control.orientation = Quaternion::rotation_x(u_slow * 0.2)
                        * Quaternion::rotation_z(u_slowalt * 0.1);
                    next.control.scale = Vec3::one();
                },
                Some(ToolKind::Hammer(_)) => {
                    next.hand_l.position = Vec3::new(
                        -skeleton_attr.hand.0 - 7.0,
                        skeleton_attr.hand.1 - 7.0,
                        skeleton_attr.hand.2 + 10.0,
                    );
                    next.hand_l.orientation =
                        Quaternion::rotation_x(0.57) * Quaternion::rotation_z(1.57);
                    next.hand_l.scale = Vec3::one() * 1.02;

                    next.hand_r.position = Vec3::new(
                        skeleton_attr.hand.0 - 7.0,
                        skeleton_attr.hand.1 - 7.0,
                        skeleton_attr.hand.2 + 10.0,
                    );
                    next.hand_r.orientation =
                        Quaternion::rotation_x(0.57) * Quaternion::rotation_z(1.57);
                    next.hand_r.scale = Vec3::one() * 1.02;
                    next.hand_r.orientation =
                        Quaternion::rotation_x(0.57) * Quaternion::rotation_z(1.57);
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
                    next.hand_l.position = Vec3::new(11.0, 5.0, -4.0);
                    next.hand_l.orientation =
                        Quaternion::rotation_x(1.27) * Quaternion::rotation_y(0.0);
                    next.hand_l.scale = Vec3::one() * 1.05;
                    next.hand_r.position = Vec3::new(17.0, 7.5, 2.0);
                    next.hand_r.orientation =
                        Quaternion::rotation_x(1.57) * Quaternion::rotation_y(0.8);
                    next.hand_r.scale = Vec3::one() * 1.05;

                    next.shoulder_l.position = Vec3::new(
                        -skeleton_attr.shoulder.0,
                        skeleton_attr.shoulder.1,
                        skeleton_attr.shoulder.2,
                    );
                    next.shoulder_l.orientation =
                        Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
                    next.shoulder_l.scale = Vec3::one();

                    next.shoulder_r.position = Vec3::new(
                        skeleton_attr.shoulder.0,
                        skeleton_attr.shoulder.1,
                        skeleton_attr.shoulder.2,
                    );
                    next.shoulder_r.orientation =
                        Quaternion::rotation_z(0.4) * Quaternion::rotation_x(1.0);
                    next.shoulder_r.scale = Vec3::one();

                    next.main.position = Vec3::new(10.0, 12.5, 13.2);
                    next.main.orientation = Quaternion::rotation_y(PI);

                    next.control.position = Vec3::new(-18.0, 1.0, -2.0);
                    next.control.orientation = Quaternion::rotation_x(-0.3 + u_slow * 0.1)
                        * Quaternion::rotation_y(0.15)
                        * Quaternion::rotation_z(u_slowalt * 0.08);
                    next.control.scale = Vec3::one();
                },
                Some(ToolKind::NpcWeapon(_)) => {
                    next.shoulder_l.position = Vec3::new(
                        -skeleton_attr.shoulder.0,
                        skeleton_attr.shoulder.1,
                        skeleton_attr.shoulder.2,
                    );
                    next.shoulder_l.orientation =
                        Quaternion::rotation_z(0.0) * Quaternion::rotation_x(breathe);
                    next.shoulder_l.scale = Vec3::one() + breathe;

                    next.shoulder_r.position = Vec3::new(
                        skeleton_attr.shoulder.0,
                        skeleton_attr.shoulder.1,
                        skeleton_attr.shoulder.2,
                    );
                    next.shoulder_r.orientation =
                        Quaternion::rotation_z(0.0) * Quaternion::rotation_x(breathe);
                    next.shoulder_r.scale = Vec3::one() + breathe;

                    next.hand_l.position = Vec3::new(
                        -skeleton_attr.hand.0,
                        skeleton_attr.hand.1,
                        skeleton_attr.hand.2 + torso * 0.6,
                    );
                    next.hand_l.orientation =
                        Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
                    next.hand_l.scale = Vec3::one() * 1.02;

                    next.hand_r.position = Vec3::new(
                        skeleton_attr.hand.0,
                        skeleton_attr.hand.1,
                        skeleton_attr.hand.2 + torso * 0.6,
                    );
                    next.hand_r.orientation =
                        Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
                    next.hand_r.scale = Vec3::one() * 1.02;

                    next.control.position = Vec3::new(0.0, 0.0, 0.0);
                    next.control.orientation = Quaternion::rotation_z(0.0);
                    next.control.scale = Vec3::one();
                    if velocity < 0.5 {
                        next.head.position = Vec3::new(
                            0.0,
                            skeleton_attr.head.0,
                            skeleton_attr.head.1 + torso * 0.2,
                        ) * 1.02;
                        next.head.orientation = Quaternion::rotation_z(look.x * 0.6)
                            * Quaternion::rotation_x(look.y * 0.6 + breathe);
                        next.head.scale = Vec3::one() * 1.02 + breathe * 0.4;

                        next.upper_torso.position = Vec3::new(
                            0.0,
                            skeleton_attr.upper_torso.0,
                            skeleton_attr.upper_torso.1 + torso * 0.5,
                        );
                        next.upper_torso.orientation =
                            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-breathe);
                        next.upper_torso.scale = Vec3::one() - breathe * 0.4;

                        next.lower_torso.position = Vec3::new(
                            0.0,
                            skeleton_attr.lower_torso.0,
                            skeleton_attr.lower_torso.1 + torso * 0.15,
                        );
                        next.lower_torso.orientation =
                            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(breathe);
                        next.lower_torso.scale = Vec3::one() * 1.02 + breathe * 0.4;

                        next.jaw.orientation = Quaternion::rotation_x(-0.1 + breathe * 2.0);
                        next.jaw.scale = Vec3::one() * 0.98;

                        next.tail.position =
                            Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
                        next.tail.orientation =
                            Quaternion::rotation_z(0.0 + slow * 0.2 + tailmove.x)
                                * Quaternion::rotation_x(0.0);
                        next.tail.scale = Vec3::one();

                        next.control.position = Vec3::new(0.0, 0.0, 0.0);
                        next.control.orientation = Quaternion::rotation_z(0.0);
                        next.control.scale = Vec3::one();

                        next.second.position = Vec3::new(0.0, 0.0, 0.0);
                        next.second.orientation = Quaternion::rotation_x(PI)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(0.0);
                        next.second.scale = Vec3::one() * 0.0;

                        next.main.position = Vec3::new(-5.0, -7.0, 7.0);
                        next.main.orientation = Quaternion::rotation_x(PI)
                            * Quaternion::rotation_y(0.6)
                            * Quaternion::rotation_z(1.57);
                        next.main.scale = Vec3::one() * 1.02;

                        next.shoulder_l.position = Vec3::new(
                            -skeleton_attr.shoulder.0,
                            skeleton_attr.shoulder.1,
                            skeleton_attr.shoulder.2,
                        );
                        next.shoulder_l.orientation =
                            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(breathe);
                        next.shoulder_l.scale = Vec3::one() + breathe;

                        next.shoulder_r.position = Vec3::new(
                            skeleton_attr.shoulder.0,
                            skeleton_attr.shoulder.1,
                            skeleton_attr.shoulder.2,
                        );
                        next.shoulder_r.orientation =
                            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(breathe);
                        next.shoulder_r.scale = Vec3::one() + breathe;

                        next.hand_l.position = Vec3::new(
                            -skeleton_attr.hand.0,
                            skeleton_attr.hand.1,
                            skeleton_attr.hand.2 + torso * 0.6,
                        );
                        next.hand_l.orientation =
                            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
                        next.hand_l.scale = Vec3::one() * 1.02;

                        next.hand_r.position = Vec3::new(
                            skeleton_attr.hand.0,
                            skeleton_attr.hand.1,
                            skeleton_attr.hand.2 + torso * 0.6,
                        );
                        next.hand_r.orientation =
                            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
                        next.hand_r.scale = Vec3::one() * 1.02;

                        next.arm_control_l.scale = Vec3::one() * 1.0;
                        next.arm_control_r.scale = Vec3::one() * 1.0;

                        next.leg_control_l.scale = Vec3::one() * 1.0;

                        next.leg_l.position = Vec3::new(
                            -skeleton_attr.leg.0,
                            skeleton_attr.leg.1,
                            skeleton_attr.leg.2 + torso * 0.2,
                        );
                        next.leg_l.orientation =
                            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
                        next.leg_l.scale = Vec3::one();

                        next.leg_r.position = Vec3::new(
                            skeleton_attr.leg.0,
                            skeleton_attr.leg.1,
                            skeleton_attr.leg.2 + torso * 0.2,
                        );
                        next.leg_r.orientation =
                            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
                        next.leg_r.scale = Vec3::one();

                        next.foot_l.position = Vec3::new(
                            -skeleton_attr.foot.0,
                            skeleton_attr.foot.1,
                            skeleton_attr.foot.2,
                        );
                        next.foot_l.orientation =
                            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
                        next.foot_l.scale = Vec3::one();

                        next.foot_r.position = Vec3::new(
                            skeleton_attr.foot.0,
                            skeleton_attr.foot.1,
                            skeleton_attr.foot.2,
                        );
                        next.foot_r.orientation =
                            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
                        next.foot_r.scale = Vec3::one();

                        next.torso.position = Vec3::new(0.0, 0.0, 0.0) / 8.0;
                        next.torso.orientation =
                            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
                        next.torso.scale = Vec3::one() / 8.0;

                        next.leg_control_l.scale = Vec3::one() * 1.0;
                        next.leg_control_r.scale = Vec3::one() * 1.0;
                        next.arm_control_l.scale = Vec3::one() * 1.0;
                        next.arm_control_r.scale = Vec3::one() * 1.0;

                        next.hold.scale = Vec3::one() * 0.0;
                    } else {
                        next.head.position =
                            Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1) * 1.02;
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

                        next.jaw.position =
                            Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
                        next.jaw.orientation = Quaternion::rotation_x(0.0);
                        next.jaw.scale = Vec3::one() * 1.02;

                        next.tail.position =
                            Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
                        next.tail.orientation = Quaternion::rotation_x(shortalt * 0.3);
                        next.tail.scale = Vec3::one();

                        next.second.position = Vec3::new(0.0, 0.0, 0.0);
                        next.second.orientation = Quaternion::rotation_x(PI)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(0.0);
                        next.second.scale = Vec3::one() * 0.0;

                        next.control.position = Vec3::new(0.0, 0.0, 0.0);
                        next.control.orientation = Quaternion::rotation_z(0.0);
                        next.control.scale = Vec3::one();

                        next.main.position = Vec3::new(-5.0, -7.0, 7.0);
                        next.main.orientation = Quaternion::rotation_x(PI)
                            * Quaternion::rotation_y(0.6)
                            * Quaternion::rotation_z(1.57);
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
                        next.hand_l.orientation =
                            Quaternion::rotation_x(0.15 + (handhoril * -1.2).max(-0.3))
                                * Quaternion::rotation_y(handhoril * -0.1);
                        next.hand_l.scale = Vec3::one() * 1.02;

                        next.hand_r.position = Vec3::new(
                            1.0 + skeleton_attr.hand.0,
                            skeleton_attr.hand.1 + foothorir * -4.0,
                            skeleton_attr.hand.2 + foothorir * 1.0,
                        );
                        next.hand_r.orientation =
                            Quaternion::rotation_x(0.15 + (handhorir * -1.2).max(-0.3))
                                * Quaternion::rotation_y(handhorir * 0.1);
                        next.hand_r.scale = Vec3::one() * 1.02;

                        next.leg_l.position = Vec3::new(
                            -skeleton_attr.leg.0,
                            skeleton_attr.leg.1,
                            skeleton_attr.leg.2,
                        ) * 0.98;
                        next.leg_l.orientation = Quaternion::rotation_z(short * 0.18)
                            * Quaternion::rotation_x(foothoril * 0.3);
                        next.leg_l.scale = Vec3::one() * 0.98;

                        next.leg_r.position = Vec3::new(
                            skeleton_attr.leg.0,
                            skeleton_attr.leg.1,
                            skeleton_attr.leg.2,
                        ) * 0.98;

                        next.leg_r.orientation = Quaternion::rotation_z(short * 0.18)
                            * Quaternion::rotation_x(foothorir * 0.3);
                        next.leg_r.scale = Vec3::one() * 0.98;

                        next.foot_l.position = Vec3::new(
                            -skeleton_attr.foot.0,
                            skeleton_attr.foot.1 + foothoril * 8.5,
                            skeleton_attr.foot.2 + ((footvertl * 6.5).max(0.0)),
                        );
                        next.foot_l.orientation = Quaternion::rotation_x(-0.5 + footrotl * 0.85)
                            * Quaternion::rotation_y(0.0);
                        next.foot_l.scale = Vec3::one();

                        next.foot_r.position = Vec3::new(
                            skeleton_attr.foot.0,
                            skeleton_attr.foot.1 + foothorir * 8.5,
                            skeleton_attr.foot.2 + ((footvertr * 6.5).max(0.0)),
                        );
                        next.foot_r.orientation = Quaternion::rotation_x(-0.5 + footrotr * 0.85)
                            * Quaternion::rotation_y(0.0);
                        next.foot_r.scale = Vec3::one();

                        next.torso.position = Vec3::new(0.0, 0.0, 0.0) / 8.0;
                        next.torso.orientation =
                            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.25);
                        next.torso.scale = Vec3::one() / 8.0;

                        next.leg_control_l.scale = Vec3::one() * 1.0;
                        next.leg_control_r.scale = Vec3::one() * 1.0;
                        next.arm_control_l.scale = Vec3::one() * 1.0;
                        next.arm_control_r.scale = Vec3::one() * 1.0;

                        next.hold.scale = Vec3::one() * 0.0;
                    }
                },
                _ => {},
            }
        };
        next
    }
}
