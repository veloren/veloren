use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::ToolKind;
use std::f32::consts::PI;
use vek::*;

pub struct ChargeAnimation;

impl Animation for ChargeAnimation {
    type Dependency = (Option<ToolKind>, f32, f64);
    type Skeleton = CharacterSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (active_tool_kind, velocity, _global_time): Self::Dependency,
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

        next.head.offset = Vec3::new(
            0.0 + stop * -2.0 + skeleton_attr.neck_right,
            -2.0 + stop * 2.5 + skeleton_attr.neck_forward,
            skeleton_attr.neck_height + 14.0,
        );
        next.head.ori = Quaternion::rotation_z(stop * -1.0)
            * Quaternion::rotation_x(0.0)
            * Quaternion::rotation_y(stop * -0.3);
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(0.0, 0.0, 7.0);
        next.chest.ori = Quaternion::rotation_z(stop * 1.2 + stress * stop * 0.02)
            * Quaternion::rotation_x(0.0)
            * Quaternion::rotation_y(0.0);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0, 0.0, -2.0);
        next.belt.ori = Quaternion::rotation_z(stop * -0.5);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(0.0, 0.0, -5.0);
        next.shorts.ori = Quaternion::rotation_z(stop * -0.7);
        next.shorts.scale = Vec3::one();

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
                next.main.scale = Vec3::one();

                next.control.offset = Vec3::new(
                    -7.0 + quick * 3.5 * (1.0 / (stopa + 0.1)),
                    6.0 + quicka * 3.5 * (1.0 / (stopa + 0.1)),
                    6.0 - stop * 3.0,
                );
                next.control.ori = Quaternion::rotation_x(stop * -0.2)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(stop * 0.2);
                next.control.scale = Vec3::one();
            },
            Some(ToolKind::Bow(_)) => {
                next.l_hand.offset = Vec3::new(1.0, -4.0 + stop * -1.0, 0.0);
                next.l_hand.ori = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6)
                    * Quaternion::rotation_z(-0.3);
                next.l_hand.scale = Vec3::one() * 1.05;
                next.r_hand.offset = Vec3::new(3.0, -1.0, -5.0);
                next.r_hand.ori = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6)
                    * Quaternion::rotation_z(-0.3);
                next.r_hand.scale = Vec3::one() * 1.05;
                next.main.offset = Vec3::new(3.0, 2.0, -13.0);
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.3)
                    * Quaternion::rotation_z(-0.6);
                next.main.scale = Vec3::one();

                next.control.offset = Vec3::new(-9.0 + stop * 13.0, 6.0 + stop * 4.0, 8.0);
                next.control.ori = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(stop * -0.5)
                    * Quaternion::rotation_z(stop * -0.9);
                next.control.scale = Vec3::one();
            },
            _ => {},
        }
        if velocity > 0.5 {
            next.l_foot.offset = Vec3::new(-3.4 - foot * 1.5, foote * 2.0, 8.0);
            next.l_foot.ori = Quaternion::rotation_x(foote * -0.1)
                * Quaternion::rotation_z(0.4)
                * Quaternion::rotation_y(0.15);
            next.l_foot.scale = Vec3::one();

            next.r_foot.offset = Vec3::new(3.4 + foot * 1.5, foote * -1.5, 8.0);
            next.r_foot.ori = Quaternion::rotation_x(0.0)
                * Quaternion::rotation_z(0.4)
                * Quaternion::rotation_y(0.0);
            next.r_foot.scale = Vec3::one();
            next.torso.offset =
                Vec3::new(0.0 + foot * 0.03, foote * 0.05, 0.1) * skeleton_attr.scaler;
            next.torso.ori = Quaternion::rotation_z(0.0)
                * Quaternion::rotation_x(0.0)
                * Quaternion::rotation_y(0.0);
            next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
        } else {
            next.l_foot.offset = Vec3::new(-3.4, -2.5 + stop * -1.3, 8.0);
            next.l_foot.ori = Quaternion::rotation_x(stop * -0.2 - 0.2 + stop * stress * 0.02)
                * Quaternion::rotation_z(stop * 0.1)
                * Quaternion::rotation_y(stop * 0.08);
            next.l_foot.scale = Vec3::one();

            next.r_foot.offset = Vec3::new(3.4, 3.5 + stop * 1.5, 8.0);
            next.r_foot.ori =
                Quaternion::rotation_x(stop * 0.1) * Quaternion::rotation_z(stop * 0.1);
            next.r_foot.scale = Vec3::one();
            next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
            next.torso.ori = Quaternion::rotation_z(0.0)
                * Quaternion::rotation_x(0.0)
                * Quaternion::rotation_y(0.0);
            next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
        }

        next.l_shoulder.offset = Vec3::new(-5.0, 0.0, 4.7);
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(5.0, 0.0, 4.7);
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.glider.offset = Vec3::new(0.0, 5.0, 0.0);
        next.glider.ori = Quaternion::rotation_y(0.0);
        next.glider.scale = Vec3::one() * 0.0;

        next.lantern.offset = Vec3::new(0.0, 0.0, 0.0);
        next.lantern.ori = Quaternion::rotation_x(0.0);
        next.lantern.scale = Vec3::one() * 0.0;

        next.l_control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.l_control.ori = Quaternion::rotation_x(0.0);
        next.l_control.scale = Vec3::one();

        next.r_control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.r_control.ori = Quaternion::rotation_x(0.0);
        next.r_control.scale = Vec3::one();
        next
    }
}
