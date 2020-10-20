use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use std::{f32::consts::PI, ops::Mul};

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency = f64;
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_idle")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab = 1.0;
        let torso = (anim_time as f32 * lab as f32 + 1.5 * PI).sin();

        let slower = (anim_time as f32 * 1.0 + PI).sin();
        let slow = (anim_time as f32 * 3.5 + PI).sin();

        let look = Vec2::new(
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
        let tailmove = Vec2::new(
            ((global_time + anim_time) as f32 / 2.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.25,
            ((global_time + anim_time) as f32 / 2.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.125,
        );

        let breathe = if s_a.beast {
            // Controls for the beast breathing
            let intensity = 0.04;
            let lenght = 1.5;
            let chop = 0.2;
            let chop_freq = 60.0;
            intensity * (lenght * anim_time as f32).sin()
                + 0.05
                    * chop
                    * (anim_time as f32 * chop_freq).sin()
                    * (anim_time as f32 * lenght).cos()
        } else {
            0.0
        };

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1 + torso * 0.2) * 1.02;
        next.head.orientation =
            Quaternion::rotation_z(look.x * 0.6) * Quaternion::rotation_x(look.y * 0.6 + breathe);
        next.head.scale = Vec3::one() * 1.02 + breathe * 0.4;

        next.upper_torso.position =
            Vec3::new(0.0, s_a.upper_torso.0, s_a.upper_torso.1 + torso * 0.5);
        next.upper_torso.orientation =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-breathe);
        next.upper_torso.scale = Vec3::one() - breathe * 0.4;

        next.lower_torso.position =
            Vec3::new(0.0, s_a.lower_torso.0, s_a.lower_torso.1 + torso * 0.15);
        next.lower_torso.orientation =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(breathe);
        next.lower_torso.scale = Vec3::one() * 1.02 + breathe * 0.4;

        if s_a.beast {
            next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
        } else {
            next.jaw.position = Vec3::new(0.0, s_a.jaw.0 - slower * 0.12, s_a.jaw.1 + slow * 0.2);
        }
        next.jaw.orientation = Quaternion::rotation_x(-0.1 + breathe * 2.0);
        next.jaw.scale = Vec3::one() * 0.98;

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.tail.orientation =
            Quaternion::rotation_z(0.0 + slow * 0.2 + tailmove.x) * Quaternion::rotation_x(0.0);
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
        next.shoulder_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(breathe);
        next.shoulder_l.scale = Vec3::one() + breathe;

        next.shoulder_r.position = Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(breathe);
        next.shoulder_r.scale = Vec3::one() + breathe;

        next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2 + torso * 0.6);
        next.hand_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.hand_l.scale = Vec3::one() * 1.02;

        next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2 + torso * 0.6);
        next.hand_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.hand_r.scale = Vec3::one() * 1.02;

        next.arm_control_l.scale = Vec3::one() * 1.0;
        next.arm_control_r.scale = Vec3::one() * 1.0;

        next.leg_control_l.scale = Vec3::one() * 1.0;

        next.leg_l.position = Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2 + torso * 0.2);
        next.leg_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.leg_l.scale = Vec3::one() * 1.02;

        next.leg_r.position = Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2 + torso * 0.2);
        next.leg_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.leg_r.scale = Vec3::one() * 1.02;

        next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2 + torso * -0.6);
        next.foot_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.foot_l.scale = Vec3::one();

        next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2 + torso * -0.6);
        next.foot_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.foot_r.scale = Vec3::one();

        next.torso.position = Vec3::new(0.0, 0.0, 0.0) / 8.0;
        next.torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);
        next.torso.scale = Vec3::one() / 8.0;

        next.leg_control_l.scale = Vec3::one() * 1.0;
        next.leg_control_r.scale = Vec3::one() * 1.0;
        next.arm_control_l.scale = Vec3::one() * 1.0;
        next.arm_control_r.scale = Vec3::one() * 1.0;

        next.hold.scale = Vec3::one() * 0.0;

        next
    }
}
