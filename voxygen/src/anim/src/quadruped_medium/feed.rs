use super::{
    super::{vek::*, Animation},
    QuadrupedMediumSkeleton, SkeletonAttr,
};
use std::{f32::consts::PI, ops::Mul};

pub struct FeedAnimation;

impl Animation for FeedAnimation {
    type Dependency = f64;
    type Skeleton = QuadrupedMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_medium_feed\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_medium_feed")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let slower = (anim_time as f32 * 1.0 + PI).sin();
        let slow = (anim_time as f32 * 3.5 + PI).sin();
        let fast = (anim_time as f32 * 5.0).sin();
        let faster = (anim_time as f32 * 14.0).sin();

        let transition = (anim_time as f32).min(0.4) / 0.4;

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

        next.neck.scale = Vec3::one() * 1.02;
        next.torso_front.scale = Vec3::one() * s_a.scaler / 11.0;
        next.torso_back.scale = Vec3::one() * 0.99;
        next.neck.scale = Vec3::one() * 1.02;
        next.jaw.scale = Vec3::one() * 1.02;

        if s_a.feed.0 {
            next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1 + slower * 0.2);
            next.head.orientation = Quaternion::rotation_z(0.3 * look.x)
                * Quaternion::rotation_x(
                    fast * 0.05 + faster * 0.08 + 0.8 * s_a.feed.1 * transition,
                );

            next.neck.position = Vec3::new(
                0.0,
                s_a.neck.0,
                s_a.neck.1 + slower * 0.1 - 4.0 * transition,
            );
            next.neck.orientation = Quaternion::rotation_x(-2.5 * s_a.feed.1 * transition);

            next.jaw.position = Vec3::new(0.0, s_a.jaw.0 - slower * 0.12, s_a.jaw.1 + slow * 0.2);
            next.jaw.orientation = Quaternion::rotation_x((fast * 0.18 + faster * 0.26).min(0.0));
        } else {
            next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1 + slower * 0.2);
            next.head.orientation =
                Quaternion::rotation_z(0.3 * look.x) * Quaternion::rotation_x(0.3 * look.y);

            next.neck.position = Vec3::new(0.0, s_a.neck.0, s_a.neck.1 + slower * 0.1);

            next.jaw.position =
                Vec3::new(0.0, s_a.jaw.0 - slower * 0.12, s_a.jaw.1 + slow * 0.2 + 0.5);
            next.jaw.orientation = Quaternion::rotation_x(slow * 0.05 - 0.08);
        }

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.tail.orientation = Quaternion::rotation_z(0.0 + slow * 0.2 + look.x);

        next.torso_front.position =
            Vec3::new(0.0, s_a.torso_front.0, s_a.torso_front.1 + slower * 0.3) * s_a.scaler / 11.0;
        next.torso_front.orientation = Quaternion::rotation_y(slow * 0.02);

        next.torso_back.position =
            Vec3::new(0.0, s_a.torso_back.0, s_a.torso_back.1 + slower * 0.2);
        next.torso_back.orientation = Quaternion::rotation_y(-slow * 0.005);

        next.ears.position = Vec3::new(0.0, s_a.ears.0, s_a.ears.1);
        next.ears.orientation = Quaternion::rotation_x(0.0 + slower * 0.03);

        next.leg_fl.position = Vec3::new(
            -s_a.leg_f.0,
            s_a.leg_f.1,
            s_a.leg_f.2 + slow * -0.15 + slower * -0.15,
        );
        next.leg_fl.orientation = Quaternion::rotation_y(slow * -0.02);

        next.leg_fr.position = Vec3::new(
            s_a.leg_f.0,
            s_a.leg_f.1,
            s_a.leg_f.2 + slow * 0.15 + slower * -0.15,
        );
        next.leg_fr.orientation = Quaternion::rotation_y(slow * -0.02);

        next.leg_bl.position = Vec3::new(-s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2 + slower * -0.3);
        next.leg_bl.orientation = Quaternion::rotation_y(slow * -0.02);

        next.leg_br.position = Vec3::new(s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2 + slower * -0.3);
        next.leg_br.orientation = Quaternion::rotation_y(slow * -0.02);

        next.foot_fl.position =
            Vec3::new(-s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2 + slower * -0.2);

        next.foot_fr.position = Vec3::new(s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2 + slower * -0.2);

        next.foot_bl.position =
            Vec3::new(-s_a.feet_b.0, s_a.feet_b.1, s_a.feet_b.2 + slower * -0.2);

        next.foot_br.position = Vec3::new(s_a.feet_b.0, s_a.feet_b.1, s_a.feet_b.2 + slower * -0.2);

        next
    }
}
