use super::{super::Animation, DragonSkeleton, SkeletonAttr};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct FlyAnimation;

#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.1)]
const TEST_1: f32 = 0.0;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.1)]
const TEST_2: f32 = 0.0;
#[const_tweaker::tweak(min = -1.0, max = 1.0, step = 0.01)]
const TEST_3: f32 = 0.0;
#[const_tweaker::tweak(min = -1.0, max = 1.0, step = 0.01)]
const TEST_4: f32 = 0.0;

impl Animation for FlyAnimation {
    type Dependency = (f32, f64);
    type Skeleton = DragonSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (_velocity, global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab = 12.0;

        let wave_ultra_slow = (anim_time as f32 * 1.0 + PI).sin();
        let wave_ultra_slow_cos = (anim_time as f32 * 3.0 + PI).cos();
        let wave_slow = (anim_time as f32 * 4.5).sin();
        let wave_slow_cos = (anim_time as f32 * 4.5).cos();

        let wingl = (anim_time as f32 * 2.0 + PI).sin();
        let wingr = (anim_time as f32 * 2.0).sin();

        let vertlf = (anim_time as f32 * lab as f32 + PI * 1.8).sin().max(0.15);
        let vertrfoffset = (anim_time as f32 * lab as f32 + PI * 0.80).sin().max(0.15);
        let vertlboffset = (anim_time as f32 * lab as f32).sin().max(0.15);
        let vertrb = (anim_time as f32 * lab as f32 + PI).sin().max(0.15);

        let horilf = (anim_time as f32 * lab as f32 + PI * 1.2).sin();
        let horirfoffset = (anim_time as f32 * lab as f32 + PI * 0.20).sin();
        let horilboffset = (anim_time as f32 * lab as f32 + PI * 1.4).sin();
        let horirb = (anim_time as f32 * lab as f32 + PI * 0.4).sin();

        let vertchest = (anim_time as f32 * lab as f32 + PI * 0.3).sin().max(0.2);
        let horichest = (anim_time as f32 * lab as f32 + PI * 0.8).sin();
        let verthead = (anim_time as f32 * lab as f32 + PI * 0.3).sin();

        let footl = (anim_time as f32 * lab as f32 + PI).sin();
        let footr = (anim_time as f32 * lab as f32).sin();

        let center = (anim_time as f32 * lab as f32 + PI / 2.0).sin();
        let centeroffset = (anim_time as f32 * lab as f32 + PI * 1.5).sin();

        let wolf_look = Vec2::new(
            ((global_time + anim_time) as f32 / 4.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.25,
            ((global_time + anim_time) as f32 / 4.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.125,
        );

        let wave = (anim_time as f32 * 14.0).sin();
        let wave_slow = (anim_time as f32 * 3.5 + PI).sin();
        let wave_stop = (anim_time as f32 * 5.0).min(PI / 2.0).sin();

        next.head_upper.offset = Vec3::new(
            0.0,
            skeleton_attr.head_upper.0,
            skeleton_attr.head_upper.1 + wave_ultra_slow * 0.20,
        ) * 1.05;
        next.head_upper.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_ultra_slow * -0.10);
        next.head_upper.scale = Vec3::one() * 1.05;

        next.head_lower.offset = Vec3::new(
            0.0,
            skeleton_attr.head_lower.0,
            skeleton_attr.head_lower.1 + wave_ultra_slow * 0.20,
        ) * 1.05;
        next.head_lower.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(wave_ultra_slow * -0.10);
        next.head_lower.scale = Vec3::one() * 1.05;

        next.jaw.offset = Vec3::new(
            0.0,
            skeleton_attr.jaw.0 - wave_ultra_slow_cos * 0.12,
            skeleton_attr.jaw.1 + wave_slow * 0.2,
        );
        next.jaw.ori = Quaternion::rotation_x(wave_slow * 0.05);
        next.jaw.scale = Vec3::one() * 0.98;

        next.tail_front.offset = Vec3::new(
            0.0,
            skeleton_attr.tail_front.0,
            skeleton_attr.tail_front.1 + centeroffset * 0.6,
        );
        next.tail_front.ori = Quaternion::rotation_x(center * 0.03);
        next.tail_front.scale = Vec3::one();

        next.tail_rear.offset = Vec3::new(
            0.0,
            skeleton_attr.tail_rear.0,
            skeleton_attr.tail_rear.1 + centeroffset * 0.6,
        );
        next.tail_rear.ori = Quaternion::rotation_x(center * 0.03);
        next.tail_rear.scale = Vec3::one();

        next.chest_front.offset = Vec3::new(
            0.0,
            skeleton_attr.chest_front.0,
            skeleton_attr.chest_front.1,
        ) * 1.05;
        next.chest_front.ori = Quaternion::rotation_y(center * 0.05);
        next.chest_front.scale = Vec3::one() * 1.05;

        next.chest_rear.offset = Vec3::new(
            0.0,
            skeleton_attr.chest_rear.0,
            skeleton_attr.chest_rear.1,
        ) * 1.05;
        next.chest_rear.ori = Quaternion::rotation_y(center * 0.05);
        next.chest_rear.scale = Vec3::one() * 1.05;

        next.foot_fl.offset = Vec3::new(
            -skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2,
        ) * 1.05;
        next.foot_fl.ori = Quaternion::rotation_x(-1.3 + footl * 0.06);
        next.foot_fl.scale = Vec3::one() * 1.05;

        next.foot_fr.offset = Vec3::new(
            skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2,
        ) * 1.05;
        next.foot_fr.ori = Quaternion::rotation_x(-1.3 + footl * 0.06);
        next.foot_fr.scale = Vec3::one() * 1.05;

        next.foot_bl.offset = Vec3::new(
            -skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2,
        ) * 1.05;
        next.foot_bl.ori = Quaternion::rotation_x(-1.3 + footl * 0.06);
        next.foot_bl.scale = Vec3::one() * 1.05;

        next.foot_br.offset = Vec3::new(
            skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2,
        ) * 1.05;
        next.foot_br.ori = Quaternion::rotation_x(-1.3 + footl * 0.06);
        next.foot_br.scale = Vec3::one() * 1.05;

        next.wing_in_l.offset = Vec3::new(
            -skeleton_attr.wing_in.0,
            skeleton_attr.wing_in.1,
            skeleton_attr.wing_in.2,
        );
        next.wing_in_l.ori = Quaternion::rotation_y((0.15 + wingl * 0.6).max(0.2));
        next.wing_in_l.scale = Vec3::one() * 1.05;

        next.wing_in_r.offset = Vec3::new(
            skeleton_attr.wing_in.0,
            skeleton_attr.wing_in.1,
            skeleton_attr.wing_in.2,
        );
        next.wing_in_r.ori = Quaternion::rotation_y((-0.15 + wingr * 0.6).min(0.2));
        next.wing_in_r.scale = Vec3::one() * 1.05;

        next.wing_out_l.offset = Vec3::new(
            -skeleton_attr.wing_out.0,
            skeleton_attr.wing_out.1,
            skeleton_attr.wing_out.2,
        );
        next.wing_out_l.ori = Quaternion::rotation_y((0.35 + wingl * 0.6).max(0.0));
        next.wing_out_l.scale = Vec3::one() * 1.05;

        next.wing_out_r.offset = Vec3::new(
            skeleton_attr.wing_out.0,
            skeleton_attr.wing_out.1,
            skeleton_attr.wing_out.2,
        );
        next.wing_out_r.ori = Quaternion::rotation_y((-0.35 + wingr * 0.6).min(0.0));
        next.wing_out_r.scale = Vec3::one() * 1.05;

        next
    }
}
