use super::{
    super::{vek::*, Animation},
    QuadrupedMediumSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct StunnedAnimation;

impl Animation for StunnedAnimation {
    type Dependency<'a> = (f32, f32, Option<StageSection>, f32);
    type Skeleton = QuadrupedMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_medium_stunned\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_medium_stunned")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, global_time, stage_section, timer): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (movement1base, movement2, twitch) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
            Some(StageSection::Recover) => {
                (1.0, anim_time.powf(3.0), ((1.0 - anim_time) * 7.0).sin())
            },
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - movement2;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let movement1 = movement1base * mirror * pullback;
        let movement1abs = movement1base * pullback;

        next.head.orientation = Quaternion::rotation_x(movement1abs * -0.55)
            * Quaternion::rotation_y(movement1 * 0.35)
            * Quaternion::rotation_z(movement1 * 0.15 + twitch * 0.3 * mirror);

        next.neck.orientation = Quaternion::rotation_x(movement1abs * -0.4)
            * Quaternion::rotation_y(movement1 * 0.0)
            * Quaternion::rotation_z(movement1 * 0.10 + movement1 * -0.15);

        next.jaw.orientation = Quaternion::rotation_x(0.0);

        next.tail.orientation = Quaternion::rotation_z(movement1 * 0.5);
        next.torso_front.position = Vec3::new(
            0.0,
            s_a.torso_front.0 + movement1abs * -4.0,
            s_a.torso_front.1,
        );
        next.torso_front.orientation =
            Quaternion::rotation_y(0.0) * Quaternion::rotation_z(movement1 * 0.15);

        next.torso_back.orientation =
            Quaternion::rotation_y(movement1 * 0.18) * Quaternion::rotation_z(movement1 * -0.4);

        next.ears.orientation = Quaternion::rotation_x(twitch * 0.1 * mirror);

        next.leg_fl.position = Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fl.orientation = Quaternion::rotation_y(0.0);

        next.leg_fr.position = Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fr.orientation = Quaternion::rotation_y(0.0);

        next.leg_bl.position = Vec3::new(-s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_bl.orientation = Quaternion::rotation_y(movement1 * -0.3);

        next.leg_br.position = Vec3::new(s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_br.orientation = Quaternion::rotation_y(movement1 * -0.3);

        next.foot_fl.position = Vec3::new(-s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2);
        next.foot_fl.orientation = Quaternion::rotation_x(movement1abs * 0.2);

        next.foot_fr.position = Vec3::new(s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2);
        next.foot_fr.orientation = Quaternion::rotation_x(movement1abs * 0.2);

        next.foot_bl.position = Vec3::new(-s_a.feet_b.0, s_a.feet_b.1, s_a.feet_b.2);

        next.foot_br.position = Vec3::new(s_a.feet_b.0, s_a.feet_b.1, s_a.feet_b.2);

        next
    }
}
