use super::{
    super::{vek::*, Animation},
    QuadrupedSmallSkeleton, SkeletonAttr,
};
use std::{f32::consts::PI, ops::Mul};

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency<'a> = f32;
    type Skeleton = QuadrupedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_small_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_small_idle")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let slow = (anim_time * 3.5).sin();

        let slow_alt = (anim_time * 3.5 + PI / 2.0).sin();

        let head_look = Vec2::new(
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

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1 + slow * 0.2);
        next.head.orientation = Quaternion::rotation_z(head_look.x)
            * Quaternion::rotation_x(head_look.y + slow_alt * 0.03);

        next.chest.position = Vec3::new(slow * 0.05, s_a.chest.0, s_a.chest.1);
        next.chest.orientation = Quaternion::rotation_y(slow * 0.05);

        next.leg_fl.position = Vec3::new(-s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2 + slow * -0.2);
        next.leg_fl.orientation =
            Quaternion::rotation_x(slow * 0.03) * Quaternion::rotation_y(slow * -0.05);

        next.leg_fr.position = Vec3::new(s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2 + slow * 0.2);
        next.leg_fr.orientation =
            Quaternion::rotation_x(slow_alt * 0.03) * Quaternion::rotation_y(slow * -0.05);

        next.leg_bl.position = Vec3::new(-s_a.feet_b.0, s_a.feet_b.1, s_a.feet_b.2 + slow * -0.2);
        next.leg_bl.orientation =
            Quaternion::rotation_x(slow_alt * 0.03) * Quaternion::rotation_y(slow * -0.05);

        next.leg_br.position = Vec3::new(s_a.feet_b.0, s_a.feet_b.1, s_a.feet_b.2 + slow * 0.2);
        next.leg_br.orientation =
            Quaternion::rotation_x(slow * 0.03) * Quaternion::rotation_y(slow * -0.05);

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.tail.orientation = Quaternion::rotation_z(slow * 0.4);

        next
    }
}
