use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};

pub struct GlideWieldAnimation;

type GlideWieldAnimationDependency = (Quaternion<f32>, Quaternion<f32>);
impl Animation for GlideWieldAnimation {
    type Dependency<'a> = GlideWieldAnimationDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_glidewield\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_glidewield")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (orientation, glider_orientation): Self::Dependency<'_>,
        _anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let glider_ori = orientation.inverse() * glider_orientation;
        let glider_pos = Vec3::new(0.0, -5.0, 13.0);
        *rate = 1.0;

        next.hand_l.position =
            glider_pos + glider_ori * Vec3::new(-s_a.hand.0 + -2.0, s_a.hand.1 + 8.0, s_a.hand.2);
        next.hand_l.orientation = Quaternion::rotation_x(3.35) * Quaternion::rotation_y(0.2);

        next.hand_r.position =
            glider_pos + glider_ori * Vec3::new(s_a.hand.0 + 2.0, s_a.hand.1 + 8.0, s_a.hand.2);
        next.hand_r.orientation = Quaternion::rotation_x(3.35) * Quaternion::rotation_y(-0.2);
        next.glider.scale = Vec3::one() * 1.0;
        next.glider.orientation = glider_ori;

        next.glider.position = Vec3::new(0.0, -5.0, 13.0);

        next
    }
}
