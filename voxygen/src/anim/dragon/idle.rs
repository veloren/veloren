use super::{super::Animation, DragonSkeleton, SkeletonAttr};
use std::ops::Mul;
use vek::*;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency = f64;
    type Skeleton = DragonSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave_slow = (anim_time as f32 * 4.5).sin();
        let wave_slow_cos = (anim_time as f32 * 4.5).cos();

        let duck_head_look = Vec2::new(
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

        next.head.offset = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1);
        next.head.ori = Quaternion::rotation_z(duck_head_look.x)
            * Quaternion::rotation_x(-duck_head_look.y.abs() + wave_slow_cos * 0.03);
        next.head.scale = Vec3::one();

        next.chest_front.offset = Vec3::new(
            0.0,
            skeleton_attr.chest_front.0,
            wave_slow * 0.3 + skeleton_attr.chest_front.1,
        ) * 1.05;
        next.chest_front.ori = Quaternion::rotation_y(wave_slow * 0.03);
        next.chest_front.scale = Vec3::one() * 1.05;

        next.chest_rear.offset = Vec3::new(
            0.0,
            skeleton_attr.chest_rear.0,
            wave_slow * 0.3 + skeleton_attr.chest_rear.1,
        ) * 1.05;
        next.chest_rear.ori = Quaternion::rotation_y(wave_slow * 0.03);
        next.chest_rear.scale = Vec3::one() * 1.05;

        next.tail_front.offset = Vec3::new(0.0, skeleton_attr.tail_front.0, skeleton_attr.tail_front.1);
        next.tail_front.ori = Quaternion::rotation_x(wave_slow_cos * 0.03);
        next.tail_front.scale = Vec3::one();

        next.tail_rear.offset = Vec3::new(0.0, skeleton_attr.tail_rear.0, skeleton_attr.tail_rear.1);
        next.tail_rear.ori = Quaternion::rotation_x(wave_slow_cos * 0.03);
        next.tail_rear.scale = Vec3::one();

        next.wing_in_l.offset = Vec3::new(
            -skeleton_attr.wing_in.0,
            skeleton_attr.wing_in.1,
            skeleton_attr.wing_in.2,
        );
        next.wing_in_l.ori = Quaternion::rotation_z(0.0);
        next.wing_in_l.scale = Vec3::one() * 1.05;

        next.wing_in_r.offset = Vec3::new(
            skeleton_attr.wing_in.0,
            skeleton_attr.wing_in.1,
            skeleton_attr.wing_in.2,
        );
        next.wing_in_r.ori = Quaternion::rotation_y(0.0);
        next.wing_in_r.scale = Vec3::one() * 1.05;

        next.wing_out_l.offset = Vec3::new(
            -skeleton_attr.wing_out.0,
            skeleton_attr.wing_out.1,
            skeleton_attr.wing_out.2,
        );
        next.wing_out_l.ori = Quaternion::rotation_z(0.0);
        next.wing_out_l.scale = Vec3::one() * 1.05;

        next.wing_in_r.offset = Vec3::new(
            skeleton_attr.wing_out.0,
            skeleton_attr.wing_out.1,
            skeleton_attr.wing_out.2,
        );
        next.wing_out_r.ori = Quaternion::rotation_y(0.0);
        next.wing_out_r.scale = Vec3::one() * 1.05;

        next.foot_fl.offset = Vec3::new(
            -skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2,
        ) * 1.05;
        next.foot_fl.ori = Quaternion::rotation_x(0.0);
        next.foot_fl.scale = Vec3::one() * 1.05;

        next.foot_fr.offset = Vec3::new(
            skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2,
        ) * 1.05;
        next.foot_fr.ori = Quaternion::rotation_x(0.0);
        next.foot_fr.scale = Vec3::one() * 1.05;

        next.foot_bl.offset = Vec3::new(
            -skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2,
        ) * 1.05;
        next.foot_bl.ori = Quaternion::rotation_x(0.0);
        next.foot_bl.scale = Vec3::one() * 1.05;

        next.foot_br.offset = Vec3::new(
            skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1,
            skeleton_attr.feet_b.2,
        ) * 1.05;
        next.foot_br.ori = Quaternion::rotation_x(0.0);
        next.foot_br.scale = Vec3::one() * 1.05;

        next
    }
}
