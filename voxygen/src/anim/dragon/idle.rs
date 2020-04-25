use super::{super::Animation, DragonSkeleton, SkeletonAttr};
//use std::{f32::consts::PI, ops::Mul};
use vek::*;
pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency = f64;
    type Skeleton = DragonSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        _anim_time: f64,
        _rate: &mut f32,
        _skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        #[const_tweaker::tweak(min = -100.0, max = 20.0, step = 0.5)]
        const HEAD_X: f32 = 0.0;
        #[const_tweaker::tweak(min = -100.0, max = 20.0, step = 0.5)]
        const HEAD_Z: f32 = 0.0;
        #[const_tweaker::tweak(min = -100.0, max = 20.0, step = 0.5)]
        const CHEST_F_X: f32 = 0.0;
        #[const_tweaker::tweak(min = -100.0, max = 20.0, step = 0.5)]
        const CHEST_F_Z: f32 = 0.0;
        #[const_tweaker::tweak(min = -100.0, max = 20.0, step = 0.5)]
        const CHEST_R_X: f32 = 0.0;
        #[const_tweaker::tweak(min = -100.0, max = 20.0, step = 0.5)]
        const CHEST_R_Z: f32 = 0.0;
        #[const_tweaker::tweak(min = -100.0, max = 20.0, step = 0.5)]
        const TAIL_F_X: f32 = 0.0;
        #[const_tweaker::tweak(min = -100.0, max = 20.0, step = 0.5)]
        const TAIL_F_Z: f32 = 0.0;
        #[const_tweaker::tweak(min = -100.0, max = 20.0, step = 0.5)]
        const TAIL_R_X: f32 = 0.0;
        #[const_tweaker::tweak(min = -100.0, max = 20.0, step = 0.5)]
        const TAIL_R_Z: f32 = 0.0;
        #[const_tweaker::tweak(min = -100.0, max = 20.0, step = 0.5)]
        const WING_IN_X: f32 = 0.0;
        #[const_tweaker::tweak(min = -100.0, max = 20.0, step = 0.5)]
        const WING_IN_Z: f32 = 0.0;
        #[const_tweaker::tweak(min = -100.0, max = 20.0, step = 0.5)]
        const WING_OUT_X: f32 = 0.0;
        #[const_tweaker::tweak(min = -100.0, max = 20.0, step = 0.5)]
        const WING_OUT_Z: f32 = 0.0;
        #[const_tweaker::tweak(min = -100.0, max = 20.0, step = 0.5)]
        const FEET_F_X: f32 = 0.0;
        #[const_tweaker::tweak(min = -100.0, max = 20.0, step = 0.5)]
        const FEET_F_Y: f32 = 0.0;
        #[const_tweaker::tweak(min = -100.0, max = 20.0, step = 0.5)]
        const FEET_F_Z: f32 = 0.0;
        #[const_tweaker::tweak(min = -100.0, max = 20.0, step = 0.5)]
        const FEET_B_X: f32 = 0.0;
        #[const_tweaker::tweak(min = -100.0, max = 20.0, step = 0.5)]
        const FEET_B_Y: f32 = 0.0;
        #[const_tweaker::tweak(min = -100.0, max = 20.0, step = 0.5)]
        const FEET_B_Z: f32 = 0.0;        

        next.head.offset = Vec3::new(0.0, *HEAD_X, *HEAD_Z);
        next.head.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.head.scale = Vec3::one() * 1.01;

        next.chest_front.offset = Vec3::new(0.0, *CHEST_F_X, *CHEST_F_Z);
        next.chest_front.ori = Quaternion::rotation_x(0.0);
        next.chest_front.scale = Vec3::one() * 1.01;

        next.chest_rear.offset = Vec3::new(0.0, *CHEST_R_X, *CHEST_R_Z);
        next.chest_rear.ori = Quaternion::rotation_x(0.0);
        next.chest_rear.scale = Vec3::one() * 1.01;

        next.tail_front.offset = Vec3::new(0.0, *TAIL_F_X, *TAIL_F_Z);
        next.tail_front.ori = Quaternion::rotation_x(0.0);
        next.tail_front.scale = Vec3::one() * 1.01;

        next.tail_rear.offset = Vec3::new(0.0, *TAIL_R_X, *TAIL_R_Z);
        next.tail_rear.ori = Quaternion::rotation_x(0.0);
        next.tail_rear.scale = Vec3::one() * 1.01;

        next.wing_in_l.offset = Vec3::new(0.0, *WING_IN_X, *WING_IN_Z);
        next.wing_in_l.ori = Quaternion::rotation_x(0.0);
        next.wing_in_l.scale = Vec3::one() * 1.01;

        next.wing_in_r.offset = Vec3::new(0.0, *WING_IN_X, *WING_IN_Z);
        next.wing_in_r.ori = Quaternion::rotation_x(0.0);
        next.wing_in_r.scale = Vec3::one() * 1.01;

        next.wing_out_l.offset = Vec3::new(0.0, *WING_OUT_X, *WING_OUT_Z);
        next.wing_out_l.ori = Quaternion::rotation_x(0.0);
        next.wing_out_l.scale = Vec3::one() * 1.01;

        next.wing_out_r.offset = Vec3::new(0.0, *WING_OUT_X, *WING_OUT_Z);
        next.wing_out_r.ori = Quaternion::rotation_x(0.0);
        next.wing_out_r.scale = Vec3::one() * 1.01;

        next.foot_fl.offset = Vec3::new(*FEET_F_X, *FEET_F_Y, *FEET_F_Z);
        next.foot_fl.ori = Quaternion::rotation_x(0.0);
        next.foot_fl.scale = Vec3::one() * 1.01;

        next.foot_fr.offset = Vec3::new(*FEET_F_X, *FEET_F_Y, *FEET_F_Z);
        next.foot_fr.ori = Quaternion::rotation_x(0.0);
        next.foot_fr.scale = Vec3::one() * 1.01;

        next.foot_bl.offset = Vec3::new(*FEET_F_X, *FEET_B_Y, *FEET_B_Z);
        next.foot_bl.ori = Quaternion::rotation_x(0.0);
        next.foot_bl.scale = Vec3::one() * 1.01;

        next.foot_br.offset = Vec3::new(*FEET_F_X, *FEET_B_Y, *FEET_B_Z);
        next.foot_br.ori = Quaternion::rotation_x(0.0);
        next.foot_br.scale = Vec3::one() * 1.01;
        next
    }
}
