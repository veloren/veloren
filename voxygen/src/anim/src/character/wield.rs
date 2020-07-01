use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::{Hands, ToolKind};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct WieldAnimation;

impl Animation for WieldAnimation {
    type Dependency = (Option<ToolKind>, Option<ToolKind>, f32, f64);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_wield\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_wield")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, velocity, global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let lab = 1.0;

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

        let slowalt = (anim_time as f32 * 6.0 + PI).cos();
        let u_slow = (anim_time as f32 * 1.0 + PI).sin();
        let slow = (anim_time as f32 * 3.0 + PI).sin();

        let u_slowalt = (anim_time as f32 * 3.0 + PI).cos();
        let short = (((5.0)
            / (1.5 + 3.5 * ((anim_time as f32 * lab as f32 * 16.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 16.0).sin());
        let noisea = (anim_time as f32 * 11.0 + PI / 6.0).sin();
        let noiseb = (anim_time as f32 * 19.0 + PI / 4.0).sin();

        if velocity > 0.5 {
            next.torso.offset = Vec3::new(0.0, 0.0, 0.0) * skeleton_attr.scaler;
            next.torso.ori = Quaternion::rotation_x(-0.2);
            next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

            next.back.offset = Vec3::new(0.0, skeleton_attr.back.0, skeleton_attr.back.1);
            next.back.ori = Quaternion::rotation_x(
                (-0.5 + short * 0.3 + noisea * 0.3 + noiseb * 0.3).min(-0.1),
            );
            next.back.scale = Vec3::one() * 1.02;
        } else {
            next.head.offset = Vec3::new(
                0.0,
                -2.0 + skeleton_attr.head.0,
                skeleton_attr.head.1 + u_slow * 0.1,
            );
            next.head.ori =
                Quaternion::rotation_z(head_look.x) * Quaternion::rotation_x(head_look.y.abs());
            next.head.scale = Vec3::one() * skeleton_attr.head_scale;

            next.chest.offset = Vec3::new(
                0.0 + slowalt * 0.5,
                skeleton_attr.chest.0,
                skeleton_attr.chest.1 + u_slow * 0.5,
            );
            next.torso.offset = Vec3::new(0.0, 0.0, 0.0) * skeleton_attr.scaler;
            next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

            next.l_foot.offset = Vec3::new(
                -skeleton_attr.foot.0,
                -2.0 + skeleton_attr.foot.1,
                skeleton_attr.foot.2,
            );
            next.l_foot.ori = Quaternion::rotation_x(u_slowalt * 0.035 - 0.2);

            next.r_foot.offset = Vec3::new(
                skeleton_attr.foot.0,
                2.0 + skeleton_attr.foot.1,
                skeleton_attr.foot.2,
            );
            next.r_foot.ori = Quaternion::rotation_x(u_slow * 0.035);

            next.chest.ori =
                Quaternion::rotation_y(u_slowalt * 0.04) * Quaternion::rotation_z(0.15);

            next.belt.offset = Vec3::new(0.0, skeleton_attr.belt.0, skeleton_attr.belt.1);
            next.belt.ori = Quaternion::rotation_y(u_slowalt * 0.03) * Quaternion::rotation_z(0.22);
            next.belt.scale = Vec3::one() * 1.02;

            next.back.offset = Vec3::new(0.0, skeleton_attr.back.0, skeleton_attr.back.1);
            next.back.ori = Quaternion::rotation_x(-0.2);
            next.back.scale = Vec3::one() * 1.02;
            next.shorts.offset = Vec3::new(0.0, skeleton_attr.shorts.0, skeleton_attr.shorts.1);
            next.shorts.ori = Quaternion::rotation_z(0.3);
        }
        match active_tool_kind {
            //TODO: Inventory
            Some(ToolKind::Sword(_)) => {
                next.l_hand.offset = Vec3::new(-0.75, -1.0, -2.5);
                next.l_hand.ori = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(-0.2);
                next.l_hand.scale = Vec3::one() * 1.04;
                next.r_hand.offset = Vec3::new(0.75, -1.5, -5.5);
                next.r_hand.ori = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(0.3);
                next.r_hand.scale = Vec3::one() * 1.05;
                next.main.offset = Vec3::new(0.0, 0.0, -3.0);
                next.main.ori = Quaternion::rotation_x(-0.1)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);

                next.control.offset = Vec3::new(-7.0, 6.0, 6.0);
                next.control.ori = Quaternion::rotation_x(u_slow * 0.15)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(u_slowalt * 0.08);
                next.control.scale = Vec3::one();
            },
            Some(ToolKind::Dagger(_)) => {
                // hands should be larger when holding a dagger grip,
                // also reduce flicker with overlapping polygons
                let hand_scale = 1.12;

                next.control.offset = Vec3::new(0.0, 0.0, 0.0);
                // next.control.ori = Quaternion::rotation_x(u_slow * 0.15 + 1.0)
                //     * Quaternion::rotation_y(0.0)
                //     * Quaternion::rotation_z(u_slowalt * 0.08);
                // next.control.scale = Vec3::one();

                next.l_hand.offset = Vec3::new(0.0, 0.0, 0.0);
                next.l_hand.ori = Quaternion::rotation_x(0.0 * PI)
                    * Quaternion::rotation_y(0.0 * PI)
                    * Quaternion::rotation_z(0.0 * PI);
                next.l_hand.scale = Vec3::one() * hand_scale;

                next.main.offset = Vec3::new(0.0, 0.0, 0.0);
                next.main.ori = Quaternion::rotation_x(0.0 * PI)
                    * Quaternion::rotation_y(0.0 * PI)
                    * Quaternion::rotation_z(0.0 * PI);

                next.l_control.offset = Vec3::new(-7.0, 0.0, 0.0);
                // next.l_control.ori = Quaternion::rotation_x(u_slow * 0.15 + 1.0)
                //     * Quaternion::rotation_y(0.0)
                //     * Quaternion::rotation_z(u_slowalt * 0.08);
                // next.l_control.scale = Vec3::one();

                next.r_hand.offset = Vec3::new(0.0, 0.0, 0.0);
                next.r_hand.ori = Quaternion::rotation_x(0.0 * PI)
                    * Quaternion::rotation_y(0.0 * PI)
                    * Quaternion::rotation_z(0.0 * PI);
                next.r_hand.scale = Vec3::one() * hand_scale;

                next.second.offset = Vec3::new(0.0, 0.0, 0.0);
                next.second.ori = Quaternion::rotation_x(0.0 * PI)
                    * Quaternion::rotation_y(0.0 * PI)
                    * Quaternion::rotation_z(0.0 * PI);
                next.second.scale = Vec3::one();

                next.r_control.offset = Vec3::new(7.0, 0.0, 0.0);
                // next.r_control.ori = Quaternion::rotation_x(0.0 * PI)
                // * Quaternion::rotation_y(0.0 * PI)
                // * Quaternion::rotation_z(0.0 * PI);
                // next.r_control.scale = Vec3::one();
            },
            Some(ToolKind::Axe(_)) => {
                next.l_hand.offset = Vec3::new(-4.0, 3.0, 6.0);
                next.l_hand.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_z(3.14 - 0.3)
                    * Quaternion::rotation_y(-0.8);
                next.l_hand.scale = Vec3::one() * 1.08;
                next.r_hand.offset = Vec3::new(-2.5, 9.0, 4.0);
                next.r_hand.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_z(3.14 - 0.3)
                    * Quaternion::rotation_y(-0.8);
                next.r_hand.scale = Vec3::one() * 1.06;
                next.main.offset = Vec3::new(-6.0, 10.0, -1.0);
                next.main.ori = Quaternion::rotation_x(1.27)
                    * Quaternion::rotation_y(-0.3)
                    * Quaternion::rotation_z(-0.8);

                next.control.offset = Vec3::new(0.0, 0.0, 0.0);
                next.control.ori = Quaternion::rotation_x(u_slowalt * 0.1 + 0.2)
                    * Quaternion::rotation_y(-0.3)
                    * Quaternion::rotation_z(u_slow * 0.1 + 0.0);
                next.control.scale = Vec3::one();
            },
            Some(ToolKind::Hammer(_)) => {
                next.l_hand.offset = Vec3::new(-7.0, 5.5, 3.5);
                next.l_hand.ori = Quaternion::rotation_x(0.3) * Quaternion::rotation_y(0.32);
                next.l_hand.scale = Vec3::one() * 1.08;
                next.r_hand.offset = Vec3::new(8.0, 7.75, 0.0);
                next.r_hand.ori = Quaternion::rotation_x(0.3) * Quaternion::rotation_y(0.22);
                next.r_hand.scale = Vec3::one() * 1.06;
                next.main.offset = Vec3::new(6.0, 7.0, 0.0);
                next.main.ori = Quaternion::rotation_x(0.3)
                    * Quaternion::rotation_y(-1.35)
                    * Quaternion::rotation_z(1.57);

                next.control.offset = Vec3::new(0.0, 0.0, 0.0);
                next.control.ori = Quaternion::rotation_x(u_slow * 0.15)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(u_slowalt * 0.08);
                next.control.scale = Vec3::one();
            },
            Some(ToolKind::Staff(_)) => {
                next.l_hand.offset = Vec3::new(1.5, 0.5, -4.0);
                next.l_hand.ori = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(-0.3);
                next.l_hand.scale = Vec3::one() * 1.05;
                next.r_hand.offset = Vec3::new(8.0, 4.0, 2.0);
                next.r_hand.ori = Quaternion::rotation_x(1.8)
                    * Quaternion::rotation_y(0.5)
                    * Quaternion::rotation_z(-0.27);
                next.r_hand.scale = Vec3::one() * 1.05;
                next.main.offset = Vec3::new(12.0, 8.4, 13.2);
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(3.14 + 0.3)
                    * Quaternion::rotation_z(0.9);

                next.control.offset = Vec3::new(-14.0, 1.8, 3.0);
                next.control.ori = Quaternion::rotation_x(u_slow * 0.2)
                    * Quaternion::rotation_y(-0.2)
                    * Quaternion::rotation_z(u_slowalt * 0.1);
                next.control.scale = Vec3::one();
            },
            Some(ToolKind::Shield(_)) => {
                // hands should be larger when holding a dagger grip,
                // also reduce flicker with overlapping polygons
                let hand_scale = 1.12;

                next.control.offset = Vec3::new(0.0, 0.0, 0.0);
                // next.control.ori = Quaternion::rotation_x(u_slow * 0.15 + 1.0)
                //     * Quaternion::rotation_y(0.0)
                //     * Quaternion::rotation_z(u_slowalt * 0.08);
                // next.control.scale = Vec3::one();

                next.l_hand.offset = Vec3::new(0.0, 0.0, 0.0);
                next.l_hand.ori = Quaternion::rotation_x(0.0 * PI)
                    * Quaternion::rotation_y(0.0 * PI)
                    * Quaternion::rotation_z(0.0 * PI);
                next.l_hand.scale = Vec3::one() * hand_scale;

                next.main.offset = Vec3::new(0.0, 0.0, 0.0);
                next.main.ori = Quaternion::rotation_x(0.0 * PI)
                    * Quaternion::rotation_y(0.0 * PI)
                    * Quaternion::rotation_z(0.0 * PI);

                next.l_control.offset = Vec3::new(-7.0, 0.0, 0.0);
                // next.l_control.ori = Quaternion::rotation_x(u_slow * 0.15 + 1.0)
                //     * Quaternion::rotation_y(0.0)
                //     * Quaternion::rotation_z(u_slowalt * 0.08);
                // next.l_control.scale = Vec3::one();

                next.r_hand.offset = Vec3::new(0.0, 0.0, 0.0);
                next.r_hand.ori = Quaternion::rotation_x(0.0 * PI)
                    * Quaternion::rotation_y(0.0 * PI)
                    * Quaternion::rotation_z(0.0 * PI);
                next.r_hand.scale = Vec3::one() * hand_scale;

                next.second.offset = Vec3::new(0.0, 0.0, 0.0);
                next.second.ori = Quaternion::rotation_x(0.0 * PI)
                    * Quaternion::rotation_y(0.0 * PI)
                    * Quaternion::rotation_z(0.0 * PI);
                next.second.scale = Vec3::one();

                next.r_control.offset = Vec3::new(7.0, 0.0, 0.0);
                // next.r_control.ori = Quaternion::rotation_x(0.0 * PI)
                // * Quaternion::rotation_y(0.0 * PI)
                // * Quaternion::rotation_z(0.0 * PI);
                // next.r_control.scale = Vec3::one();
            },
            Some(ToolKind::Bow(_)) => {
                next.l_hand.offset = Vec3::new(2.0, 1.5, 0.0);
                next.l_hand.ori = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6)
                    * Quaternion::rotation_z(-0.3);
                next.l_hand.scale = Vec3::one() * 1.05;
                next.r_hand.offset = Vec3::new(5.9, 4.5, -5.0);
                next.r_hand.ori = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6)
                    * Quaternion::rotation_z(-0.3);
                next.r_hand.scale = Vec3::one() * 1.05;
                next.main.offset = Vec3::new(3.0, 2.0, -13.0);
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.3)
                    * Quaternion::rotation_z(-0.6);

                next.hold.offset = Vec3::new(1.2, -1.0, -5.2);
                next.hold.ori = Quaternion::rotation_x(-1.7)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(-0.1);
                next.hold.scale = Vec3::one() * 1.0;

                next.control.offset = Vec3::new(-7.0, 6.0, 6.0);
                next.control.ori =
                    Quaternion::rotation_x(u_slow * 0.2) * Quaternion::rotation_z(u_slowalt * 0.1);
                next.control.scale = Vec3::one();
            },
            Some(ToolKind::Debug(_)) => {
                next.l_hand.offset = Vec3::new(-7.0, 4.0, 3.0);
                next.l_hand.ori = Quaternion::rotation_x(1.27)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(7.0, 2.5, -1.25);
                next.r_hand.ori = Quaternion::rotation_x(1.27)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(-0.3);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.main.offset = Vec3::new(5.0, 8.75, -2.0);
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(-1.27)
                    * Quaternion::rotation_z(0.0);
                next.main.scale = Vec3::one();
            },
            Some(ToolKind::Farming(_)) => {
                if velocity < 0.5 {
                    next.head.ori = Quaternion::rotation_z(head_look.x)
                        * Quaternion::rotation_x(-0.2 + head_look.y.abs());
                    next.head.scale = Vec3::one() * skeleton_attr.head_scale;
                }
                next.l_hand.offset = Vec3::new(9.0, 1.0, 1.0);
                next.l_hand.ori = Quaternion::rotation_x(1.57) * Quaternion::rotation_y(0.0);
                next.l_hand.scale = Vec3::one() * 1.05;
                next.r_hand.offset = Vec3::new(9.0, 1.0, 11.0);
                next.r_hand.ori = Quaternion::rotation_x(1.57)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.r_hand.scale = Vec3::one() * 1.05;
                next.main.offset = Vec3::new(7.5, 7.5, 13.2);
                next.main.ori = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(3.14)
                    * Quaternion::rotation_z(0.0);

                next.control.offset = Vec3::new(-11.0 + slow * 2.0, 1.8, 4.0);
                next.control.ori = Quaternion::rotation_x(u_slow * 0.1)
                    * Quaternion::rotation_y(0.6 + u_slow * 0.1)
                    * Quaternion::rotation_z(u_slowalt * 0.1);
                next.control.scale = Vec3::one();
            },
            _ => {},
        }

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
