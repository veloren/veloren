use super::{
    super::{vek::*, Animation},
    BirdMediumSkeleton, SkeletonAttr,
};
use std::ops::Mul;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency<'a> = f32;
    type Skeleton = BirdMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"bird_medium_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "bird_medium_idle")]

    fn update_skeleton_inner<'a>(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency<'a>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave_slow = (anim_time * 4.5).sin();
        let wave_slow_cos = (anim_time * 4.5).cos();

        let duck_head_look = Vec2::new(
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

        next.torso.scale = Vec3::one() / 11.0;
        next.wing_l.scale = Vec3::one() * 1.02;
        next.wing_r.scale = Vec3::one() * 1.02;
        next.leg_l.scale = Vec3::one() / 11.0;
        next.leg_r.scale = Vec3::one() / 11.0;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_z(duck_head_look.x)
            * Quaternion::rotation_x(-duck_head_look.y.abs() + wave_slow_cos * 0.03);

        next.torso.position = Vec3::new(0.0, s_a.chest.0, wave_slow * 0.3 + s_a.chest.1) / 11.0;
        next.torso.orientation = Quaternion::rotation_y(wave_slow * 0.03);

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.tail.orientation = Quaternion::rotation_x(wave_slow_cos * 0.03);

        next.wing_l.position = Vec3::new(-s_a.wing.0, s_a.wing.1, s_a.wing.2);

        next.wing_r.position = Vec3::new(s_a.wing.0, s_a.wing.1, s_a.wing.2);

        next.leg_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2) / 11.0;

        next.leg_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2) / 11.0;
        next
    }
}
