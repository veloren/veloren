use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::{Hands, ToolKind};
use std::f32::consts::PI;
use vek::*;

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
    #[allow(clippy::useless_conversion)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, velocity, orientation, last_ori, _global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
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

        next.head.offset = Vec3::new(
            stop * -2.0,
            -3.5 + stop * 2.5 + skeleton_attr.head.0,
            skeleton_attr.head.1,
        );
        next.head.ori =
            Quaternion::rotation_z(stop * -1.0 + tilt * -2.0) * Quaternion::rotation_y(stop * -0.3);
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1);
        next.chest.ori = Quaternion::rotation_z(stop * 1.2 + stress * stop * 0.02 + tilt * -2.0);

        next.belt.offset = Vec3::new(0.0, skeleton_attr.belt.0, skeleton_attr.belt.1);
        next.belt.ori = Quaternion::rotation_z(stop * -0.5 + tilt * 2.0);

        next.shorts.offset = Vec3::new(0.0, skeleton_attr.shorts.0, skeleton_attr.shorts.1);
        next.shorts.ori = Quaternion::rotation_z(stop * -0.7 + tilt * 4.0);

        match active_tool_kind {
            //TODO: Inventory
            Some(ToolKind::Staff(_)) => {
                next.l_hand.offset = Vec3::new(1.0, -2.0, -5.0);
                next.l_hand.ori = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(-0.3);
                next.l_hand.scale = Vec3::one() * 1.05;
                next.r_hand.offset = Vec3::new(9.0, 1.0, 0.0);
                next.r_hand.ori = Quaternion::rotation_x(1.8)
                    * Quaternion::rotation_y(0.5)
                    * Quaternion::rotation_z(-0.27);
                next.r_hand.scale = Vec3::one() * 1.05;
                next.main.offset = Vec3::new(9.2, 8.4, 13.2);
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(3.14 + 0.3)
                    * Quaternion::rotation_z(0.9);

                next.control.offset = Vec3::new(
                    -7.0 + quick * 3.5 * (1.0 / (stopa + 0.1)),
                    6.0 + quicka * 3.5 * (1.0 / (stopa + 0.1)),
                    6.0 - stop * 3.0,
                );
                next.control.ori =
                    Quaternion::rotation_x(stop * -0.2) * Quaternion::rotation_z(stop * 0.2);
                next.control.scale = Vec3::one();
            },
            Some(ToolKind::Bow(_)) => {
                next.l_hand.offset = Vec3::new(1.0, -2.0 + stop * -1.0, 0.0);
                next.l_hand.ori = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6)
                    * Quaternion::rotation_z(-0.3);
                next.l_hand.scale = Vec3::one() * 1.05;
                next.r_hand.offset = Vec3::new(4.9, 1.0, -5.0);
                next.r_hand.ori = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6)
                    * Quaternion::rotation_z(-0.3);
                next.r_hand.scale = Vec3::one() * 1.05;
                next.main.offset = Vec3::new(3.0, -1.0, -14.0);
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.3)
                    * Quaternion::rotation_z(-0.6);

                next.hold.offset = Vec3::new(0.4, -0.3, -5.8);
                next.hold.ori = Quaternion::rotation_x(-1.6)
                    * Quaternion::rotation_y(-0.1)
                    * Quaternion::rotation_z(0.0);
                next.hold.scale = Vec3::one() * 1.0;

                next.control.offset = Vec3::new(-10.0 + stop * 13.0, 6.0 + stop * 4.0, 8.0);
                next.control.ori = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(stop * -0.4)
                    * Quaternion::rotation_z(stop * -0.6);
                next.control.scale = Vec3::one();
            },
            _ => {},
        }

        if velocity > 0.2 {
            next.l_foot.offset = Vec3::new(
                -skeleton_attr.foot.0 - foot * 1.5,
                skeleton_attr.foot.1 + foote * 2.0,
                skeleton_attr.foot.2,
            );
            next.l_foot.ori = Quaternion::rotation_x(foote * -0.1)
                * Quaternion::rotation_z(0.4)
                * Quaternion::rotation_y(0.15);

            next.r_foot.offset = Vec3::new(
                skeleton_attr.foot.0 + foot * 1.5,
                skeleton_attr.foot.1 + foote * -1.5,
                skeleton_attr.foot.2,
            );
            next.r_foot.ori = Quaternion::rotation_x(0.0)
                * Quaternion::rotation_z(0.4)
                * Quaternion::rotation_y(0.0);

            next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
            next.torso.ori = Quaternion::rotation_z(0.0);
            next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
        } else {
            next.l_foot.offset = Vec3::new(
                -skeleton_attr.foot.0,
                -2.5 + stop * -1.3,
                skeleton_attr.foot.2 + tilt * -4.0 * foot,
            );
            next.l_foot.ori = Quaternion::rotation_x(stop * -0.2 - 0.2 + stop * stress * 0.02)
                * Quaternion::rotation_z(stop * 0.1)
                * Quaternion::rotation_y(stop * 0.08);

            next.r_foot.offset = Vec3::new(
                skeleton_attr.foot.0,
                3.5 + stop * 1.5,
                skeleton_attr.foot.2 + tilt * 4.0 * foot,
            );
            next.r_foot.ori =
                Quaternion::rotation_x(stop * 0.1) * Quaternion::rotation_z(stop * 0.1);

            next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
            next.torso.ori = Quaternion::rotation_z(0.0);
            next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
        }
        next.back.offset = Vec3::new(0.0, skeleton_attr.back.0, skeleton_attr.back.1);
        next.back.ori = Quaternion::rotation_x(-0.3);
        next.back.scale = Vec3::one() * 1.02;

        next.l_shoulder.offset = Vec3::new(
            -skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.glider.offset = Vec3::new(0.0, 0.0, 10.0);
        next.glider.scale = Vec3::one() * 0.0;

        next.lantern.offset = Vec3::new(
            skeleton_attr.lantern.0,
            skeleton_attr.lantern.1,
            skeleton_attr.lantern.2,
        );
        next.lantern.ori = Quaternion::rotation_x(0.1) * Quaternion::rotation_y(0.1);
        next.lantern.scale = Vec3::one() * 0.65;

        next.l_control.scale = Vec3::one();

        next.r_control.scale = Vec3::one();

        next.second.scale = match (
            active_tool_kind.map(|tk| tk.into_hands()),
            second_tool_kind.map(|tk| tk.into_hands()),
        ) {
            (Some(Hands::OneHand), Some(Hands::OneHand)) => Vec3::one(),
            (_, _) => Vec3::zero(),
        };

        next
    }
}
