use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};

use std::f32::consts::PI;

pub struct PetAnimation;

impl Animation for PetAnimation {
    type Dependency<'a> = (Vec3<f32>, Option<vek::Vec3<f32>>, f32);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_pet\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_pet")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (pos, target_pos, _global_time): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let fast = (anim_time * 3.0).sin();
        let fast_offset = (anim_time * 3.0 + PI * 0.5).sin();

        let z_diff = target_pos.map_or(0., |target_pos| target_pos.z - pos.z);

        // Tilt head down by 10 deg
        next.head.orientation = Quaternion::rotation_x(-1. * PI / 2. / 9.);

        // Lift hand up and out, slight hand position change depending on height
        next.hand_r.position = Vec3::new(
            s_a.hand.0 + -2. * fast_offset,
            s_a.hand.1 + 8.0,
            s_a.hand.2 + 4.0 + 1. * fast + z_diff,
        );

        // Raise arm 90deg then up and down
        next.hand_r.orientation =
            Quaternion::rotation_x(PI / 2. + fast * 0.15).rotated_z(fast_offset * 0.5);

        next
    }
}
