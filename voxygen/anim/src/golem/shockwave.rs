use super::{
    super::{vek::*, Animation},
    GolemSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
use std::f32::consts::PI;

pub struct ShockwaveAnimation;

impl Animation for ShockwaveAnimation {
    type Dependency<'a> = (Option<StageSection>, f32, f32);
    type Skeleton = GolemSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"golem_shockwave\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "golem_shockwave")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (stage_section, velocity, _global_time): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (move1base, move2base, move3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powf(2.0)),
            _ => (0.0, 0.0, 0.0),
        };

        let pullback = 1.0 - move3;
        let move1 = move1base * pullback;
        let move2 = move2base * pullback;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1) * 1.02;
        next.head.orientation = Quaternion::rotation_z(move1 * -PI);

        next.upper_torso.position =
            Vec3::new(0.0, s_a.upper_torso.0, s_a.upper_torso.1 + move2 * -5.0);
        next.upper_torso.orientation = Quaternion::rotation_z(move1 * -PI);

        next.lower_torso.position =
            Vec3::new(0.0, s_a.lower_torso.0, s_a.lower_torso.1 + move2 * 2.0);
        next.lower_torso.orientation = Quaternion::rotation_z(move1 * PI);

        next.shoulder_l.position = Vec3::new(
            -s_a.shoulder.0 - 2.0,
            s_a.shoulder.1,
            s_a.shoulder.2 + move2 * -1.0,
        );
        next.shoulder_l.orientation = Quaternion::rotation_y(move1 * 1.0 + move2 * -1.2);

        next.shoulder_r.position = Vec3::new(
            s_a.shoulder.0 + 2.0,
            s_a.shoulder.1,
            s_a.shoulder.2 + move2 * -1.0,
        );
        next.shoulder_r.orientation = Quaternion::rotation_y(move1 * -1.0 + move2 * 1.2);

        next.hand_l.orientation = Quaternion::rotation_y(move1 * -1.0 + move2 * 1.2);

        next.hand_r.orientation =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_y(move1 * 1.0 + move2 * -1.2);
        if velocity < 0.5 {
            next.leg_l.position = Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2 + move2 * 2.0) * 1.02;

            next.leg_r.position = Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2 + move2 * 2.0) * 1.02;

            next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2 + move2);

            next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2 + move2);
        } else {
        }
        next
    }
}
