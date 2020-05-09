use super::{super::Animation, DragonSkeleton, SkeletonAttr};
use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
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

        let lab = 14;

        let wave_ultra_slow_cos = (anim_time as f32 * 3.0 + PI).cos();
        let wave_slow = (anim_time as f32 * 4.5).sin();

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

        let center = (anim_time as f32 * lab as f32 + PI / 2.0).sin();
        let centeroffset = (anim_time as f32 * lab as f32 + PI * 1.5).sin();

        let dragon_look = Vec2::new(
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

        next.head_upper.offset =
            Vec3::new(0.0, skeleton_attr.head_upper.0, skeleton_attr.head_upper.1);
        next.head_upper.ori =
            Quaternion::rotation_x(dragon_look.y) * Quaternion::rotation_z(dragon_look.x);
        next.head_upper.scale = Vec3::one();

        next.head_lower.offset =
            Vec3::new(0.0, skeleton_attr.head_lower.0, skeleton_attr.head_lower.1);
        next.head_lower.ori = Quaternion::rotation_x(wave_slow * 0.05);
        next.head_lower.scale = Vec3::one();

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
            skeleton_attr.chest_front.0 + horichest * 1.25,
            skeleton_attr.chest_front.1 + vertchest * -1.6 + 1.0,
        );
        next.chest_front.ori = Quaternion::rotation_y(horichest * -0.09);
        next.chest_front.scale = Vec3::one();

        next.chest_rear.offset =
            Vec3::new(0.0, skeleton_attr.chest_rear.0, skeleton_attr.chest_rear.1);
        next.chest_rear.ori = Quaternion::rotation_y(horichest * -0.09);
        next.chest_rear.scale = Vec3::one();

        next.foot_fl.offset = Vec3::new(
            -skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1 + horilf * 2.5,
            skeleton_attr.feet_f.2 + vertlf * 5.0 * skeleton_attr.height - 0.5,
        );
        next.foot_fl.ori = Quaternion::rotation_x(horilf * 0.4);
        next.foot_fl.scale = Vec3::one();

        next.foot_fr.offset = Vec3::new(
            skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1 + horirfoffset * 2.5,
            skeleton_attr.feet_f.2 + vertrfoffset * 5.0 * skeleton_attr.height - 0.5,
        );
        next.foot_fr.ori = Quaternion::rotation_x(horirfoffset * 0.4);
        next.foot_fr.scale = Vec3::one();

        next.foot_bl.offset = Vec3::new(
            -skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1 + horilboffset * 3.0,
            skeleton_attr.feet_b.2 + vertlboffset * 5.0 * skeleton_attr.height - 0.5,
        );
        next.foot_bl.ori = Quaternion::rotation_x(horilboffset * 0.35);
        next.foot_bl.scale = Vec3::one();

        next.foot_br.offset = Vec3::new(
            skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1 + horirb * 3.0,
            skeleton_attr.feet_b.2 + vertrb * 5.0 * skeleton_attr.height - 0.5,
        );
        next.foot_br.ori = Quaternion::rotation_x(horirb * 0.35);
        next.foot_br.scale = Vec3::one();

        next.wing_in_l.offset = Vec3::new(
            -skeleton_attr.wing_in.0,
            skeleton_attr.wing_in.1,
            skeleton_attr.wing_in.2,
        );
        next.wing_in_l.ori = Quaternion::rotation_y(0.8);
        next.wing_in_l.scale = Vec3::one() * 1.05;

        next.wing_in_r.offset = Vec3::new(
            skeleton_attr.wing_in.0,
            skeleton_attr.wing_in.1,
            skeleton_attr.wing_in.2,
        );
        next.wing_in_r.ori = Quaternion::rotation_y(-0.8);
        next.wing_in_r.scale = Vec3::one();

        next.wing_out_l.offset = Vec3::new(
            -skeleton_attr.wing_out.0,
            skeleton_attr.wing_out.1,
            skeleton_attr.wing_out.2 - 1.4,
        );
        next.wing_out_l.ori = Quaternion::rotation_y(-2.0);
        next.wing_out_l.scale = Vec3::one();

        next.wing_out_r.offset = Vec3::new(
            skeleton_attr.wing_out.0,
            skeleton_attr.wing_out.1,
            skeleton_attr.wing_out.2 - 1.4,
        );
        next.wing_out_r.ori = Quaternion::rotation_y(2.0);
        next.wing_out_r.scale = Vec3::one();

        next
    }
}
