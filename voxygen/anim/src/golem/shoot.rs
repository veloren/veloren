use super::{
    super::{vek::*, Animation},
    GolemSkeleton, SkeletonAttr,
};
use common::{states::utils::StageSection, util::Dir};
use core::f32::consts::PI;

pub struct ShootAnimation;

impl Animation for ShootAnimation {
    type Dependency<'a> = (Option<StageSection>, f32, f32, Dir);
    type Skeleton = GolemSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"golem_shoot\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "golem_shoot")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (stage_section, _global_time, _timer, look_dir): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        _s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (move1base, move2base, move3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.4), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powf(4.0)),
            _ => (0.0, 0.0, 0.0),
        };

        let pullback = 1.0 - move3;

        let move1 = move1base * pullback;
        let move2 = move2base * pullback;

        next.head.orientation = Quaternion::rotation_x(-0.2) * Quaternion::rotation_z(move1 * -0.5);

        next.upper_torso.orientation =
            Quaternion::rotation_x(0.0) * Quaternion::rotation_z(move1 * 0.5);

        next.lower_torso.orientation =
            Quaternion::rotation_z(move1 * -0.5) * Quaternion::rotation_x(0.0);

        next.shoulder_l.orientation =
            Quaternion::rotation_y(0.0) * Quaternion::rotation_z(move1 * 0.7);

        next.shoulder_r.orientation = Quaternion::rotation_x(move1 * (look_dir.z * 1.2 + PI / 2.0))
            * Quaternion::rotation_y(move1 * 0.0);

        next.hand_l.orientation =
            Quaternion::rotation_z(move1 * -0.3) * Quaternion::rotation_x(move1 * 1.3);

        next.hand_r.orientation = Quaternion::rotation_y(move1 * -0.3)
            * Quaternion::rotation_z(move1 * -0.9 + move2 * -1.6);

        next.torso.position = Vec3::new(0.0, 0.0, 0.0);
        next
    }
}
