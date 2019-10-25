use super::{
    super::{Animation, SkeletonAttr},
    DragonSkeleton,
};
use std::f32::consts::PI;
use vek::*;

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Skeleton = DragonSkeleton;
    type Dependency = (f32, f64);

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        _skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave_ultra_slow = (anim_time as f32 * 1.0 + PI).sin();
        let wave_ultra_slow_cos = (anim_time as f32 * 1.0 + PI).cos();
        let wave_slow = (anim_time as f32 * 3.5 + PI).sin();
        let wave_slow_cos = (anim_time as f32 * 3.5 + PI).cos();

        next.dragon_head.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.dragon_head.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.dragon_head.scale = Vec3::one() / 10.88;

        next.dragon_chest_front.offset = Vec3::new(0.0, 4.5 - wave_ultra_slow_cos * 0.12, 2.0);
        next.dragon_chest_front.ori = Quaternion::rotation_x(0.0);
        next.dragon_chest_front.scale = Vec3::one() * 1.01;

        next.dragon_chest_rear.offset = Vec3::new(0.0, 4.5 - wave_ultra_slow_cos * 0.12, 2.0);
        next.dragon_chest_rear.ori = Quaternion::rotation_x(0.0);
        next.dragon_chest_rear.scale = Vec3::one() * 1.01;

        next.dragon_tail_front.offset = Vec3::new(0.0, 4.5 - wave_ultra_slow_cos * 0.12, 2.0);
        next.dragon_tail_front.ori = Quaternion::rotation_x(0.0);
        next.dragon_tail_front.scale = Vec3::one() * 1.01;

        next.dragon_tail_rear.offset = Vec3::new(0.0, 4.5 - wave_ultra_slow_cos * 0.12, 2.0);
        next.dragon_tail_rear.ori = Quaternion::rotation_x(0.0);
        next.dragon_tail_rear.scale = Vec3::one() * 1.01;

        next.dragon_wing_in_l.offset = Vec3::new(0.0, 4.5 - wave_ultra_slow_cos * 0.12, 2.0);
        next.dragon_wing_in_l.ori = Quaternion::rotation_x(0.0);
        next.dragon_wing_in_l.scale = Vec3::one() * 1.01;

        next.dragon_wing_in_r.offset = Vec3::new(0.0, 4.5 - wave_ultra_slow_cos * 0.12, 2.0);
        next.dragon_wing_in_r.ori = Quaternion::rotation_x(0.0);
        next.dragon_wing_in_r.scale = Vec3::one() * 1.01;

        next.dragon_wing_out_l.offset = Vec3::new(0.0, 4.5 - wave_ultra_slow_cos * 0.12, 2.0);
        next.dragon_wing_out_l.ori = Quaternion::rotation_x(0.0);
        next.dragon_wing_out_l.scale = Vec3::one() * 1.01;

        next.dragon_wing_out_r.offset = Vec3::new(0.0, 4.5 - wave_ultra_slow_cos * 0.12, 2.0);
        next.dragon_wing_out_r.ori = Quaternion::rotation_x(0.0);
        next.dragon_wing_out_r.scale = Vec3::one() * 1.01;

        next.dragon_foot_fl.offset = Vec3::new(0.0, 4.5 - wave_ultra_slow_cos * 0.12, 2.0);
        next.dragon_foot_fl.ori = Quaternion::rotation_x(0.0);
        next.dragon_foot_fl.scale = Vec3::one() * 1.01;

        next.dragon_foot_fr.offset = Vec3::new(0.0, 4.5 - wave_ultra_slow_cos * 0.12, 2.0);
        next.dragon_foot_fr.ori = Quaternion::rotation_x(0.0);
        next.dragon_foot_fr.scale = Vec3::one() * 1.01;

        next.dragon_foot_bl.offset = Vec3::new(0.0, 4.5 - wave_ultra_slow_cos * 0.12, 2.0);
        next.dragon_foot_bl.ori = Quaternion::rotation_x(0.0);
        next.dragon_foot_bl.scale = Vec3::one() * 1.01;

        next.dragon_foot_br.offset = Vec3::new(0.0, 4.5 - wave_ultra_slow_cos * 0.12, 2.0);
        next.dragon_foot_br.ori = Quaternion::rotation_x(0.0);
        next.dragon_foot_br.scale = Vec3::one() * 1.01;
        next
    }
}
