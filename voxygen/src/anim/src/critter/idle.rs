use super::{super::Animation, CritterAttr, CritterSkeleton};
//use std::{f32::consts::PI, ops::Mul};
use super::super::vek::*;
use std::{f32::consts::PI, ops::Mul};

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency = f64;
    type Skeleton = CritterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"critter_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "critter_idle")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        skeleton_attr: &CritterAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave = (anim_time as f32 * 10.0).sin();
        let wave_slow = (anim_time as f32 * 5.5 + PI).sin();

        let rat_head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 3.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.5,
            ((global_time + anim_time) as f32 / 3.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.25,
        );
        next.head.offset = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1);
        next.head.ori = Quaternion::rotation_z(rat_head_look.x)
            * Quaternion::rotation_x(rat_head_look.y + wave * 0.03);
        next.head.scale = Vec3::one();

        next.chest.offset = Vec3::new(
            0.0,
            skeleton_attr.chest.0,
            skeleton_attr.chest.1 + wave * 0.3,
        ) / 18.0;
        next.chest.ori = Quaternion::rotation_y(wave_slow * 0.06);
        next.chest.scale = Vec3::one() / 18.0;

        next.feet_f.offset = Vec3::new(0.0, skeleton_attr.feet_f.0, skeleton_attr.feet_f.1);
        next.feet_f.ori = Quaternion::rotation_z(0.0);
        next.feet_f.scale = Vec3::one();

        next.feet_b.offset = Vec3::new(0.0, skeleton_attr.feet_b.0, skeleton_attr.feet_b.1);
        next.feet_b.ori = Quaternion::rotation_x(0.0);
        next.feet_b.scale = Vec3::one();

        next.tail.offset = Vec3::new(0.0, skeleton_attr.tail.0 + wave * 0.2, skeleton_attr.tail.1);
        next.tail.ori = Quaternion::rotation_y(wave_slow * 0.05);
        next.tail.scale = Vec3::one();

        next
    }
}
