use super::{
    super::{vek::*, Animation},
    DragonSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct FlyAnimation;

impl Animation for FlyAnimation {
    type Dependency<'a> = (f32, f32);
    type Skeleton = DragonSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"dragon_fly\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "dragon_fly")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab: f32 = 12.0;

        let wave_ultra_slow = (anim_time * 1.0 + PI).sin();
        let wave_ultra_slow_cos = (anim_time * 3.0 + PI).cos();
        let wave_slow = (anim_time * 3.5 + PI).sin();

        let wingl = (anim_time * 2.0 + PI).sin();
        let wingr = (anim_time * 2.0).sin();

        let footl = (anim_time * lab + PI).sin();
        let footr = (anim_time * lab).sin();

        let center = (anim_time * lab + PI / 2.0).sin();
        let centeroffset = (anim_time * lab + PI * 1.5).sin();

        next.head_upper.scale = Vec3::one() * 1.05;
        next.head_lower.scale = Vec3::one() * 1.05;
        next.jaw.scale = Vec3::one() * 1.05;
        next.tail_front.scale = Vec3::one() * 0.98;
        next.tail_rear.scale = Vec3::one() * 0.98;

        next.head_upper.position = Vec3::new(
            0.0,
            s_a.head_upper.0,
            s_a.head_upper.1 + wave_ultra_slow * 0.20,
        );
        next.head_upper.orientation =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_ultra_slow * -0.10);

        next.head_lower.position = Vec3::new(
            0.0,
            s_a.head_lower.0,
            s_a.head_lower.1 + wave_ultra_slow * 0.20,
        );
        next.head_lower.orientation =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_ultra_slow * -0.10);

        next.jaw.position = Vec3::new(
            0.0,
            s_a.jaw.0 - wave_ultra_slow_cos * 0.12,
            s_a.jaw.1 + wave_slow * 0.2,
        );
        next.jaw.orientation = Quaternion::rotation_x(wave_slow * 0.03);

        next.tail_front.position =
            Vec3::new(0.0, s_a.tail_front.0, s_a.tail_front.1 + centeroffset * 0.6);
        next.tail_front.orientation = Quaternion::rotation_x(center * 0.03);

        next.tail_rear.position =
            Vec3::new(0.0, s_a.tail_rear.0, s_a.tail_rear.1 + centeroffset * 0.6);
        next.tail_rear.orientation = Quaternion::rotation_x(center * 0.03);

        next.chest_front.position = Vec3::new(0.0, s_a.chest_front.0, s_a.chest_front.1);
        next.chest_front.orientation = Quaternion::rotation_y(center * 0.05);

        next.chest_rear.position = Vec3::new(0.0, s_a.chest_rear.0, s_a.chest_rear.1);
        next.chest_rear.orientation = Quaternion::rotation_y(center * 0.05);

        next.foot_fl.position = Vec3::new(-s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2);
        next.foot_fl.orientation = Quaternion::rotation_x(-1.3 + footl * 0.06);

        next.foot_fr.position = Vec3::new(s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2);
        next.foot_fr.orientation = Quaternion::rotation_x(-1.3 + footr * 0.06);

        next.foot_bl.position = Vec3::new(-s_a.feet_b.0, s_a.feet_b.1, s_a.feet_b.2);
        next.foot_bl.orientation = Quaternion::rotation_x(-1.3 + footl * 0.06);

        next.foot_br.position = Vec3::new(s_a.feet_b.0, s_a.feet_b.1, s_a.feet_b.2);
        next.foot_br.orientation = Quaternion::rotation_x(-1.3 + footr * 0.06);

        next.wing_in_l.position = Vec3::new(-s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);
        next.wing_in_l.orientation = Quaternion::rotation_y(0.4 + wingl * 0.6);

        next.wing_in_r.position = Vec3::new(s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);
        next.wing_in_r.orientation = Quaternion::rotation_y(-0.4 + wingr * 0.6);

        next.wing_out_l.position = Vec3::new(-s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
        next.wing_out_l.orientation = Quaternion::rotation_y((0.35 + wingl * 0.6).max(0.2));

        next.wing_out_r.position = Vec3::new(s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
        next.wing_out_r.orientation = Quaternion::rotation_y((-0.35 + wingr * 0.6).min(-0.2));

        next
    }
}
