use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Dependency = f64;
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_jump\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_jump")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab = 1.0;
        let torso = (anim_time as f32 * lab as f32 + 1.5 * PI).sin();

        let wave_slow = (anim_time as f32 * 0.8).sin();

        next.head.scale = Vec3::one() * 1.02;

        next.hand_l.scale = Vec3::one() * 1.04;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1 + torso * 0.2) * 1.02;
        next.head.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);

        next.upper_torso.position =
            Vec3::new(0.0, s_a.upper_torso.0, s_a.upper_torso.1 + torso * 0.5);
        next.upper_torso.orientation = Quaternion::rotation_x(-0.3);
        next.upper_torso.scale = Vec3::one();

        next.lower_torso.position =
            Vec3::new(0.0, s_a.lower_torso.0, s_a.lower_torso.1 + torso * 0.15);
        next.lower_torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.2);
        next.lower_torso.scale = Vec3::one() * 1.02;

        next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
        next.jaw.orientation = Quaternion::rotation_x(wave_slow * 0.09);
        next.jaw.scale = Vec3::one();

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1 + torso * 0.0);
        next.tail.orientation = Quaternion::rotation_z(0.0);
        next.tail.scale = Vec3::one();

        next.control.position = Vec3::new(0.0, 0.0, 0.0);
        next.control.orientation = Quaternion::rotation_z(0.0);
        next.control.scale = Vec3::one();

        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation =
            Quaternion::rotation_x(PI) * Quaternion::rotation_y(0.0) * Quaternion::rotation_z(0.0);
        next.second.scale = Vec3::one() * 0.0;

        next.main.position = Vec3::new(-5.0, -7.0, 7.0);
        next.main.orientation =
            Quaternion::rotation_x(PI) * Quaternion::rotation_y(0.6) * Quaternion::rotation_z(1.57);
        next.main.scale = Vec3::one() * 1.02;

        next.shoulder_l.position = Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.5);
        next.shoulder_l.scale = Vec3::one();

        next.shoulder_r.position = Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.5);
        next.shoulder_r.scale = Vec3::one();

        next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2 + torso * 0.6);
        next.hand_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.8);

        next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2 + torso * 0.6);
        next.hand_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.8);
        next.hand_r.scale = Vec3::one() * 1.02;

        next.leg_l.position = Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2 + torso * 0.2) * 1.02;
        next.leg_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.4);
        next.leg_l.scale = Vec3::one() * 1.02;

        next.leg_r.position = Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2 + torso * 0.2) * 1.02;
        next.leg_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.4);
        next.leg_r.scale = Vec3::one() * 1.02;

        next.foot_l.position = Vec3::new(-s_a.foot.0, -5.0 + s_a.foot.1, s_a.foot.2);
        next.foot_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.4);
        next.foot_l.scale = Vec3::one();

        next.foot_r.position = Vec3::new(s_a.foot.0, 5.0 + s_a.foot.1, s_a.foot.2);
        next.foot_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.4);
        next.foot_r.scale = Vec3::one();

        next.torso.position = Vec3::new(0.0, 0.0, 0.0) / 8.0;
        next.torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.torso.scale = Vec3::one() / 8.0;

        next
    }
}
