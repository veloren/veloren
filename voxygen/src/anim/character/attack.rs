// Standard
use std::{f32::consts::PI, ops::Mul};

// Library
use vek::*;

// Local
use super::{super::Animation, CharacterSkeleton, SCALE};

pub struct Input {
    pub attack: bool,
}
pub struct AttackAnimation;

impl Animation for AttackAnimation {
    type Skeleton = CharacterSkeleton;
    type Dependency = f64;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        global_time: f64,
        anim_time: f64,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 1.0).sin();
        let wave_cos = (anim_time as f32 * 12.0).cos();
        let wave_slow = (anim_time as f32 * 6.0 + PI).sin();
        let wave_slow_cos = (anim_time as f32 * 6.0 + PI).cos();
        let wave_ultra_slow = (anim_time as f32 * 1.0 + PI).sin();
        let wave_ultra_slow_cos = (anim_time as f32 * 1.0 + PI).cos();
        let wave_stop = (anim_time as f32 * 3.0).min(PI / 2.0).sin();
        let wave_stop_alt = (anim_time as f32 * 28.0).min(PI / 2.0).sin();
        let wave_stop_quick = (anim_time as f32 * 8.0).min(PI / 2.0).sin();
        let peakwave = 1.0- (anim_time as f32 * 1.0).cos();


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
        next.head.offset = Vec3::new(0.0, 2.0, 11.0);
        next.head.ori = Quaternion::rotation_z(0.0);
        next.head.scale = Vec3::one();

        next.chest.offset = Vec3::new(0.0, 0.0, 7.0);
        next.chest.ori = Quaternion::rotation_x(0.0);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0, 0.0, 5.0);
        next.belt.ori = Quaternion::rotation_x(0.0);
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(0.0, 0.0, 2.0);
        next.shorts.ori = Quaternion::rotation_x(0.0);
        next.shorts.scale = Vec3::one();

        next.l_hand.offset = Vec3::new(
            -8.0,
            4.0,
            9.0,
        ) / 11.0;
        next.l_hand.ori = Quaternion::rotation_x(0.0 + wave * 2.0 )* Quaternion::rotation_z(wave * 2.0);
        next.l_hand.scale = Vec3::one() / 11.0;

        next.r_hand.offset = Vec3::new(
            8.0,
            4.0,
            6.5,
        ) / 11.0;
        next.r_hand.ori = Quaternion::rotation_x(0.0);
        next.r_hand.scale = Vec3::one() / 11.0;

        next.l_foot.offset = Vec3::new(-3.3, -0.1, 8.0);
        next.l_foot.ori = Quaternion::identity();
        next.l_foot.scale = Vec3::one();

        next.r_foot.offset = Vec3::new(4.1, -0.1, 8.0);
        next.r_foot.ori = Quaternion::identity();
        next.r_foot.scale = Vec3::one();

        next.weapon.offset = Vec3::new(-7.0, -5.0, 15.0);
        next.weapon.ori = Quaternion::rotation_y(2.5);
        next.weapon.scale = Vec3::one();

        next.l_shoulder.offset = Vec3::new(-10.0, -3.2, 2.5);
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = Vec3::one() * 1.04;

        next.r_shoulder.offset = Vec3::new(0.0, -3.2, 2.5);
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = Vec3::one() * 1.04;

        next.draw.offset = Vec3::new(0.0, 5.0, 0.0);
        next.draw.ori = Quaternion::rotation_y(0.0);
        next.draw.scale = Vec3::one() * 0.0;

        next.left_equip.offset = Vec3::new(-8.0, 4.0, 9.0) / 11.0;
        next.left_equip.ori = Quaternion::rotation_x(0.0 + wave * 2.0)* Quaternion::rotation_z(1.57 + wave * 2.0);
        next.left_equip.scale = Vec3::one() / 11.0;

        next.right_equip.offset = Vec3::new(0.0, 0.0, 5.0) / 11.0;
        next.right_equip.ori = Quaternion::rotation_x(0.0);;
        next.right_equip.scale = Vec3::one() * 0.0;
        
        next.torso.offset = Vec3::new(0.0, -0.2, 0.1);
        next.torso.ori = Quaternion::rotation_x(0.0);
        next.torso.scale = Vec3::one() / 11.0;
        next
    }
}
