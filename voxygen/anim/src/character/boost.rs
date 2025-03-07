use super::{
    super::{Animation, vek::*},
    CharacterSkeleton, SkeletonAttr,
};

pub struct BoostAnimation;

type BoostAnimationDependency<'a> = ();

impl Animation for BoostAnimation {
    type Dependency<'a> = BoostAnimationDependency<'a>;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_boost\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "character_boost"))]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        _dep: Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        next.main.position = Vec3::new(10.0, -10.0, -10.0);
        next.main.orientation = Quaternion::rotation_z(0.0);
        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation = Quaternion::rotation_z(0.0);

        let (move1, move2, move3, tension) = (
            anim_time.powf(0.25).min(1.0),
            0.0,
            0.0,
            (anim_time * 20.0).sin() - 0.5,
        );
        let pullback = 1.0 - move3;
        let move1 = move1 * pullback;
        let move2 = move2 * pullback;

        next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
        next.hand_l.orientation =
            Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
        next.hand_r.position = Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
        next.hand_r.orientation = Quaternion::rotation_x(std::f32::consts::PI * 0.5);
        next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2 + move2 * 5.0);
        next.control.orientation = Quaternion::rotation_x(s_a.sc.3 + move1 * -0.9)
            * Quaternion::rotation_y(move1 * 1.0 + move2 * -1.0)
            * Quaternion::rotation_z(move1 * 1.3 + move2 * -1.3);

        next.chest.orientation =
            Quaternion::rotation_z(move1 * 1.0 + tension * 0.02 + move2 * -1.2);
        next.head.orientation = Quaternion::rotation_z(move1 * -0.4 + move2 * 0.3);
        next.belt.orientation = Quaternion::rotation_z(move1 * -0.25 + move2 * 0.2);
        next.shorts.orientation = Quaternion::rotation_z(move1 * -0.5 + move2 * 0.4);

        next
    }
}
