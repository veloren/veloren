use super::{
    super::{vek::*, Animation},
    QuadrupedSmallSkeleton, SkeletonAttr,
};
use std::{f32::consts::PI, ops::Mul};

pub struct FeedAnimation;

impl Animation for FeedAnimation {
    type Dependency = f64;
    type Skeleton = QuadrupedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_small_feed\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_small_feed")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let slow = (anim_time as f32 * 5.0).sin();
        let quick = (anim_time as f32 * 14.0).sin();

        let slow_alt = (anim_time as f32 * 3.5 + PI).sin();

        let head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 2.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 1.0,
            ((global_time + anim_time) as f32 / 2.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.5,
        );

        next.head.offset = Vec3::new(
            0.0,
            skeleton_attr.head.0 + 1.5,
            skeleton_attr.head.1 + slow * 0.2,
        );
        next.head.ori = Quaternion::rotation_z(head_look.y)
            * Quaternion::rotation_x(slow * 0.05 + quick * 0.08 - 0.4 * skeleton_attr.feed);
        next.head.scale = Vec3::one();

        next.chest.offset = Vec3::new(slow * 0.02, skeleton_attr.chest.0, skeleton_attr.chest.1)
            / 11.0
            * skeleton_attr.scaler;
        next.chest.ori = Quaternion::rotation_x(-0.35 * skeleton_attr.feed)
            * Quaternion::rotation_y(head_look.y * 0.1);
        next.chest.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next.leg_fl.offset = Vec3::new(
            -skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2 + 0.5,
        );
        next.leg_fl.ori = Quaternion::rotation_x(slow * 0.01 + 0.25 * skeleton_attr.feed)
            * Quaternion::rotation_y(slow * -0.02 - head_look.y * 0.1);
        next.leg_fl.scale = Vec3::one();

        next.leg_fr.offset = Vec3::new(
            skeleton_attr.feet_f.0,
            skeleton_attr.feet_f.1,
            skeleton_attr.feet_f.2 + 0.5,
        );
        next.leg_fr.ori = Quaternion::rotation_x(slow_alt * 0.01 + 0.25 * skeleton_attr.feed)
            * Quaternion::rotation_y(slow * -0.02 - head_look.y * 0.1);
        next.leg_fr.scale = Vec3::one();

        next.leg_bl.offset = Vec3::new(
            -skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1 + 1.0,
            skeleton_attr.feet_b.2 - 1.0,
        );
        next.leg_bl.ori = Quaternion::rotation_x(slow_alt * 0.01 + 0.15 * skeleton_attr.feed)
            * Quaternion::rotation_y(slow * -0.02 - head_look.y * 0.1);
        next.leg_bl.scale = Vec3::one();

        next.leg_br.offset = Vec3::new(
            skeleton_attr.feet_b.0,
            skeleton_attr.feet_b.1 + 1.0,
            skeleton_attr.feet_b.2 - 1.0,
        );
        next.leg_br.ori = Quaternion::rotation_x(slow * 0.01 + 0.15 * skeleton_attr.feed)
            * Quaternion::rotation_y(slow * -0.02 - head_look.y * 0.1);
        next.leg_br.scale = Vec3::one();

        next.tail.offset = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
        next.tail.ori = Quaternion::rotation_z(slow * 0.3 + head_look.y * 0.3);
        next.tail.scale = Vec3::one();

        next
    }
}
