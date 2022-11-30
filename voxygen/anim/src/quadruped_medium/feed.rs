use super::{
    super::{vek::*, Animation},
    QuadrupedMediumSkeleton, SkeletonAttr,
};
use std::{f32::consts::PI, ops::Mul};

pub struct FeedAnimation;

impl Animation for FeedAnimation {
    type Dependency<'a> = f32;
    type Skeleton = QuadrupedMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_medium_feed\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_medium_feed")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let slower = (anim_time * 1.0 + PI).sin();
        let slow = (anim_time * 3.5 + PI).sin();
        let fast = (anim_time * 5.0).sin();
        let faster = (anim_time * 14.0).sin();

        let transition = ((anim_time.powf(2.0)).min(1.0)) * s_a.feed.1;

        let look = Vec2::new(
            (global_time / 2.0 + anim_time / 8.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.5,
            (global_time / 2.0 + anim_time / 8.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.25,
        );

        if s_a.feed.0 {
            next.head.position = Vec3::new(
                0.0,
                s_a.head.0,
                s_a.head.1 + slower * 0.2 + transition * 1.5,
            );
            next.head.orientation = Quaternion::rotation_z(0.3 * look.x)
                * Quaternion::rotation_x(fast * 0.05 + faster * 0.08 + transition * -0.5);

            next.neck.position = Vec3::new(
                0.0,
                s_a.neck.0 + transition * 1.0,
                s_a.neck.1 + slower * 0.1 + transition * 1.5,
            );
            next.neck.orientation = Quaternion::rotation_x(transition * -0.5);

            next.jaw.position = Vec3::new(0.0, s_a.jaw.0 - slower * 0.12, s_a.jaw.1 + slow * 0.2);
            next.jaw.orientation = Quaternion::rotation_x((fast * 0.18 + faster * 0.26).min(0.0));
        } else {
            next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1 + slower * 0.2);
            next.head.orientation =
                Quaternion::rotation_z(0.3 * look.x) * Quaternion::rotation_x(0.3 * look.y);

            next.neck.position = Vec3::new(0.0, s_a.neck.0, s_a.neck.1 + slower * 0.1);

            next.jaw.position =
                Vec3::new(0.0, s_a.jaw.0 - slower * 0.12, s_a.jaw.1 + slow * 0.2 + 0.5);
            next.jaw.orientation = Quaternion::rotation_x(slow * 0.05 * anim_time.min(1.0) - 0.08);
        }

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.tail.orientation = Quaternion::rotation_z(0.0 + slow * 0.2 + look.x);

        next.torso_front.position = Vec3::new(
            0.0,
            s_a.torso_front.0,
            s_a.torso_front.1 + slower * 0.3 + transition * -6.0,
        );
        next.torso_front.orientation =
            Quaternion::rotation_x(transition * -0.7) * Quaternion::rotation_y(slow * 0.02);

        next.torso_back.position =
            Vec3::new(0.0, s_a.torso_back.0, s_a.torso_back.1 + slower * 0.2);
        next.torso_back.orientation =
            Quaternion::rotation_x(transition * 0.5) * Quaternion::rotation_y(-slow * 0.005);

        next.ears.position = Vec3::new(0.0, s_a.ears.0, s_a.ears.1);
        next.ears.orientation = Quaternion::rotation_x(0.0 + slower * 0.03);

        next.leg_fl.position = Vec3::new(
            -s_a.leg_f.0,
            s_a.leg_f.1 + transition * -2.2,
            s_a.leg_f.2 + slow * -0.15 + slower * -0.15 + transition * 2.4,
        );
        next.leg_fl.orientation =
            Quaternion::rotation_x(transition * 1.0) * Quaternion::rotation_y(slow * -0.02);

        next.leg_fr.position = Vec3::new(
            s_a.leg_f.0,
            s_a.leg_f.1 + transition * -2.2,
            s_a.leg_f.2 + slow * 0.15 + slower * -0.15 + transition * 2.4,
        );
        next.leg_fr.orientation =
            Quaternion::rotation_x(transition * 1.0) * Quaternion::rotation_y(slow * -0.02);

        next.leg_bl.position = Vec3::new(
            -s_a.leg_b.0,
            s_a.leg_b.1,
            s_a.leg_b.2 + slower * -0.3 + transition * -1.3,
        );
        next.leg_bl.orientation =
            Quaternion::rotation_x(transition * 0.2) * Quaternion::rotation_y(slow * -0.02);

        next.leg_br.position = Vec3::new(
            s_a.leg_b.0,
            s_a.leg_b.1,
            s_a.leg_b.2 + slower * -0.3 + transition * -1.3,
        );
        next.leg_br.orientation =
            Quaternion::rotation_x(transition * 0.2) * Quaternion::rotation_y(slow * -0.02);

        next.foot_fl.position =
            Vec3::new(-s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2 + slower * -0.2);
        next.foot_fl.orientation = Quaternion::rotation_x(transition * -0.3);

        next.foot_fr.position = Vec3::new(s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2 + slower * -0.2);
        next.foot_fr.orientation = Quaternion::rotation_x(transition * -0.3);
        next.foot_bl.position =
            Vec3::new(-s_a.feet_b.0, s_a.feet_b.1, s_a.feet_b.2 + slower * -0.2);

        next.foot_br.position = Vec3::new(s_a.feet_b.0, s_a.feet_b.1, s_a.feet_b.2 + slower * -0.2);

        next
    }
}
