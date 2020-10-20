use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use std::f32::consts::PI;

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
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_charge\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_charge")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, velocity, orientation, last_ori, _global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;

        let mut next = (*skeleton).clone();

        let lab = 1.0;

        let foot = (((5.0)
            / (0.2 + 4.8 * ((anim_time as f32 * lab as f32 * 8.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 8.0).sin());
        let foote = (((5.0)
            / (0.5 + 4.5 * ((anim_time as f32 * lab as f32 * 8.0 + 1.57).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 8.0).sin());
        let stress = (((5.0)
            / (0.5 + 4.5 * ((anim_time as f32 * lab as f32 * 20.0).cos()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 20.0).cos());
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

        next.head.position = Vec3::new(stop * -2.0, -1.5 + stop * 2.5 + s_a.head.0, s_a.head.1);
        next.head.orientation =
            Quaternion::rotation_z(stop * -1.0 + tilt * -2.0) * Quaternion::rotation_y(stop * -0.3);
        next.head.scale = Vec3::one() * s_a.head_scale;

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);
        next.chest.orientation =
            Quaternion::rotation_z(stop * 1.2 + stress * stop * 0.02 + tilt * -2.0);

        next.belt.position = Vec3::new(0.0, s_a.belt.0, s_a.belt.1);
        next.belt.orientation = Quaternion::rotation_z(stop * -0.5 + tilt * 2.0);

        next.shorts.position = Vec3::new(0.0, s_a.shorts.0, s_a.shorts.1);
        next.shorts.orientation = Quaternion::rotation_z(stop * -0.7 + tilt * 4.0);

        match active_tool_kind {
            //TODO: Inventory
            Some(ToolKind::Staff(_)) | Some(ToolKind::Sceptre(_)) => {
                next.hand_l.position = Vec3::new(11.0, 5.0, -4.0);
                next.hand_l.orientation = Quaternion::rotation_x(1.27);
                next.hand_r.position = Vec3::new(12.0, 5.5, 2.0);
                next.hand_r.orientation =
                    Quaternion::rotation_x(1.57) * Quaternion::rotation_y(0.2);
                next.main.position = Vec3::new(12.0, 8.5, 13.2);
                next.main.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(3.14)
                    * Quaternion::rotation_z(0.0);

                next.control.position = Vec3::new(
                    -7.0 + quick * 3.5 * (1.0 / (stopa + 0.1)),
                    0.0 + quicka * 3.5 * (1.0 / (stopa + 0.1)),
                    8.0 - stop * 3.0,
                );
                next.control.orientation =
                    Quaternion::rotation_x(stop * -0.2) * Quaternion::rotation_z(stop * 0.2);
            },
            Some(ToolKind::Bow(_)) => {
                next.hand_l.position = Vec3::new(1.0, -2.0 + stop * -1.0, 0.0);
                next.hand_l.orientation = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6)
                    * Quaternion::rotation_z(-0.3);
                next.hand_r.position = Vec3::new(4.9, 1.0, -5.0);
                next.hand_r.orientation = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6)
                    * Quaternion::rotation_z(-0.3);
                next.main.position = Vec3::new(3.0, -1.0, -14.0);
                next.main.orientation = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.3)
                    * Quaternion::rotation_z(-0.6);

                next.hold.position = Vec3::new(0.4, -0.3, -5.8);
                next.hold.orientation = Quaternion::rotation_x(-1.6)
                    * Quaternion::rotation_y(-0.1)
                    * Quaternion::rotation_z(0.0);
                next.hold.scale = Vec3::one() * 1.0;

                next.control.position = Vec3::new(-10.0 + stop * 13.0, 6.0 + stop * 4.0, 8.0);
                next.control.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(stop * -0.4)
                    * Quaternion::rotation_z(stop * -0.6);
            },
            Some(ToolKind::Hammer(_)) => {
                next.hand_l.position = Vec3::new(-8.0, -2.0 + stop * -1.0, 13.0);
                next.hand_l.orientation = Quaternion::rotation_x(2.1)
                    * Quaternion::rotation_y(0.7)
                    * Quaternion::rotation_z(-0.3);
                next.hand_r.position = Vec3::new(-11.0, 2.0, 6.0);
                next.hand_r.orientation = Quaternion::rotation_x(1.8)
                    * Quaternion::rotation_y(2.3)
                    * Quaternion::rotation_z(0.3);
                next.main.position = Vec3::new(-12.0, 1.0, 4.0);
                next.main.orientation = Quaternion::rotation_x(0.3)
                    * Quaternion::rotation_y(0.3)
                    * Quaternion::rotation_z(0.6);
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
            next.foot_r.orientation = Quaternion::rotation_x(0.0)
                * Quaternion::rotation_z(0.4)
                * Quaternion::rotation_y(0.0);
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
        next.back.position = Vec3::new(0.0, s_a.back.0, s_a.back.1);
        next.back.orientation = Quaternion::rotation_x(-0.3);

        next.glider.position = Vec3::new(0.0, 0.0, 10.0);

        next.lantern.orientation = Quaternion::rotation_x(0.1) * Quaternion::rotation_y(0.1);
        next.lantern.scale = Vec3::one() * 0.65;

        next.second.scale = match (
            active_tool_kind.map(|tk| tk.hands()),
            second_tool_kind.map(|tk| tk.hands()),
        ) {
            (Some(Hands::OneHand), Some(Hands::OneHand)) => Vec3::one(),
            (_, _) => Vec3::zero(),
        };

        next
    }
}
