use super::{
    super::{Animation, vek::*},
    ItemSkeleton, SkeletonAttr,
};

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency<'a> = f32;
    type Skeleton = ItemSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"item_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "item_idle"))]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        _: Self::Dependency<'_>,
        _anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        next.bone0.position = Vec3::new(s_a.bone0.0, s_a.bone0.1, s_a.bone0.2);
        next.bone0.orientation.rotate_x(s_a.bone0.3);

        next
    }
}
