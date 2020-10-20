use super::{
    super::{vek::*, Animation},
    GolemSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct ShockwaveAnimation;

impl Animation for ShockwaveAnimation {
    type Dependency = (f32, f64);
    type Skeleton = GolemSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"golem_shockwave\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "golem_shockwave")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, _global_time): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab = 1.0;
        let breathe = (anim_time as f32 * lab as f32 + 1.5 * PI).sin();
        let twist = anim_time as f32 * lab as f32 * 2.5;

        let slower = (((1.0)
            / (0.00001
                + 0.9999
                    * ((anim_time as f32 * lab as f32 * 2.0 - 0.5 * PI).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 2.0 - 0.5 * PI).sin())
            + 1.0;
        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1) * 1.02;
        next.head.orientation =
            Quaternion::rotation_z((-twist * 2.0).max(-PI)) * Quaternion::rotation_x(0.0);
        next.head.scale = Vec3::one() * 1.02;

        next.upper_torso.position = Vec3::new(
            0.0,
            s_a.upper_torso.0,
            s_a.upper_torso.1 + slower * -3.0 + breathe * 1.0,
        ) / 8.0;
        next.upper_torso.orientation =
            Quaternion::rotation_z((twist * 2.0).min(PI)) * Quaternion::rotation_x(0.0);
        next.upper_torso.scale = Vec3::one() / 8.0;

        next.lower_torso.position =
            Vec3::new(0.0, s_a.lower_torso.0, s_a.lower_torso.1 + slower * 1.0);
        next.lower_torso.orientation =
            Quaternion::rotation_z((-twist * 2.0).max(-PI)) * Quaternion::rotation_x(0.0);
        next.lower_torso.scale = Vec3::one();

        next.shoulder_l.position = Vec3::new(
            -s_a.shoulder.0 - 2.0,
            s_a.shoulder.1,
            s_a.shoulder.2 - slower * 1.0,
        );
        next.shoulder_l.orientation =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_y(0.6 + slower * -0.3);
        next.shoulder_l.scale = Vec3::one();

        next.shoulder_r.position = Vec3::new(
            s_a.shoulder.0 + 2.0,
            s_a.shoulder.1,
            s_a.shoulder.2 - slower * 1.0,
        );
        next.shoulder_r.orientation =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_y(-0.6 + slower * 0.3);
        next.shoulder_r.scale = Vec3::one();

        next.hand_l.position = Vec3::new(
            -s_a.hand.0 - 1.0,
            s_a.hand.1,
            s_a.hand.2 - slower * 0.5 + breathe * -1.0,
        );
        next.hand_l.orientation =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_y(-0.6 + slower * 0.3);
        next.hand_l.scale = Vec3::one() * 1.02;

        next.hand_r.position = Vec3::new(
            s_a.hand.0 + 1.0,
            s_a.hand.1,
            s_a.hand.2 - slower * 0.5 + breathe * -1.0,
        );
        next.hand_r.orientation =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_y(0.6 + slower * -0.3);
        next.hand_r.scale = Vec3::one() * 1.02;
        if velocity < 0.5 {
            next.leg_l.position =
                Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2 + slower * -0.5) * 1.02;
            next.leg_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
            next.leg_l.scale = Vec3::one() * 1.02;

            next.leg_r.position = Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2 + slower * -0.5) * 1.02;
            next.leg_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
            next.leg_r.scale = Vec3::one() * 1.02;

            next.foot_l.position = Vec3::new(
                -s_a.foot.0,
                s_a.foot.1,
                s_a.foot.2 + slower * 2.5 + breathe * -1.0,
            );
            next.foot_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
            next.foot_l.scale = Vec3::one();

            next.foot_r.position = Vec3::new(
                s_a.foot.0,
                s_a.foot.1,
                s_a.foot.2 + slower * 2.5 + breathe * -1.0,
            );
            next.foot_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
            next.foot_r.scale = Vec3::one();

            next.torso.position = Vec3::new(0.0, 0.0, 0.0);
            next.torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
            next.torso.scale = Vec3::one();
        } else {
        }
        next
    }
}
