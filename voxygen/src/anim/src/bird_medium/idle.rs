use super::{
    super::{vek::*, Animation},
    BirdMediumSkeleton, SkeletonAttr,
};
use std::ops::Mul;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency = f64;
    type Skeleton = BirdMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"bird_medium_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "bird_medium_idle")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave_slow = (anim_time as f32 * 4.5).sin();
        let wave_slow_cos = (anim_time as f32 * 4.5).cos();

        let duck_head_look = Vec2::new(
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

        next.head.offset = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1);
        next.head.ori = Quaternion::rotation_z(duck_head_look.x)
            * Quaternion::rotation_x(-duck_head_look.y.abs() + wave_slow_cos * 0.03);
        next.head.scale = Vec3::one();

        next.torso.offset = Vec3::new(
            0.0,
            skeleton_attr.chest.0,
            wave_slow * 0.3 + skeleton_attr.chest.1,
        ) / 11.0;
        next.torso.ori = Quaternion::rotation_y(wave_slow * 0.03);
        next.torso.scale = Vec3::one() / 11.0;

        next.tail.offset = Vec3::new(0.0, skeleton_attr.tail.0, skeleton_attr.tail.1);
        next.tail.ori = Quaternion::rotation_x(wave_slow_cos * 0.03);
        next.tail.scale = Vec3::one();

        next.wing_l.offset = Vec3::new(
            -skeleton_attr.wing.0,
            skeleton_attr.wing.1,
            skeleton_attr.wing.2,
        );
        next.wing_l.ori = Quaternion::rotation_z(0.0);
        next.wing_l.scale = Vec3::one() * 1.05;

        next.wing_r.offset = Vec3::new(
            skeleton_attr.wing.0,
            skeleton_attr.wing.1,
            skeleton_attr.wing.2,
        );
        next.wing_r.ori = Quaternion::rotation_y(0.0);
        next.wing_r.scale = Vec3::one() * 1.05;

        next.leg_l.offset = Vec3::new(
            -skeleton_attr.foot.0,
            skeleton_attr.foot.1,
            skeleton_attr.foot.2,
        ) / 11.0;
        next.leg_l.ori = Quaternion::rotation_y(0.0);
        next.leg_l.scale = Vec3::one() / 11.0;

        next.leg_r.offset = Vec3::new(
            skeleton_attr.foot.0,
            skeleton_attr.foot.1,
            skeleton_attr.foot.2,
        ) / 11.0;
        next.leg_r.ori = Quaternion::rotation_x(0.0);
        next.leg_r.scale = Vec3::one() / 11.0;
        next
    }
}
