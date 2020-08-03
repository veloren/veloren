use super::{
    super::{vek::*, Animation},
    DragonSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct FlyAnimation;

impl Animation for FlyAnimation {
    type Dependency = (f32, f64);
    type Skeleton = DragonSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"dragon_fly\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "dragon_fly")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab = 12.0;

        let wave_ultra_slow = (anim_time as f32 * 1.0 + PI).sin();
        let wave_ultra_slow_cos = (anim_time as f32 * 3.0 + PI).cos();
        let wave_slow = (anim_time as f32 * 3.5 + PI).sin();

        let wingl = (anim_time as f32 * 2.0 + PI).sin();
        let wingr = (anim_time as f32 * 2.0).sin();

        let footl = (anim_time as f32 * lab as f32 + PI).sin();
        let footr = (anim_time as f32 * lab as f32).sin();

        let center = (anim_time as f32 * lab as f32 + PI / 2.0).sin();
        let centeroffset = (anim_time as f32 * lab as f32 + PI * 1.5).sin();

        next.head_upper.offset = Vec3::new(
            0.0,
            skeleton_attr.head_upper.0,
            skeleton_attr.head_upper.1 + wave_ultra_slow * 0.20,
        );
        next.head_upper.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_ultra_slow * -0.10);
        next.head_upper.scale = Vec3::one() * 1.05;

        next.head_lower.offset = Vec3::new(
            0.0,
            skeleton_attr.head_lower.0,
            skeleton_attr.head_lower.1 + wave_ultra_slow * 0.20,
        );
        next.head_lower.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_ultra_slow * -0.10);
        next.head_lower.scale = Vec3::one() * 1.05;

        next.jaw.offset = Vec3::new(
            0.0,
            skeleton_attr.jaw.0 - wave_ultra_slow_cos * 0.12,
            skeleton_attr.jaw.1 + wave_slow * 0.2,
        );
        next.jaw.ori = Quaternion::rotation_x(wave_slow * 0.03);
        next.jaw.scale = Vec3::one() * 1.05;

        next.tail_front.offset = Vec3::new(
            0.0,
            skeleton_attr.tail_front.0,
            skeleton_attr.tail_front.1 + centeroffset * 0.6,
        );
        next.tail_front.ori = Quaternion::rotation_x(center * 0.03);
        next.tail_front.scale = Vec3::one() * 0.98;

        next.tail_rear.offset = Vec3::new(
            0.0,
            skeleton_attr.tail_rear.0,
            skeleton_attr.tail_rear.1 + centeroffset * 0.6,
        );
        next.tail_rear.ori = Quaternion::rotation_x(center * 0.03);
        next.tail_rear.scale = Vec3::one() * 0.98;

        next.chest_front.offset = Vec3::new(
            0.0,
            skeleton_attr.chest_front.0,
            skeleton_attr.chest_front.1,
        );
        next.chest_front.ori = Quaternion::rotation_y(center * 0.05);
        next.chest_front.scale = Vec3::one();

        next.chest_rear.offset =
            Vec3::new(0.0, skeleton_attr.chest_rear.0, skeleton_attr.chest_rear.1);
        next.chest_rear.ori = Quaternion::rotation_y(center * 0.05);
        next.chest_rear.scale = Vec3::one();

        next.foot_fl.offset = Vec3::new(
            -skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2,
        );
        next.foot_fl.ori = Quaternion::rotation_x(-1.3 + footl * 0.06);
        next.foot_fl.scale = Vec3::one();

        next.foot_fr.offset = Vec3::new(
            skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2,
        );
        next.foot_fr.ori = Quaternion::rotation_x(-1.3 + footr * 0.06);
        next.foot_fr.scale = Vec3::one();

        next.foot_bl.offset = Vec3::new(
            -skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2,
        );
        next.foot_bl.ori = Quaternion::rotation_x(-1.3 + footl * 0.06);
        next.foot_bl.scale = Vec3::one();

        next.foot_br.offset = Vec3::new(
            skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2,
        );
        next.foot_br.ori = Quaternion::rotation_x(-1.3 + footr * 0.06);
        next.foot_br.scale = Vec3::one();

        next.wing_in_l.offset = Vec3::new(
            -skeleton_attr.wing_in.0,
            skeleton_attr.wing_in.1,
            skeleton_attr.wing_in.2,
        );
        next.wing_in_l.ori = Quaternion::rotation_y(0.4 + wingl * 0.6);
        next.wing_in_l.scale = Vec3::one();

        next.wing_in_r.offset = Vec3::new(
            skeleton_attr.wing_in.0,
            skeleton_attr.wing_in.1,
            skeleton_attr.wing_in.2,
        );
        next.wing_in_r.ori = Quaternion::rotation_y(-0.4 + wingr * 0.6);
        next.wing_in_r.scale = Vec3::one();

        next.wing_out_l.offset = Vec3::new(
            -skeleton_attr.wing_out.0,
            skeleton_attr.wing_out.1,
            skeleton_attr.wing_out.2,
        );
        next.wing_out_l.ori = Quaternion::rotation_y((0.35 + wingl * 0.6).max(0.2));
        next.wing_out_l.scale = Vec3::one();

        next.wing_out_r.offset = Vec3::new(
            skeleton_attr.wing_out.0,
            skeleton_attr.wing_out.1,
            skeleton_attr.wing_out.2,
        );
        next.wing_out_r.ori = Quaternion::rotation_y((-0.35 + wingr * 0.6).min(-0.2));
        next.wing_out_r.scale = Vec3::one();

        next
    }
}
