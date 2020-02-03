use super::{super::Animation, BipedLargeSkeleton, SkeletonAttr};
use std::f32::consts::PI;
use vek::*;

pub struct RunAnimation;

impl Animation for RunAnimation {
    type Dependency = (f32, f64);
    type Skeleton = BipedLargeSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (_velocity, _global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab = 14.0;

        let legl = (anim_time as f32 * lab as f32).sin();
        let legr = (anim_time as f32 * lab as f32 + PI).sin();
        let belt = (anim_time as f32 * lab as f32 + 1.5 * PI).sin();

        let foothoril = (anim_time as f32 * lab as f32).sin();
        let foothorir = (anim_time as f32 * lab as f32 + PI).sin();

        let footvertl = (anim_time as f32 * lab as f32 + PI * 1.4).sin().max(0.0);
        let footvertr = (anim_time as f32 * lab as f32 + PI * 0.4).sin().max(0.0);

        next.head.offset =
            Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1 + belt * 1.0) / 8.0;
        next.head.ori = Quaternion::rotation_z(belt * 0.1) * Quaternion::rotation_x(0.3);
        next.head.scale = Vec3::one() / 8.0;

        next.upper_torso.offset = Vec3::new(
            0.0,
            skeleton_attr.upper_torso.0,
            skeleton_attr.upper_torso.1 + belt * 1.0,
        ) / 8.0;
        next.upper_torso.ori = Quaternion::rotation_z(belt * 0.3) * Quaternion::rotation_x(0.0);
        next.upper_torso.scale = Vec3::one() / 8.0;

        next.lower_torso.offset = Vec3::new(
            0.0,
            skeleton_attr.lower_torso.0,
            skeleton_attr.lower_torso.1,
        );
        next.lower_torso.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.lower_torso.scale = Vec3::one() * 1.02;

        next.shoulder_l.offset = Vec3::new(
            -skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.shoulder_l.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(legr * 0.06);
        next.shoulder_l.scale = Vec3::one();

        next.shoulder_r.offset = Vec3::new(
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.shoulder_r.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(legl * 0.1);
        next.shoulder_r.scale = Vec3::one();

        next.hand_l.offset = Vec3::new(
            -skeleton_attr.hand.0,
            skeleton_attr.hand.1,
            skeleton_attr.hand.2,
        );
        next.hand_l.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.3 + legr * 0.5);
        next.hand_l.scale = Vec3::one();

        next.hand_r.offset = Vec3::new(
            skeleton_attr.hand.0,
            skeleton_attr.hand.1,
            skeleton_attr.hand.2,
        );
        next.hand_r.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.3 - legr * 0.5);
        next.hand_r.scale = Vec3::one();

        next.leg_l.offset = Vec3::new(
            -skeleton_attr.leg.0,
            skeleton_attr.leg.1,
            skeleton_attr.leg.2 + belt * 0.4,
        ) / 8.0;
        next.leg_l.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-legl * 0.3);
        next.leg_l.scale = Vec3::one() / 8.0;

        next.leg_r.offset = Vec3::new(
            skeleton_attr.leg.0,
            skeleton_attr.leg.1,
            skeleton_attr.leg.2 + belt * 0.4,
        ) / 8.0;
        next.leg_r.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-legr * 0.3);
        next.leg_r.scale = Vec3::one() / 8.0;

        next.foot_l.offset = Vec3::new(
            -skeleton_attr.foot.0,
            skeleton_attr.foot.1 - foothoril * 4.5 - 0.5,
            skeleton_attr.foot.2 + footvertl * 6.0,
        ) / 8.0;
        next.foot_l.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0 - legl * 0.5);
        next.foot_l.scale = Vec3::one() / 8.0 * 0.98;

        next.foot_r.offset = Vec3::new(
            skeleton_attr.foot.0,
            skeleton_attr.foot.1 - foothorir * 4.5 - 0.5,
            skeleton_attr.foot.2 + footvertr * 6.0,
        ) / 8.0;
        next.foot_r.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0 - legr * 0.5);
        next.foot_r.scale = Vec3::one() / 8.0 * 0.98;

        next.torso.offset = Vec3::new(0.0, 0.0, 0.0);
        next.torso.ori = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.3);
        next.torso.scale = Vec3::one();
        next
    }
}
