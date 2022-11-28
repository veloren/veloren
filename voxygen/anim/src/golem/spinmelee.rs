use super::{
    super::{vek::*, Animation},
    GolemSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
use core::f32::consts::PI;

pub struct SpinMeleeAnimation;

impl Animation for SpinMeleeAnimation {
    type Dependency<'a> = Option<StageSection>;
    type Skeleton = GolemSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"golem_spinmelee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "golem_spinmelee")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        stage_section: Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (movement1, movement2, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powf(4.0)),
            _ => (0.0, 0.0, 0.0),
        };

        let pullback = 1.0 - movement3;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1) * 1.02;
        next.head.orientation =
            Quaternion::rotation_z(movement1 * 0.5 * PI + movement2 * -2.5 * PI)
                * Quaternion::rotation_x(-0.2);

        next.upper_torso.position =
            Vec3::new(0.0, s_a.upper_torso.0, s_a.upper_torso.1 + movement1 * -6.0);
        next.upper_torso.orientation =
            Quaternion::rotation_z(movement1 * -0.5 * PI + movement2 * 2.5 * PI);

        next.lower_torso.position = Vec3::new(0.0, s_a.lower_torso.0, s_a.lower_torso.1);
        next.lower_torso.orientation =
            Quaternion::rotation_z(movement1 * 0.5 * PI + movement2 * -2.5 * PI);

        next.shoulder_l.position = Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_l.orientation =
            Quaternion::rotation_x(0.0) * Quaternion::rotation_x(movement1 * 1.2 * pullback);

        next.shoulder_r.position = Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_r.orientation =
            Quaternion::rotation_x(0.0) * Quaternion::rotation_x(movement1 * -1.2 * pullback);

        next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2);
        next.hand_l.orientation = Quaternion::rotation_x(movement1 * -0.2 * pullback);

        next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2);
        next.hand_r.orientation = Quaternion::rotation_x(movement1 * 0.2 * pullback);

        next.leg_l.position = Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2 + movement1 * 2.0) * 1.02;
        next.leg_l.orientation = Quaternion::rotation_x(0.0);

        next.leg_r.position = Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2 + movement1 * 2.0) * 1.02;
        next.leg_r.orientation = Quaternion::rotation_x(0.0);

        next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2 + movement1 * 4.0);
        next.foot_l.orientation = Quaternion::rotation_x(0.0);

        next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2 + movement1 * 4.0);
        next.foot_r.orientation = Quaternion::rotation_x(0.0);

        next.torso.position = Vec3::new(0.0, 0.0, 0.0);
        next.torso.orientation = Quaternion::rotation_z(0.0);

        next
    }
}
