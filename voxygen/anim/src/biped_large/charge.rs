use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::comp::item::ToolKind;
use core::f32::consts::PI;

pub struct ChargeAnimation;

impl Animation for ChargeAnimation {
    type Dependency<'a> = (
        Option<ToolKind>,
        Option<ToolKind>,
        f32,
        Vec3<f32>,
        Vec3<f32>,
        f32,
    );
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_charge\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_charge")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, _second_tool_kind, velocity, orientation, last_ori, _global_time): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab: f32 = 1.0;

        let foot = ((5.0 / (0.2 + 4.8 * ((anim_time * lab * 8.0).sin()).powi(2))).sqrt())
            * ((anim_time * lab * 8.0).sin());
        let foote = ((5.0 / (0.5 + 4.5 * ((anim_time * lab * 8.0 + PI / 2.0).sin()).powi(2)))
            .sqrt())
            * ((anim_time * lab * 8.0).sin());
        let stress = ((5.0 / (0.5 + 4.5 * ((anim_time * lab * 20.0).cos()).powi(2))).sqrt())
            * ((anim_time * lab * 20.0).cos());
        let quick = ((5.0 / (3.5 + 1.5 * ((anim_time * lab * 8.0).sin()).powi(2))).sqrt())
            * ((anim_time * lab * 8.0).sin());
        let stop = (anim_time.powf(0.3)).min(1.2);
        let stopa = (anim_time.powf(0.9)).min(5.0);

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

        next.head.position = Vec3::new(stop * -2.0, -1.5 + stop * 2.5 + s_a.head.0, s_a.head.1);
        next.head.orientation =
            Quaternion::rotation_z(stop * -1.0 + tilt * -2.0) * Quaternion::rotation_y(stop * -0.3);
        next.head.scale = Vec3::one();

        next.upper_torso.position = Vec3::new(0.0, s_a.upper_torso.0, s_a.upper_torso.1);
        next.upper_torso.orientation =
            Quaternion::rotation_z(stop * 1.2 + stress * stop * 0.02 + tilt * -2.0);

        next.lower_torso.position = Vec3::new(0.0, s_a.lower_torso.0, s_a.lower_torso.1);
        next.lower_torso.orientation = Quaternion::rotation_z(stop * -0.7 + tilt * 4.0);

        match active_tool_kind {
            Some(ToolKind::Staff) | Some(ToolKind::Sceptre) => {
                next.hand_l.position = Vec3::new(s_a.sthl.0, s_a.sthl.1, s_a.sthl.2);
                next.hand_l.orientation = Quaternion::rotation_x(s_a.sthl.3);

                next.hand_r.position = Vec3::new(s_a.sthr.0, s_a.sthr.1, s_a.sthr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.sthr.3) * Quaternion::rotation_y(s_a.sthr.4);

                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_y(0.0);

                next.control.position = Vec3::new(
                    s_a.stc.0 + quick * 3.5 * (1.0 / (stopa + 0.1)),
                    s_a.stc.1,
                    s_a.stc.2 - stop * 3.0,
                );
                next.control.orientation = Quaternion::rotation_x(s_a.stc.3 + stop * -0.2)
                    * Quaternion::rotation_y(s_a.stc.4)
                    * Quaternion::rotation_z(s_a.stc.5 + stop * 0.2);
            },
            Some(ToolKind::Bow) => {
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);
                next.hand_l.position = Vec3::new(s_a.bhl.0, s_a.bhl.1, s_a.bhl.2);
                next.hand_l.orientation = Quaternion::rotation_x(s_a.bhl.3);
                next.hand_r.position = Vec3::new(s_a.bhr.0, s_a.bhr.1, s_a.bhr.2);
                next.hand_r.orientation = Quaternion::rotation_x(s_a.bhr.3);

                next.hold.position = Vec3::new(0.0, -1.0, -15.2);
                next.hold.orientation = Quaternion::rotation_x(-PI / 2.0);
                next.hold.scale = Vec3::one() * 1.0;

                next.control.position = Vec3::new(
                    3.0 + s_a.bc.0 + stop * 13.0,
                    -5.0 + s_a.bc.1 + stop * 4.0,
                    6.0 + s_a.bc.2,
                );
                next.control.orientation = Quaternion::rotation_x(0.2 + s_a.bc.3)
                    * Quaternion::rotation_y(-0.8 + s_a.bc.4 + stop * -0.4)
                    * Quaternion::rotation_z(s_a.bc.5 + stop * -0.6);
            },
            _ => {},
        }

        if velocity > 0.2 {
            next.foot_l.position = Vec3::new(
                -s_a.foot.0 - foot * 1.5,
                s_a.foot.1 + foote * 2.0,
                s_a.foot.2,
            );
            next.foot_l.orientation = Quaternion::rotation_x(foote * -0.1)
                * Quaternion::rotation_z(0.4)
                * Quaternion::rotation_y(0.15);

            next.foot_r.position = Vec3::new(
                s_a.foot.0 + foot * 1.5,
                s_a.foot.1 + foote * -1.5,
                s_a.foot.2,
            );
            next.foot_r.orientation = Quaternion::rotation_z(0.4);
        } else {
            next.foot_l.position = Vec3::new(
                -s_a.foot.0,
                -2.5 + stop * -1.3,
                s_a.foot.2 + tilt * -4.0 * foot,
            );
            next.foot_l.orientation =
                Quaternion::rotation_x(stop * -0.2 - 0.2 + stop * stress * 0.02)
                    * Quaternion::rotation_z(stop * 0.1)
                    * Quaternion::rotation_y(stop * 0.08);

            next.foot_r.position =
                Vec3::new(s_a.foot.0, 3.5 + stop * 1.5, s_a.foot.2 + tilt * 4.0 * foot);
            next.foot_r.orientation =
                Quaternion::rotation_x(stop * 0.1) * Quaternion::rotation_z(stop * 0.1);
        }

        next
    }
}
