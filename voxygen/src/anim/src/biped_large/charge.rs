use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use std::{f32::consts::PI, ops::Mul};

pub struct ChargeAnimation;

impl Animation for ChargeAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        f32,
        Vec3<f32>,
        Vec3<f32>,
        f64,
    );
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_charge\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_charge")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, _second_tool_kind, velocity, orientation, last_ori, global_time): Self::Dependency,
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

        let quick = (((5.0)
            / (3.5 + 1.5 * ((anim_time as f32 * lab as f32 * 8.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 8.0).sin());
        let quicka = (((5.0)
            / (3.5
                + 1.5
                    * ((anim_time as f32 * lab as f32 * 8.0 + PI / 2.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 8.0 + PI / 2.0).sin());
        let stress = (((5.0)
            / (0.5 + 4.5 * ((anim_time as f32 * lab as f32 * 20.0).cos()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 20.0).cos());

        let short = (anim_time as f32 * lab as f32 * 16.0).sin();
        let shortalt = (anim_time as f32 * lab as f32 * 16.0 + PI / 2.0).sin();
        let stop = ((anim_time as f32).powf(0.3 as f32)).min(1.2);
        let stopa = ((anim_time as f32).powf(0.9 as f32)).min(5.0);

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

        next.head.position = Vec3::new(
            stop * -2.0,
            -3.5 + stop * 2.5 + skeleton_attr.head.0,
            skeleton_attr.head.1,
        );
        next.head.orientation =
            Quaternion::rotation_z(stop * -1.0 + tilt * -2.0) * Quaternion::rotation_y(stop * -0.3);
        next.head.scale = Vec3::one() * 1.02;

        next.upper_torso.position = Vec3::new(
            0.0,
            skeleton_attr.upper_torso.0,
            skeleton_attr.upper_torso.1,
        );
        next.upper_torso.orientation =
            Quaternion::rotation_z(stop * 1.2 + stress * stop * 0.02 + tilt * -2.0);

        next.lower_torso.position = Vec3::new(
            0.0,
            skeleton_attr.lower_torso.0,
            skeleton_attr.lower_torso.1,
        );
        next.lower_torso.orientation = Quaternion::rotation_z(stop * -0.7 + tilt * 4.0);

        if velocity < 0.5 {
            next.jaw.position = Vec3::new(
                0.0,
                skeleton_attr.jaw.0 - slower * 0.12,
                skeleton_attr.jaw.1 + slow * 0.2,
            );
            next.jaw.orientation = Quaternion::rotation_x(slow * 0.05);
            next.jaw.scale = Vec3::one() * 0.98;

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
        } else {
            next.jaw.position = Vec3::new(
                0.0,
                skeleton_attr.jaw.0 - slower * 0.12,
                skeleton_attr.jaw.1 + slow * 0.2,
            );
            next.jaw.orientation = Quaternion::rotation_x(slow * 0.05);
            next.jaw.scale = Vec3::one() * 0.98;

            next.tail.position = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
            next.tail.orientation =
                Quaternion::rotation_z(0.0 + slow * 0.2 + tailmove.x) * Quaternion::rotation_x(0.0);
            next.tail.scale = Vec3::one();

            next.torso.position = Vec3::new(0.0, 0.0, 0.0) / 8.0;
            next.torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.25);
            next.torso.scale = Vec3::one() / 8.0;

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
            );
            next.foot_l.orientation =
                Quaternion::rotation_x(-0.5 + footrotl * 0.85) * Quaternion::rotation_y(0.0);
            next.foot_l.scale = Vec3::one();

            next.foot_r.position = Vec3::new(
                skeleton_attr.foot.0,
                4.0 + skeleton_attr.foot.1 + foothorir * 8.5,
                skeleton_attr.foot.2 + ((footvertr * 6.5).max(0.0)),
            );
            next.foot_r.orientation =
                Quaternion::rotation_x(-0.5 + footrotr * 0.85) * Quaternion::rotation_y(0.0);
            next.foot_r.scale = Vec3::one();
        }
        match active_tool_kind {
            Some(ToolKind::Bow(_)) => {
                next.hand_l.position = Vec3::new(2.0, -2.0 + stop * -1.0, 0.0);
                next.hand_l.orientation = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6)
                    * Quaternion::rotation_z(-0.3);
                next.hand_l.scale = Vec3::one() * 1.05;

                next.hand_r.position = Vec3::new(5.9, 0.0, -5.0);
                next.hand_r.orientation = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6)
                    * Quaternion::rotation_z(-0.3);
                next.hand_r.scale = Vec3::one() * 1.05;

                next.shoulder_l.position = Vec3::new(
                    -skeleton_attr.shoulder.0,
                    skeleton_attr.shoulder.1 + foothoril * -1.0,
                    skeleton_attr.shoulder.2,
                );
                next.shoulder_l.orientation = Quaternion::rotation_x(1.4 + footrotl * -0.06)
                    * Quaternion::rotation_y(-0.9)
                    * Quaternion::rotation_z(footrotl * -0.05);
                next.shoulder_l.scale = Vec3::one();

                next.shoulder_r.position = Vec3::new(
                    skeleton_attr.shoulder.0,
                    skeleton_attr.shoulder.1 + foothorir * -1.0,
                    skeleton_attr.shoulder.2,
                );
                next.shoulder_r.orientation = Quaternion::rotation_x(1.3 + footrotr * -0.06)
                    * Quaternion::rotation_y(-0.5) //1.9
                    * Quaternion::rotation_z(footrotr * -0.05);
                next.shoulder_r.scale = Vec3::one();

                next.jaw.position = Vec3::new(0.0, skeleton_attr.jaw.0, skeleton_attr.jaw.1);
                next.jaw.orientation = Quaternion::rotation_x(stop * 0.05);

                next.tail.position = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
                next.tail.orientation = Quaternion::rotation_z(0.02 * stress * stop + tilt * 2.0)
                    * Quaternion::rotation_x(-0.2 * stop);
                next.tail.scale = Vec3::one();

                next.main.position = Vec3::new(7.0, 2.0, -13.0);
                next.main.orientation = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.3)
                    * Quaternion::rotation_z(-0.6);

                next.hold.position = Vec3::new(1.4, -0.3, -13.8);
                next.hold.orientation = Quaternion::rotation_x(-1.6)
                    * Quaternion::rotation_y(-0.1)
                    * Quaternion::rotation_z(0.0);
                next.hold.scale = Vec3::one() * 1.0;

                next.control.position = Vec3::new(-10.0 + stop * 13.0, 6.0 + stop * 4.0, -2.0);
                next.control.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(stop * -0.4)
                    * Quaternion::rotation_z(stop * -0.6);
                next.control.scale = Vec3::one();
            },
            Some(ToolKind::Staff(_)) => {
                next.hand_l.position = Vec3::new(11.0, 5.0, -4.0);
                next.hand_l.orientation = Quaternion::rotation_x(1.27);
                next.hand_l.scale = Vec3::one() * 1.05;

                next.hand_r.position = Vec3::new(12.0, 5.5, 2.0);
                next.hand_r.orientation =
                    Quaternion::rotation_x(1.57) * Quaternion::rotation_y(0.2);
                next.hand_r.scale = Vec3::one() * 1.05;

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

                next.tail.position = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
                next.tail.orientation = Quaternion::rotation_z(0.02 * stress * stop + tilt * 2.0)
                    * Quaternion::rotation_x(-0.2 * stop);
                next.tail.scale = Vec3::one();

                next.main.position = Vec3::new(8.0, 8.5, 13.2);
                next.main.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(3.14)
                    * Quaternion::rotation_z(0.0);

                next.control.position = Vec3::new(
                    -7.0 + quick * 3.5 * (1.0 / (stopa + 0.1)),
                    6.0 + quicka * 3.5 * (1.0 / (stopa + 0.1)),
                    6.0 - stop * 3.0,
                );
                next.control.orientation =
                    Quaternion::rotation_x(stop * -0.2) * Quaternion::rotation_z(stop * 0.2);
                next.control.scale = Vec3::one();
            },
            _ => {},
        }

        next
    }
}
