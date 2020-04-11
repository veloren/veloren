use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::ToolKind;
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct SwimAnimation;

impl Animation for SwimAnimation {
    type Dependency = (Option<ToolKind>, Vec3<f32>, f32, f64);
    type Skeleton = CharacterSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, velocity, _orientation, global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let speed = Vec2::<f32>::from(velocity).magnitude();
        *rate = 1.0;

        let lab = 1.0;

        let short = (anim_time as f32 * lab as f32 * 2.0 * speed / 5.0).sin();

        let shortalt = (anim_time as f32 * lab as f32 * 2.0 * speed / 5.0 + PI / 2.0).sin();

        let foot = (anim_time as f32 * lab as f32 * 2.0 * speed / 5.0).sin();

        let wave_stop = (anim_time as f32 * 3.0 * speed / 5.0)
            .min(PI / 2.0 / 2.0)
            .sin();

        let head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 18.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.2,
            ((global_time + anim_time) as f32 / 18.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.1,
        );

        next.head.offset = Vec3::new(
            0.0,
            -3.0 + skeleton_attr.neck_forward,
            skeleton_attr.neck_height + 13.0 + short * 0.3,
        );
        next.head.ori = Quaternion::rotation_z(head_look.x - short * 0.3)
            * Quaternion::rotation_x(head_look.y + 0.35);
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(0.0, 0.0, 7.0 + short * 1.1);
        next.chest.ori = Quaternion::rotation_z(short * 0.3);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0, 0.0, -2.0);
        next.belt.ori = Quaternion::rotation_z(short * 0.25);
        next.belt.scale = Vec3::one();

        next.back.offset = Vec3::new(0.0, -2.8, 7.25);
        next.back.ori = Quaternion::rotation_z(0.0);
        next.back.scale = Vec3::one() * 1.02;

        next.shorts.offset = Vec3::new(0.0, 0.0, -5.0);
        next.shorts.ori = Quaternion::rotation_z(short * 0.4);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(-6.0, -0.25 - foot * 1.0, 5.0 + foot * -2.5);
        next.l_hand.ori = Quaternion::rotation_x(0.8 + foot * -0.5) * Quaternion::rotation_y(0.2);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(6.0, -0.25 + foot * 1.0, 5.0 + foot * 2.5);
        next.r_hand.ori = Quaternion::rotation_x(0.8 + foot * 0.5) * Quaternion::rotation_y(-0.2);
        next.r_hand.scale = Vec3::one();

        next.l_foot.offset = Vec3::new(-3.4, 6.0 + foot * 1.0, 0.0 + foot * 5.5);
        next.l_foot.ori = Quaternion::rotation_x(-1.40 + foot * 0.5);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(3.4, 6.0 - foot * 1.0, 0.0 + foot * -5.5);
        next.r_foot.ori = Quaternion::rotation_x(-1.40 + foot * -0.5);
        next.r_foot.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(-5.0, -1.0, 4.7);
        next.l_shoulder.ori = Quaternion::rotation_x(short * 0.15);
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(5.0, -1.0, 4.7);
        next.r_shoulder.ori = Quaternion::rotation_x(short * -0.15);
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.glider.offset = Vec3::new(0.0, 5.0, 0.0);
        next.glider.ori = Quaternion::rotation_y(0.0);
        next.glider.scale = Vec3::one() * 0.0;

        next.main.offset = Vec3::new(
            -7.0 + skeleton_attr.weapon_x,
            -5.0 + skeleton_attr.weapon_y,
            15.0,
        );
        next.main.ori = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57 + short * 0.25);
        next.main.scale = Vec3::one();

        next.second.offset = Vec3::new(
            0.0 + skeleton_attr.weapon_x,
            0.0 + skeleton_attr.weapon_y,
            0.0,
        );
        next.second.ori = Quaternion::rotation_y(0.0);
        next.second.scale = Vec3::one() * 0.0;

        next.lantern.offset = Vec3::new(0.0, 5.0, 0.0);
        next.lantern.ori = Quaternion::rotation_y(0.0);
        next.lantern.scale = Vec3::one() * 0.0;

        next.torso.offset = Vec3::new(0.0, -0.3 + shortalt * -0.065, 0.4) * skeleton_attr.scaler;
        next.torso.ori =
            Quaternion::rotation_x(speed * -0.190 * wave_stop * 1.6) * Quaternion::rotation_y(0.0);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next.control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.control.ori = Quaternion::rotation_x(0.0);
        next.control.scale = Vec3::one();

        next.l_control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.l_control.ori = Quaternion::rotation_x(0.0);
        next.l_control.scale = Vec3::one();

        next.r_control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.r_control.ori = Quaternion::rotation_x(0.0);
        next.r_control.scale = Vec3::one();

        next
    }
}
