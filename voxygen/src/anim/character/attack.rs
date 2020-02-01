use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::Tool;
use std::f32::consts::PI;
use vek::*;

pub struct Input {
    pub attack: bool,
}
pub struct AttackAnimation;

impl Animation for AttackAnimation {
    type Dependency = (Option<Tool>, f64);
    type Skeleton = CharacterSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (active_tool_kind, _global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let slow = (anim_time as f32 * 9.0).sin();
        let accel_med = 1.0 - (anim_time as f32 * 16.0).cos();
        let accel_slow = 1.0 - (anim_time as f32 * 12.0).cos();
        let accel_fast = 1.0 - (anim_time as f32 * 24.0).cos();
        let decel = (anim_time as f32 * 16.0).min(PI / 2.0).sin();

        next.head.offset = Vec3::new(
            0.0 + skeleton_attr.neck_right,
            -2.0 + skeleton_attr.neck_forward,
            skeleton_attr.neck_height + 21.0,
        );
        next.head.ori = Quaternion::rotation_z(decel * -0.25)
            * Quaternion::rotation_x(0.0 + decel * -0.1)
            * Quaternion::rotation_y(decel * 0.1);
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(0.0, 0.0, 7.0);
        next.chest.ori = Quaternion::rotation_x(0.0);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0, 0.0, 5.0);
        next.belt.ori = Quaternion::rotation_x(0.0);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(0.0, 0.0, 2.0);
        next.shorts.ori = Quaternion::rotation_x(0.0);
        next.shorts.scale = Vec3::one();

        match active_tool_kind {
            //TODO: Inventory
            Some(Tool::Sword) => {
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
            },
            Some(Tool::Axe) => {
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
            },
            Some(Tool::Hammer) => {
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
            },
            Some(Tool::Staff) => {
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
            },
            Some(Tool::Shield) => {
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
            },
            Some(Tool::Bow) => {
                next.l_hand.offset = Vec3::new(-7.0, -2.0 + slow * 5.0, -1.0);
                next.l_hand.ori = Quaternion::rotation_x(PI / 2.0)
                    * Quaternion::rotation_y(-0.3)
                    * Quaternion::rotation_z(0.3);
                next.l_hand.scale = Vec3::one() * 1.01;
                next.r_hand.offset = Vec3::new(1.0, 8.0, 2.5);
                next.r_hand.ori = Quaternion::rotation_x(PI / 2.0)
                    * Quaternion::rotation_y(0.3)
                    * Quaternion::rotation_z(0.3);
                next.r_hand.scale = Vec3::one() * 1.01;
                next.main.offset = Vec3::new(
                    -4.0 + skeleton_attr.weapon_x,
                    15.0 + skeleton_attr.weapon_y,
                    -4.0,
                );
                next.main.ori = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.4)
                    * Quaternion::rotation_z(0.0);
                next.main.scale = Vec3::one();
            },
            Some(Tool::Dagger) => {
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
            },
            Some(Tool::Debug(_)) => {
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

        next.torso.offset = Vec3::new(0.0, -0.2, 0.1) * skeleton_attr.scaler;
        next.torso.ori = Quaternion::rotation_z(decel * -0.2)
            * Quaternion::rotation_x(0.0 + decel * -0.2)
            * Quaternion::rotation_y(decel * 0.2);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
        next
    }
}
