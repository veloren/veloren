use super::{
    super::{vek::*, Animation},
    GolemSkeleton, SkeletonAttr,
};
use common::{states::utils::StageSection, util::Dir};
pub struct BeamAnimation;

impl Animation for BeamAnimation {
    type Dependency<'a> = (Option<StageSection>, f32, f32, Dir);
    type Skeleton = GolemSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"golem_beam\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "golem_beam")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (stage_section, _global_time, _timer, look_dir): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (move1base, move1iso, move2base, move3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), anim_time.powf(0.25), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, 0.0, 1.0, 0.0),
            Some(StageSection::Recover) => (1.0, 0.0, 1.0, anim_time.powf(4.0)),
            _ => (0.0, 0.0, 0.0, 0.0),
        };

        let pullback = 1.0 - move3;
        let move1 = move1base * pullback;
        let move2 = move2base * pullback;

        next.head.orientation = Quaternion::rotation_x(move1iso * 0.5 + move2 * (look_dir.z * 1.0));
        next.head.position = Vec3::new(
            0.0,
            s_a.head.0,
            s_a.head.1 - move2 * 5.0 * (look_dir.z * 1.0).min(0.0),
        );

        next.upper_torso.orientation =
            Quaternion::rotation_x(move1iso * 0.3) * Quaternion::rotation_z(0.0);

        next.lower_torso.orientation =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(move1iso * -0.3);

        next.shoulder_l.orientation =
            Quaternion::rotation_x(move1 * 0.8) * Quaternion::rotation_y(move1 * -0.5);

        next.shoulder_r.orientation =
            Quaternion::rotation_x(move1 * 0.8) * Quaternion::rotation_y(move1 * 0.5);
        next.shoulder_l.position = Vec3::new(
            -s_a.shoulder.0,
            s_a.shoulder.1 + move1 * 2.0,
            s_a.shoulder.2 + move1 * -2.0,
        );
        next.shoulder_r.position = Vec3::new(
            s_a.shoulder.0,
            s_a.shoulder.1 + move1 * 2.0,
            s_a.shoulder.2 + move1 * -2.0,
        );

        next.hand_l.orientation =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_y(move1 * -1.1);

        next.hand_r.orientation = Quaternion::rotation_y(0.0) * Quaternion::rotation_y(move1 * 1.1);

        next.torso.position = Vec3::new(0.0, 0.0, 0.0);
        next
    }
}
