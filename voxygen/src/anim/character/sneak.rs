use super::{
    super::{Animation, SkeletonAttr},
    CharacterSkeleton,
};
use std::f32::consts::PI;
use std::ops::Mul;
use vek::*;

pub struct SneakAnimation;

impl Animation<'_>for SneakAnimation {
    type Skeleton = CharacterSkeleton;
    type Dependency = (Vec3<f32>, Vec3<f32>, Vec3<f32>, f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (velocity, orientation, last_ori, global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let speed = Vec2::<f32>::from(velocity).magnitude();
        *rate = speed;

        let constant = 1.0;
        let wave = (((5.0)
            / (1.1 + 3.9 * ((anim_time as f32 * constant as f32 * 1.2).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * constant as f32 * 1.2).sin());
        let wavecos = (((5.0)
            / (1.1 + 3.9 * ((anim_time as f32 * constant as f32 * 1.2).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * constant as f32 * 1.2).cos());
        let wave_cos = (((5.0)
            / (1.1 + 3.9 * ((anim_time as f32 * constant as f32 * 2.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * constant as f32 * 1.5).sin());
        let wave_cos_dub = (((5.0)
            / (1.1 + 3.9 * ((anim_time as f32 * constant as f32 * 4.8).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * constant as f32 * 1.5).sin());
        let wave_slow = (anim_time as f32 * 0.1).sin();
        let wave_diff = (anim_time as f32 * 0.6).sin();
        let wave_stop = (anim_time as f32 * 2.6).min(PI / 2.0).sin();
        let head_look = Vec2::new(
            ((global_time + anim_time) as f32 * 0.25)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.4,
            ((global_time + anim_time) as f32 * 0.25)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.2,
        );

        let ori = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);

        let tilt = if Vec2::new(ori, last_ori)
            .map(|o| Vec2::<f32>::from(o).magnitude_squared())
            .map(|m| m > 0.001 && m.is_finite())
            .reduce_and()
            && ori.angle_between(last_ori).is_finite()
        {
            ori.angle_between(last_ori).min(0.5)
                * last_ori.determine_side(Vec2::zero(), ori).signum()
        } else {
            0.0
        } * 1.3;

        next.head.offset = Vec3::new(
            0.0,
            0.0 + skeleton_attr.neck_forward,
            skeleton_attr.neck_height + 16.0,
        );
        next.head.ori = Quaternion::rotation_z(head_look.x + wave * 0.1)
            * Quaternion::rotation_x(head_look.y + 0.05);
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(0.0, -1.5, 3.0 + wave_slow * 2.0);
        next.chest.ori = Quaternion::rotation_x(-0.5) * Quaternion::rotation_z(wave * 0.15);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0, 0.0, 1.5 + wave_cos * 0.3);
        next.belt.ori = Quaternion::rotation_x(-0.1) * Quaternion::rotation_z(wave * 0.25);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(0.0, 1.0, -1.0 + wave_cos * 0.3);
        next.shorts.ori = Quaternion::rotation_x(0.2) * Quaternion::rotation_z(wave * 0.4);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(-5.0 + wave_stop * -0.5, 2.25, 4.0 - wave * 1.0);
        next.l_hand.ori =
            Quaternion::rotation_x(1.5 + wave_cos * 0.1) * Quaternion::rotation_y(wave_stop * 0.1);
        next.l_hand.scale = Vec3::one();

        next.r_hand.offset = Vec3::new(5.0 + wave_stop * 0.5, 2.25, 4.0 + wave * 1.0);
        next.r_hand.ori = Quaternion::rotation_x(1.5 + wave_cos * -0.1)
            * Quaternion::rotation_y(wave_stop * -0.1);
        next.r_hand.scale = Vec3::one();

        next.l_foot.offset = Vec3::new(-3.4, 5.0 + wave * -3.0, 4.0);
        next.l_foot.ori = Quaternion::rotation_x(-0.8 + wavecos * 0.15);
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(3.4, 5.0 + wave * 3.0, 4.0);
        next.r_foot.ori = Quaternion::rotation_x(-0.8 - wavecos * 0.15);
        next.r_foot.scale = Vec3::one();

        next.weapon.offset = Vec3::new(
            -7.0 + skeleton_attr.weapon_x,
            -5.0 + skeleton_attr.weapon_y,
            15.0,
        );
        next.weapon.ori =
            Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57 + wave_cos * 0.25);
        next.weapon.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(-5.0, 0.0, 4.7);
        next.l_shoulder.ori = Quaternion::rotation_x(wavecos * 0.05);
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(5.0, 0.0, 4.7);
        next.r_shoulder.ori = Quaternion::rotation_x(wave * 0.05);
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.draw.offset = Vec3::new(0.0, 5.0, 0.0);
        next.draw.ori = Quaternion::rotation_y(0.0);
        next.draw.scale = Vec3::one() * 0.0;

        next.torso.offset = Vec3::new(0.0, 0.3 + wave * -0.08, 0.4) * skeleton_attr.scaler;
        next.torso.ori =
            Quaternion::rotation_x(wave_stop * speed * -0.03 + wave_diff * speed * -0.005)
                * Quaternion::rotation_y(tilt);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next
    }
}
