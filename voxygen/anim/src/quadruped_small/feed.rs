use super::{
    super::{vek::*, Animation},
    QuadrupedSmallSkeleton, SkeletonAttr,
};
use std::{f32::consts::PI, ops::Mul};

pub struct FeedAnimation;

impl Animation for FeedAnimation {
    type Dependency<'a> = f32;
    type Skeleton = QuadrupedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_small_feed\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_small_feed")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let slow = (anim_time * 5.0).sin();
        let quick = (anim_time * 14.0).sin();

        let slow_alt = (anim_time * 3.5 + PI).sin();

        let head_look = Vec2::new(
            (global_time / 2.0 + anim_time / 2.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 1.0,
            (global_time / 2.0 + anim_time / 2.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.5,
        );

        next.head.position = Vec3::new(0.0, s_a.head.0 + 1.5, s_a.head.1 + slow * 0.2);
        next.head.orientation = Quaternion::rotation_z(head_look.y)
            * Quaternion::rotation_x(slow * 0.05 + quick * 0.08 - 0.4 * s_a.feed);

        next.chest.position = Vec3::new(slow * 0.02, s_a.chest.0, s_a.chest.1);
        next.chest.orientation =
            Quaternion::rotation_x(-0.35 * s_a.feed) * Quaternion::rotation_y(head_look.y * 0.1);

        next.leg_fl.position = Vec3::new(-s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2 + 0.5);
        next.leg_fl.orientation = Quaternion::rotation_x(slow * 0.01 + 0.25 * s_a.feed)
            * Quaternion::rotation_y(slow * -0.02 - head_look.y * 0.1);

        next.leg_fr.position = Vec3::new(s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2 + 0.5);
        next.leg_fr.orientation = Quaternion::rotation_x(slow_alt * 0.01 + 0.25 * s_a.feed)
            * Quaternion::rotation_y(slow * -0.02 - head_look.y * 0.1);

        next.leg_bl.position = Vec3::new(-s_a.feet_b.0, s_a.feet_b.1 + 1.0, s_a.feet_b.2 - 1.0);
        next.leg_bl.orientation = Quaternion::rotation_x(slow_alt * 0.01 + 0.15 * s_a.feed)
            * Quaternion::rotation_y(slow * -0.02 - head_look.y * 0.1);

        next.leg_br.position = Vec3::new(s_a.feet_b.0, s_a.feet_b.1 + 1.0, s_a.feet_b.2 - 1.0);
        next.leg_br.orientation = Quaternion::rotation_x(slow * 0.01 + 0.15 * s_a.feed)
            * Quaternion::rotation_y(slow * -0.02 - head_look.y * 0.1);

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.tail.orientation = Quaternion::rotation_z(slow * 0.3 + head_look.y * 0.3);

        next
    }
}
