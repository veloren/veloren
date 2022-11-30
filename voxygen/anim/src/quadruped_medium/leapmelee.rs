use super::{
    super::{vek::*, Animation},
    QuadrupedMediumSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct LeapMeleeAnimation;

impl Animation for LeapMeleeAnimation {
    type Dependency<'a> = (f32, f32, Option<StageSection>, f32);
    type Skeleton = QuadrupedMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_medium_leapmelee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_medium_leapmelee")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, global_time, stage_section, timer): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        //let speed = (Vec2::<f32>::from(velocity).magnitude()).min(24.0);

        let (movement1base, movement2base, movement3base, movement4) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0, 0.0),
            Some(StageSection::Movement) => (1.0, anim_time, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, 1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (0.0, 1.0, 1.0, anim_time.powi(4)),
            _ => (0.0, 0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - movement4;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let movement1abs = movement1base * pullback;
        let movement2abs = movement2base * pullback;
        let movement3abs = movement3base * pullback;

        let twitch1 = (movement1base * 10.0).sin() * (1.0 - movement2base);
        let twitch3 = (movement3base * 5.0).sin() * mirror;
        let twitch1abs = twitch1 * mirror;

        next.head.orientation = Quaternion::rotation_x(movement1abs * 0.2 + movement3abs * -0.7)
            * Quaternion::rotation_y(twitch1abs * 0.3 + twitch3 * 0.7);

        next.neck.orientation = Quaternion::rotation_x(movement1abs * -0.2 + movement1abs * -0.2)
            * Quaternion::rotation_y(twitch1abs * 0.1);

        next.jaw.orientation = Quaternion::rotation_x(movement1abs * -0.4 + twitch1 * 0.2);

        next.tail.orientation = Quaternion::rotation_z(twitch1abs * 1.0);
        next.torso_front.position = Vec3::new(
            0.0,
            s_a.torso_front.0 + movement1abs * -4.0,
            s_a.torso_front.1,
        );
        next.torso_front.orientation =
            Quaternion::rotation_x(movement1abs * 0.3 + movement2abs * -0.3 + movement3abs * 0.3)
                * Quaternion::rotation_y(twitch1abs * -0.1);

        next.torso_back.orientation =
            Quaternion::rotation_x(movement1abs * -0.45) * Quaternion::rotation_y(twitch1abs * 0.1);

        next.ears.orientation = Quaternion::rotation_x(twitch1 * 0.1);
        next.leg_fl.orientation = Quaternion::rotation_x(movement1abs * 0.8 + movement2abs * 0.4)
            * Quaternion::rotation_y(twitch1abs * 0.1);

        next.leg_fr.orientation = Quaternion::rotation_x(movement1abs * 0.8 + movement2abs * 0.4)
            * Quaternion::rotation_y(twitch1abs * 0.1);

        next.leg_bl.orientation = Quaternion::rotation_x(movement1abs * 0.4 + movement2abs * -1.2);

        next.leg_br.orientation = Quaternion::rotation_x(movement1abs * 0.4 + movement2abs * -1.2);

        next.foot_fl.orientation = Quaternion::rotation_x(movement1abs * -0.9);

        next.foot_fr.orientation = Quaternion::rotation_x(movement1abs * -0.9);

        next.foot_bl.orientation = Quaternion::rotation_x(movement1abs * -1.1);

        next.foot_br.orientation = Quaternion::rotation_x(movement1abs * -1.1);

        next
    }
}
