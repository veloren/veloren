use super::{
    super::{Animation, SkeletonAttr},
    DragonSkeleton,
};
//use std::{f32::consts::PI, ops::Mul};
use vek::*;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Skeleton = DragonSkeleton;
    type Dependency = f64;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        _anim_time: f64,
        _rate: &mut f32,
        _skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        next.head.offset = Vec3::new(0.0, 7.5, 15.0) / 11.0;
        next.head.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.head.scale = Vec3::one() / 10.88;

        next.chest_front.offset = Vec3::new(0.0, 4.5, 2.0);
        next.chest_front.ori = Quaternion::rotation_x(0.0);
        next.chest_front.scale = Vec3::one() * 1.01;

        next.chest_rear.offset = Vec3::new(0.0, 4.5, 2.0);
        next.chest_rear.ori = Quaternion::rotation_x(0.0);
        next.chest_rear.scale = Vec3::one() * 1.01;

        next.tail_front.offset = Vec3::new(0.0, 4.5, 2.0);
        next.tail_front.ori = Quaternion::rotation_x(0.0);
        next.tail_front.scale = Vec3::one() * 1.01;

        next.tail_rear.offset = Vec3::new(0.0, 4.5, 2.0);
        next.tail_rear.ori = Quaternion::rotation_x(0.0);
        next.tail_rear.scale = Vec3::one() * 1.01;

        next.wing_in_l.offset = Vec3::new(0.0, 4.5, 2.0);
        next.wing_in_l.ori = Quaternion::rotation_x(0.0);
        next.wing_in_l.scale = Vec3::one() * 1.01;

        next.wing_in_r.offset = Vec3::new(0.0, 4.5, 2.0);
        next.wing_in_r.ori = Quaternion::rotation_x(0.0);
        next.wing_in_r.scale = Vec3::one() * 1.01;

        next.wing_out_l.offset = Vec3::new(0.0, 4.5, 2.0);
        next.wing_out_l.ori = Quaternion::rotation_x(0.0);
        next.wing_out_l.scale = Vec3::one() * 1.01;

        next.wing_out_r.offset = Vec3::new(0.0, 4.5, 2.0);
        next.wing_out_r.ori = Quaternion::rotation_x(0.0);
        next.wing_out_r.scale = Vec3::one() * 1.01;

        next.foot_fl.offset = Vec3::new(0.0, 4.5, 2.0);
        next.foot_fl.ori = Quaternion::rotation_x(0.0);
        next.foot_fl.scale = Vec3::one() * 1.01;

        next.foot_fr.offset = Vec3::new(0.0, 4.5, 2.0);
        next.foot_fr.ori = Quaternion::rotation_x(0.0);
        next.foot_fr.scale = Vec3::one() * 1.01;

        next.foot_bl.offset = Vec3::new(0.0, 4.5, 2.0);
        next.foot_bl.ori = Quaternion::rotation_x(0.0);
        next.foot_bl.scale = Vec3::one() * 1.01;

        next.foot_br.offset = Vec3::new(0.0, 4.5, 2.0);
        next.foot_br.ori = Quaternion::rotation_x(0.0);
        next.foot_br.scale = Vec3::one() * 1.01;
        next
    }
}
