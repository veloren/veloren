// Standard
use std::{f32::consts::PI, ops::Mul};

// Library
use vek::*;

// Local
use super::{super::Animation, CharacterSkeleton, SCALE};

pub struct Input {
    pub attack: bool,
}
pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Skeleton = CharacterSkeleton;
    type Dependency = f64;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        global_time: f64,
        anim_time: f64,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 12.0).sin();
        let wave_cos = (anim_time as f32 * 12.0).cos();
        let wave_slow = (anim_time as f32 * 6.0 + PI).sin();
        let wave_slow_cos = (anim_time as f32 * 6.0 + PI).cos();
        let wave_ultra_slow = (anim_time as f32 * 1.0 + PI).sin();
        let wave_ultra_slow_cos = (anim_time as f32 * 1.0 + PI).cos();
        let wave_dip = (wave_slow.abs() - 0.5).abs();

        let head_look = Vec2::new(
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
        next.head.offset = Vec3::new(5.5, 2.0, 11.0 + wave_ultra_slow * 0.3);
        next.head.ori = Quaternion::rotation_z(head_look.x) * Quaternion::rotation_x(head_look.y);
        next.head.scale = Vec3::one();

        next.chest.offset = Vec3::new(5.5, 0.0, 7.0 + wave_ultra_slow * 0.3);
        next.chest.ori = Quaternion::rotation_x(0.0);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(5.5, 0.0, 5.0 + wave_ultra_slow * 0.3);
        next.belt.ori = Quaternion::rotation_x(0.0);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(5.5, 0.0, 2.0 + wave_ultra_slow * 0.3);
        next.shorts.ori = Quaternion::rotation_x(0.0);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(
            -6.0,
            -2.0 + wave_ultra_slow_cos * 0.15,
            11.5 + wave_ultra_slow * 0.5,
        ) / 11.;

        next.l_hand.ori = Quaternion::rotation_x(0.0 + wave_ultra_slow * 0.06);
        next.l_hand.scale = Vec3::one() / 11.;

        next.r_hand.offset = Vec3::new(
            9.0,
            -2.0 + wave_ultra_slow_cos * 0.15,
            11.5 + wave_ultra_slow * 0.5,
        ) / 11.;
        next.r_hand.ori = Quaternion::rotation_x(0.0 + wave_ultra_slow * 0.06);
        next.r_hand.scale = Vec3::one() / 11.;

        next.l_foot.offset = Vec3::new(-3.3, -0.1, 8.0);
        next.l_foot.ori = Quaternion::identity();
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(4.1, -0.1, 8.0);
        next.r_foot.ori = Quaternion::identity();
        next.r_foot.scale = Vec3::one();

        next.weapon.offset = Vec3::new(-5.0, -5.0, 12.0);
        next.weapon.ori = Quaternion::rotation_y(2.5);
        next.weapon.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(-10.0, -3.0, 2.5);
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = Vec3::one();

        next.r_shoulder.offset = Vec3::new(0.0, -3.0, 2.5);
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = Vec3::one();

        next.draw.offset = Vec3::new(13.5, 0.0, 0.0);
        next.draw.ori = Quaternion::rotation_y(0.0);
        next.draw.scale = Vec3::one() * 0.0;
        

        next.torso.offset = Vec3::new(-0.5, -0.2, 0.1);
        next.torso.ori = Quaternion::rotation_x(0.0);
        next.torso.scale = Vec3::one() / 11.0;


        next
    }
}
