use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::ToolKind;
use std::f32::consts::PI;
use vek::*;

pub struct AlphaAnimation;

impl Animation for AlphaAnimation {
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
            / (0.2
                + 4.8
                    * ((anim_time as f32 * lab as f32 * 1.3 * velocity).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 1.3 * velocity).sin());

        let accel_med = 1.0 - (anim_time as f32 * 16.0 * lab as f32).cos();
        let accel_slow = 1.0 - (anim_time as f32 * 12.0 * lab as f32).cos();
        let accel_fast = 1.0 - (anim_time as f32 * 24.0 * lab as f32).cos();
        let decel = (anim_time as f32 * 16.0 * lab as f32).min(PI / 2.0).sin();

        let slow = (((5.0)
            / (0.6 + 4.4 * ((anim_time as f32 * lab as f32 * 11.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 11.0).sin());
        let slower = (((5.0)
            / (0.1 + 4.9 * ((anim_time as f32 * lab as f32 * 4.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 4.0).sin());
        let slowax = (((5.0)
            / (0.1 + 4.9 * ((anim_time as f32 * lab as f32 * 4.0 + 1.9).cos()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 4.0 + 1.9).cos());

        match active_tool_kind {
            //TODO: Inventory
            Some(ToolKind::Sword(_)) => {
                next.head.offset = Vec3::new(
                    0.0 + skeleton_attr.neck_right,
                    -2.0 + skeleton_attr.neck_forward,
                    skeleton_attr.neck_height + 14.0,
                );
                next.head.ori = Quaternion::rotation_z(slow * 0.08)
                    * Quaternion::rotation_x(0.0 + slow * 0.08)
                    * Quaternion::rotation_y(slow * -0.08);
                next.head.scale = Vec3::one() * skeleton_attr.head_scale;

                next.chest.offset = Vec3::new(0.0, 0.0, 7.0);
                next.chest.ori = Quaternion::rotation_z(slow * -0.2)
                    * Quaternion::rotation_x(0.0 + slow * -0.2)
                    * Quaternion::rotation_y(slow * 0.2);
                next.chest.scale = Vec3::one();

                next.belt.offset = Vec3::new(0.0, 0.0, -2.0);
                next.belt.ori = next.chest.ori * -0.2;
                next.belt.scale = Vec3::one();

                next.shorts.offset = Vec3::new(0.0, 0.0, -5.0);
                next.shorts.ori = next.chest.ori * -0.15;
                next.shorts.scale = Vec3::one();

                next.l_hand.offset = Vec3::new(0.0, 1.0, 0.0);
                next.l_hand.ori = Quaternion::rotation_x(1.27);
                next.l_hand.scale = Vec3::one() * 1.04;
                next.r_hand.offset = Vec3::new(0.0, 0.0, -3.0);
                next.r_hand.ori = Quaternion::rotation_x(1.27);
                next.r_hand.scale = Vec3::one() * 1.05;
                next.main.offset = Vec3::new(0.0, 6.0, -1.0);
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.main.scale = Vec3::one();

                next.control.offset = Vec3::new(-8.0 - slow * 1.0, 3.0 - slow * 5.0, 0.0);
                next.control.ori = Quaternion::rotation_x(-1.4)
                    * Quaternion::rotation_y(slow * 1.5 + 0.7)
                    * Quaternion::rotation_z(1.4 + slow * 0.5);
                next.control.scale = Vec3::one();
                next.l_foot.offset = Vec3::new(-3.4, foot * 3.0 + slow * -5.0, 8.0);
                next.l_foot.ori =
                    Quaternion::rotation_x(foot * -0.6) * Quaternion::rotation_y(foot * 0.2);
                next.l_foot.scale = Vec3::one();

                next.r_foot.offset = Vec3::new(3.4, foot * -3.0 + slow * 5.0, 8.0);
                next.r_foot.ori =
                    Quaternion::rotation_x(foot * 0.6) * Quaternion::rotation_y(foot * -0.2);
                next.r_foot.scale = Vec3::one();
                next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                next.torso.ori = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0);
                next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
            },
            Some(ToolKind::Axe(_)) => {
                next.head.offset = Vec3::new(
                    0.0 + skeleton_attr.neck_right + slowax * 2.0,
                    0.0 + skeleton_attr.neck_forward + slowax * -2.0,
                    skeleton_attr.neck_height + 13.0,
                );
                next.head.ori = Quaternion::rotation_z(slowax * 0.25)
                    * Quaternion::rotation_x(0.0 + slowax * 0.2)
                    * Quaternion::rotation_y(slowax * 0.2);
                next.head.scale = Vec3::one() * skeleton_attr.head_scale;

                next.chest.offset = Vec3::new(0.0, 0.0, 7.0);
                next.chest.ori = Quaternion::rotation_z(slowax * 0.2)
                    * Quaternion::rotation_x(0.0 + slowax * 0.2)
                    * Quaternion::rotation_y(slowax * 0.2);
                next.chest.scale = Vec3::one();

                next.belt.offset = Vec3::new(0.0, 0.0, -2.0);
                next.belt.ori = next.chest.ori * -0.2;
                next.belt.scale = Vec3::one();

                next.shorts.offset = Vec3::new(0.0, 0.0, -5.0);
                next.shorts.ori = next.chest.ori * -0.15;
                next.shorts.scale = Vec3::one();

                next.l_hand.offset = Vec3::new(-4.0, 3.0, 2.0);
                next.l_hand.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_z(3.14 - 0.3)
                    * Quaternion::rotation_y(-0.8);
                next.l_hand.scale = Vec3::one() * 1.08;
                next.r_hand.offset = Vec3::new(-2.5, 9.0, 0.0);
                next.r_hand.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_z(3.14 - 0.3)
                    * Quaternion::rotation_y(-0.8);
                next.r_hand.scale = Vec3::one() * 1.06;
                next.main.offset = Vec3::new(-6.0, 10.0, -5.0);
                next.main.ori = Quaternion::rotation_x(1.27)
                    * Quaternion::rotation_y(-0.3)
                    * Quaternion::rotation_z(-0.8);
                next.main.scale = Vec3::one();

                next.control.offset = Vec3::new(0.0, 0.0 + slowax * 8.2, 6.0);
                next.control.ori = Quaternion::rotation_x(0.8)
                    * Quaternion::rotation_y(-0.3)
                    * Quaternion::rotation_z(-0.7 + slowax * -1.9);
                next.control.scale = Vec3::one();
                next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                next.torso.ori = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0);
                next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
            },
            Some(ToolKind::Hammer(_)) => {
                next.l_hand.offset = Vec3::new(0.0, 3.0, 8.0);
                next.l_hand.ori = Quaternion::rotation_x(1.27);
                next.l_hand.scale = Vec3::one() * 1.05;
                next.r_hand.offset = Vec3::new(0.0, 0.0, -3.0);
                next.r_hand.ori = Quaternion::rotation_x(1.27);
                next.r_hand.scale = Vec3::one() * 1.05;
                next.main.offset = Vec3::new(0.0, 6.0, -1.0);
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.main.scale = Vec3::one();
                next.head.offset = Vec3::new(
                    0.0 + skeleton_attr.neck_right,
                    -2.0 + skeleton_attr.neck_forward,
                    skeleton_attr.neck_height + 12.0,
                );
                next.head.ori = Quaternion::rotation_z(slower * 0.05)
                    * Quaternion::rotation_x(0.0 + slower * 0.05)
                    * Quaternion::rotation_y(slower * 0.05);
                next.head.scale = Vec3::one() * skeleton_attr.head_scale;

                next.chest.offset = Vec3::new(0.0, 0.0, 7.0);
                next.chest.ori = Quaternion::rotation_z(slower * 0.2)
                    * Quaternion::rotation_x(0.0 + slower * 0.2)
                    * Quaternion::rotation_y(slower * 0.2);
                next.chest.scale = Vec3::one();

                next.belt.offset = Vec3::new(0.0, 0.0, -2.0);
                next.belt.ori = next.chest.ori * -0.2;
                next.belt.scale = Vec3::one();

                next.shorts.offset = Vec3::new(0.0, 0.0, -5.0);
                next.shorts.ori = next.chest.ori * -0.15;
                next.shorts.scale = Vec3::one();
                next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                next.torso.ori = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0);
                next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

                if velocity > 0.5 {
                    next.l_foot.offset = Vec3::new(-3.4, foot * -2.0, 8.0);
                    next.l_foot.ori = Quaternion::rotation_x(foot * -0.4)
                        * Quaternion::rotation_z((slower * 0.6).max(0.0));
                    next.l_foot.scale = Vec3::one();

                    next.r_foot.offset = Vec3::new(3.4, foot * 2.0, 8.0);
                    next.r_foot.ori = Quaternion::rotation_x(foot * 0.4)
                        * Quaternion::rotation_z((slower * 0.6).max(0.0));
                    next.r_foot.scale = Vec3::one();
                    next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                    next.torso.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.15);
                    next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
                } else {
                    next.l_foot.offset = Vec3::new(-3.4, -2.5, 8.0 + (slower * 2.5).max(0.0));
                    next.l_foot.ori = Quaternion::rotation_x(slower * -0.2 - 0.2)
                        * Quaternion::rotation_z((slower * 1.0).max(0.0));
                    next.l_foot.scale = Vec3::one();

                    next.r_foot.offset = Vec3::new(3.4, 3.5 - slower * 2.0, 8.0);
                    next.r_foot.ori = Quaternion::rotation_x(slower * 0.1)
                        * Quaternion::rotation_z((slower * 0.5).max(0.0));
                    next.r_foot.scale = Vec3::one();
                    next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                    next.torso.ori = Quaternion::rotation_z(0.0)
                        * Quaternion::rotation_x(0.0)
                        * Quaternion::rotation_y(0.0);
                    next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
                }

                next.control.offset = Vec3::new(-6.0, 3.0 + slower * 2.0, 8.0 + slower * 5.0);
                next.control.ori = Quaternion::rotation_x(-0.2 + slower * 1.8)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(1.4 + 1.57);
                next.control.scale = Vec3::one();

                next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                next.torso.ori = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0);
                next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
            },
            Some(ToolKind::Staff(_)) => {
                next.head.offset = Vec3::new(
                    0.0 + skeleton_attr.neck_right,
                    0.0 + skeleton_attr.neck_forward + decel * 0.8,
                    skeleton_attr.neck_height + 14.0,
                );
                next.head.ori = Quaternion::rotation_z(decel * 0.25)
                    * Quaternion::rotation_x(0.0 + decel * 0.1)
                    * Quaternion::rotation_y(decel * -0.1);
                next.head.scale = Vec3::one() * skeleton_attr.head_scale;

                next.chest.offset = Vec3::new(0.0, 0.0, 7.0);
                next.chest.ori = Quaternion::rotation_z(decel * -0.2)
                    * Quaternion::rotation_x(0.0 + decel * -0.2)
                    * Quaternion::rotation_y(decel * 0.2);
                next.chest.scale = Vec3::one();

                next.belt.offset = Vec3::new(0.0, 0.0, -2.0);
                next.belt.ori = Quaternion::rotation_z(decel * -0.1)
                    * Quaternion::rotation_x(0.0 + decel * -0.1)
                    * Quaternion::rotation_y(decel * 0.1);
                next.belt.scale = Vec3::one();

                next.shorts.offset = Vec3::new(0.0, 0.0, -5.0);
                next.belt.ori = Quaternion::rotation_z(decel * -0.08)
                    * Quaternion::rotation_x(0.0 + decel * -0.08)
                    * Quaternion::rotation_y(decel * 0.08);
                next.shorts.scale = Vec3::one();
                next.l_hand.offset = Vec3::new(0.0, 1.0, 0.0);
                next.l_hand.ori = Quaternion::rotation_x(1.27);
                next.l_hand.scale = Vec3::one() * 1.05;
                next.r_hand.offset = Vec3::new(0.0, 0.0, 10.0);
                next.r_hand.ori = Quaternion::rotation_x(1.27);
                next.r_hand.scale = Vec3::one() * 1.05;
                next.main.offset = Vec3::new(0.0, 6.0, -4.0);
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.main.scale = Vec3::one();

                next.control.offset = Vec3::new(-8.0 - slow * 1.0, 3.0 - slow * 5.0, 0.0);
                next.control.ori = Quaternion::rotation_x(-1.2)
                    * Quaternion::rotation_y(slow * 1.5)
                    * Quaternion::rotation_z(1.4 + slow * 0.5);
                next.control.scale = Vec3::one();
                next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                next.torso.ori = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0);
                next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
            },
            Some(ToolKind::Shield(_)) => {
                next.head.offset = Vec3::new(
                    0.0 + skeleton_attr.neck_right,
                    0.0 + skeleton_attr.neck_forward + decel * 0.8,
                    skeleton_attr.neck_height + 21.0,
                );
                next.head.ori = Quaternion::rotation_z(decel * 0.25)
                    * Quaternion::rotation_x(0.0 + decel * 0.1)
                    * Quaternion::rotation_y(decel * -0.1);
                next.head.scale = Vec3::one() * skeleton_attr.head_scale;

                next.chest.offset = Vec3::new(0.0, 0.0, 7.0);
                next.chest.ori = Quaternion::rotation_z(decel * -0.2)
                    * Quaternion::rotation_x(0.0 + decel * -0.2)
                    * Quaternion::rotation_y(decel * 0.2);
                next.chest.scale = Vec3::one();

                next.belt.offset = Vec3::new(0.0, 0.0, 5.0);
                next.belt.ori = Quaternion::rotation_z(decel * -0.1)
                    * Quaternion::rotation_x(0.0 + decel * -0.1)
                    * Quaternion::rotation_y(decel * 0.1);
                next.belt.scale = Vec3::one();

                next.shorts.offset = Vec3::new(0.0, 0.0, 2.0);
                next.belt.ori = Quaternion::rotation_z(decel * -0.08)
                    * Quaternion::rotation_x(0.0 + decel * -0.08)
                    * Quaternion::rotation_y(decel * 0.08);
                next.shorts.scale = Vec3::one();
                next.l_hand.offset =
                    Vec3::new(-8.0 + accel_slow * 10.0, 8.0 + accel_fast * 3.0, 0.0);
                next.l_hand.ori = Quaternion::rotation_z(-0.8)
                    * Quaternion::rotation_x(0.0 + accel_med * -0.8)
                    * Quaternion::rotation_y(0.0 + accel_med * -0.4);
                next.l_hand.scale = Vec3::one() * 1.01;

                next.r_hand.offset =
                    Vec3::new(-8.0 + accel_slow * 10.0, 8.0 + accel_fast * 3.0, -2.0);
                next.r_hand.ori = Quaternion::rotation_z(-0.8)
                    * Quaternion::rotation_x(0.0 + accel_med * -0.8)
                    * Quaternion::rotation_y(0.0 + accel_med * -0.4);
                next.r_hand.scale = Vec3::one() * 1.01;

                next.main.offset = Vec3::new(
                    -8.0 + accel_slow * 10.0 + skeleton_attr.weapon_x,
                    8.0 + accel_fast * 3.0,
                    0.0,
                );
                next.main.ori = Quaternion::rotation_z(-0.8)
                    * Quaternion::rotation_x(0.0 + accel_med * -0.8)
                    * Quaternion::rotation_y(0.0 + accel_med * -0.4);
                next.main.scale = Vec3::one();
                next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                next.torso.ori = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0);
                next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
            },
            Some(ToolKind::Bow(_)) => {
                next.head.offset = Vec3::new(
                    0.0 + skeleton_attr.neck_right,
                    0.0 + skeleton_attr.neck_forward + decel * 0.8,
                    skeleton_attr.neck_height + 21.0,
                );
                next.head.ori = Quaternion::rotation_z(decel * -0.25)
                    * Quaternion::rotation_x(0.0 + decel * -0.1)
                    * Quaternion::rotation_y(decel * -0.1);
                next.head.scale = Vec3::one() * skeleton_attr.head_scale;

                next.chest.offset = Vec3::new(0.0, 0.0, 7.0);
                next.chest.ori = Quaternion::rotation_z(decel * -0.2)
                    * Quaternion::rotation_x(0.0 + decel * -0.2)
                    * Quaternion::rotation_y(decel * 0.2);
                next.chest.scale = Vec3::one();

                next.belt.offset = Vec3::new(0.0, 0.0, 5.0);
                next.belt.ori = Quaternion::rotation_z(decel * -0.1)
                    * Quaternion::rotation_x(0.0 + decel * -0.1)
                    * Quaternion::rotation_y(decel * 0.1);
                next.belt.scale = Vec3::one();

                next.shorts.offset = Vec3::new(0.0, 0.0, 2.0);
                next.belt.ori = Quaternion::rotation_z(decel * -0.08)
                    * Quaternion::rotation_x(0.0 + decel * -0.08)
                    * Quaternion::rotation_y(decel * 0.08);
                next.shorts.scale = Vec3::one();
                next.l_hand.offset = Vec3::new(1.0, -4.0, -1.0);
                next.l_hand.ori = Quaternion::rotation_x(1.27)
                    * Quaternion::rotation_y(-0.6)
                    * Quaternion::rotation_z(-0.3);
                next.l_hand.scale = Vec3::one() * 1.05;
                next.r_hand.offset = Vec3::new(3.0, -1.0, -6.0);
                next.r_hand.ori = Quaternion::rotation_x(1.27)
                    * Quaternion::rotation_y(-0.6)
                    * Quaternion::rotation_z(-0.3);
                next.r_hand.scale = Vec3::one() * 1.05;
                next.main.offset = Vec3::new(3.0, 2.0, -13.0);
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.3)
                    * Quaternion::rotation_z(-0.6);
                next.main.scale = Vec3::one();

                next.control.offset = Vec3::new(-7.0, 6.0, 6.0);
                next.control.ori = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.control.scale = Vec3::one();
                next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                next.torso.ori = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0);
                next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
            },
            Some(ToolKind::Dagger(_)) => {
                next.head.offset = Vec3::new(
                    0.0 + skeleton_attr.neck_right,
                    -2.0 + skeleton_attr.neck_forward + decel * 0.8,
                    skeleton_attr.neck_height + 21.0,
                );
                next.head.ori = Quaternion::rotation_z(decel * 0.25)
                    * Quaternion::rotation_x(0.0 + decel * 0.1)
                    * Quaternion::rotation_y(decel * -0.1);
                next.head.scale = Vec3::one() * skeleton_attr.head_scale;

                next.chest.offset = Vec3::new(0.0, 0.0, 7.0);
                next.chest.ori = Quaternion::rotation_z(decel * -0.2)
                    * Quaternion::rotation_x(0.0 + decel * -0.2)
                    * Quaternion::rotation_y(decel * 0.2);
                next.chest.scale = Vec3::one();

                next.belt.offset = Vec3::new(0.0, 0.0, 5.0);
                next.belt.ori = Quaternion::rotation_z(decel * -0.1)
                    * Quaternion::rotation_x(0.0 + decel * -0.1)
                    * Quaternion::rotation_y(decel * 0.1);
                next.belt.scale = Vec3::one();

                next.shorts.offset = Vec3::new(0.0, 0.0, 2.0);
                next.belt.ori = Quaternion::rotation_z(decel * -0.08)
                    * Quaternion::rotation_x(0.0 + decel * -0.08)
                    * Quaternion::rotation_y(decel * 0.08);
                next.shorts.scale = Vec3::one();
                next.l_hand.offset =
                    Vec3::new(-8.0 + accel_slow * 10.0, 8.0 + accel_fast * 3.0, 0.0);
                next.l_hand.ori = Quaternion::rotation_z(-0.8)
                    * Quaternion::rotation_x(0.0 + accel_med * -0.8)
                    * Quaternion::rotation_y(0.0 + accel_med * -0.4);
                next.l_hand.scale = Vec3::one() * 1.01;

                next.r_hand.offset =
                    Vec3::new(-8.0 + accel_slow * 10.0, 8.0 + accel_fast * 3.0, -2.0);
                next.r_hand.ori = Quaternion::rotation_z(-0.8)
                    * Quaternion::rotation_x(0.0 + accel_med * -0.8)
                    * Quaternion::rotation_y(0.0 + accel_med * -0.4);
                next.r_hand.scale = Vec3::one() * 1.01;

                next.main.offset = Vec3::new(
                    -8.0 + accel_slow * 10.0 + skeleton_attr.weapon_x,
                    8.0 + accel_fast * 3.0,
                    0.0,
                );
                next.main.ori = Quaternion::rotation_z(-0.8)
                    * Quaternion::rotation_x(0.0 + accel_med * -0.8)
                    * Quaternion::rotation_y(0.0 + accel_med * -0.4);
                next.main.scale = Vec3::one();
                next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                next.torso.ori = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0);
                next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
            },
            Some(ToolKind::Debug(_)) => {
                next.head.offset = Vec3::new(
                    0.0 + skeleton_attr.neck_right,
                    -2.0 + skeleton_attr.neck_forward + decel * 0.8,
                    skeleton_attr.neck_height + 14.0,
                );
                next.head.ori = Quaternion::rotation_x(0.0);
                next.head.scale = Vec3::one() * skeleton_attr.head_scale;

                next.chest.offset = Vec3::new(0.0, 0.0, 7.0);
                next.chest.ori = Quaternion::rotation_z(decel * -0.2)
                    * Quaternion::rotation_x(0.0 + decel * -0.2)
                    * Quaternion::rotation_y(decel * 0.2);
                next.chest.scale = Vec3::one();

                next.belt.offset = Vec3::new(0.0, 0.0, -2.0);
                next.belt.ori = Quaternion::rotation_x(0.0);
                next.belt.scale = Vec3::one();

                next.shorts.offset = Vec3::new(0.0, 0.0, -5.0);
                next.belt.ori = Quaternion::rotation_x(0.0);
                next.shorts.scale = Vec3::one();

                next.l_hand.offset =
                    Vec3::new(-8.0 + accel_slow * 10.0, 8.0 + accel_fast * 3.0, 0.0);
                next.l_hand.ori = Quaternion::rotation_z(-0.8)
                    * Quaternion::rotation_x(0.0 + accel_med * -0.8)
                    * Quaternion::rotation_y(0.0 + accel_med * -0.4);
                next.l_hand.scale = Vec3::one() * 1.01;

                next.r_hand.offset =
                    Vec3::new(-8.0 + accel_slow * 10.0, 8.0 + accel_fast * 3.0, -2.0);
                next.r_hand.ori = Quaternion::rotation_z(-0.8)
                    * Quaternion::rotation_x(0.0 + accel_med * -0.8)
                    * Quaternion::rotation_y(0.0 + accel_med * -0.4);
                next.r_hand.scale = Vec3::one() * 1.01;

                next.main.offset = Vec3::new(
                    -8.0 + accel_slow * 10.0 + skeleton_attr.weapon_x,
                    8.0 + accel_fast * 3.0,
                    0.0,
                );
                next.main.ori = Quaternion::rotation_z(-0.8)
                    * Quaternion::rotation_x(0.0 + accel_med * -0.8)
                    * Quaternion::rotation_y(0.0 + accel_med * -0.4);
                next.main.scale = Vec3::one();
                next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                next.torso.ori = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0);
                next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
            },
            _ => {},
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
