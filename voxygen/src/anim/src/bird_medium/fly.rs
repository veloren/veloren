use super::{
    super::{vek::*, Animation},
    BirdMediumSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct FlyAnimation;

impl Animation for FlyAnimation {
    type Dependency = (f32, f64);
    type Skeleton = BirdMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"bird_medium_fly\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "bird_medium_fly")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab = 12.0; //14.0

        let footl = (anim_time as f32 * lab as f32 + PI).sin();
        let footr = (anim_time as f32 * lab as f32).sin();
        let center = (anim_time as f32 * lab as f32 + PI / 2.0).sin();
        let centeroffset = (anim_time as f32 * lab as f32 + PI * 1.5).sin();

        next.head.position = Vec3::new(0.0, s_a.head.0 + 0.5, s_a.head.1 + center * 0.5 - 1.0);
        next.head.orientation =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0 + center * 0.03);
        next.head.scale = Vec3::one();

        next.torso.position = Vec3::new(
            0.0,
            s_a.chest.0 + centeroffset * 0.6,
            center * 0.6 + s_a.chest.1,
        ) / 11.0;
        next.torso.orientation = Quaternion::rotation_y(center * 0.05);
        next.torso.scale = Vec3::one() / 11.0;

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1 + centeroffset * 0.6);
        next.tail.orientation = Quaternion::rotation_x(center * 0.03);
        next.tail.scale = Vec3::one();

        next.wing_l.position = Vec3::new(-s_a.wing.0, s_a.wing.1, s_a.wing.2);
        next.wing_l.orientation = Quaternion::rotation_y((0.57 + footl * 1.2).max(0.0));
        next.wing_l.scale = Vec3::one() * 1.05;

        next.wing_r.position = Vec3::new(s_a.wing.0, s_a.wing.1, s_a.wing.2);
        next.wing_r.orientation = Quaternion::rotation_y((-0.57 + footr * 1.2).min(0.0));
        next.wing_r.scale = Vec3::one() * 1.05;

        next.leg_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2) / 11.0;
        next.leg_l.orientation = Quaternion::rotation_x(-1.3 + footl * 0.06);
        next.leg_l.scale = Vec3::one() / 11.0;

        next.leg_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2) / 11.0;
        next.leg_r.orientation = Quaternion::rotation_x(-1.3 + footr * 0.06);
        next.leg_r.scale = Vec3::one() / 11.0;
        next
    }
}
