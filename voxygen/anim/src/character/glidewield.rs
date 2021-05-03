use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};

pub struct GlideWieldAnimation;

impl Animation for GlideWieldAnimation {
    type Dependency<'a> = ();
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_glidewield\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_glidewield")]

    fn update_skeleton_inner<'a>(
        skeleton: &Self::Skeleton,
        _: Self::Dependency<'a>,
        _anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        *rate = 1.0;

        next.hand_l.position = Vec3::new(-2.0 - s_a.hand.0, s_a.hand.1, s_a.hand.2 + 15.0);
        next.hand_l.orientation = Quaternion::rotation_x(3.35) * Quaternion::rotation_y(0.2);

        next.hand_r.position = Vec3::new(2.0 + s_a.hand.0, s_a.hand.1, s_a.hand.2 + 15.0);
        next.hand_r.orientation = Quaternion::rotation_x(3.35) * Quaternion::rotation_y(-0.2);
        next.glider.scale = Vec3::one() * 1.0;
        next.glider.orientation = Quaternion::rotation_x(0.35);

        next.glider.position = Vec3::new(0.0, -5.0, 13.0);

        next
    }
}
