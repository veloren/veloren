use super::{
    super::{vek::*, Animation},
    QuadrupedMediumSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct LeapMeleeAnimation;

impl Animation for LeapMeleeAnimation {
    type Dependency = (f32, f64, Option<StageSection>, f64);
    type Skeleton = QuadrupedMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_medium_leapmelee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_medium_leapmelee")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, global_time, stage_section, timer): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        //let speed = (Vec2::<f32>::from(velocity).magnitude()).min(24.0);

        let (movement1base, movement2base, movement3) = match stage_section {
            Some(StageSection::Buildup) => ((anim_time as f32).powf(1.0), 0.0, 0.0),
            Some(StageSection::Swing) => (1.0, anim_time as f32, 0.0),
            Some(StageSection::Recover) => (0.0, 1.0, (anim_time as f32).powf(4.0)),
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - movement3;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum() as f32;
        let movement1 = movement1base * mirror * pullback;
        let movement1abs = movement1base * pullback;
        //let movement2 = movement2base*mirror*pullback;
        let movement2abs = movement2base * pullback;
        let twitch1 = (movement1 * 10.0).sin() * pullback;
        let twitch2 = (movement3 * 5.0).sin() * pullback;
        let twitchmovement = twitch1 + twitch2;

        next.head.orientation = Quaternion::rotation_x(movement1abs * 0.2 + movement2abs * 0.2)
            * Quaternion::rotation_y(twitchmovement * 0.3 * mirror);

        next.neck.orientation = Quaternion::rotation_x(movement1abs * -0.2)
            * Quaternion::rotation_y(twitchmovement * 0.1 * mirror);

        next.jaw.orientation = Quaternion::rotation_x(twitchmovement * 0.1);

        next.tail.orientation = Quaternion::rotation_z(twitchmovement * 1.0 * mirror);
        next.torso_front.position = Vec3::new(
            0.0,
            s_a.torso_front.0 + movement1abs * -4.0,
            s_a.torso_front.1,
        ) * s_a.scaler
            / 11.0;
        next.torso_front.orientation = Quaternion::rotation_x(movement1abs * 0.3)
            * Quaternion::rotation_y(twitchmovement * -0.1 * mirror);

        next.torso_back.orientation = Quaternion::rotation_x(movement1abs * -0.45)
            * Quaternion::rotation_y(twitchmovement * 0.1 * mirror);

        next.ears.orientation = Quaternion::rotation_x(twitchmovement * 0.1);
        next.leg_fl.orientation = Quaternion::rotation_x(movement1abs * 0.8)
            * Quaternion::rotation_y(twitchmovement * 0.1 * mirror);

        next.leg_fr.orientation = Quaternion::rotation_x(movement1abs * 0.8)
            * Quaternion::rotation_y(twitchmovement * 0.1 * mirror);

        next.leg_bl.orientation = Quaternion::rotation_x(movement1abs * 0.4);

        next.leg_br.orientation = Quaternion::rotation_x(movement1abs * 0.4);

        next.foot_fl.orientation = Quaternion::rotation_x(movement1abs * -0.9);

        next.foot_fr.orientation = Quaternion::rotation_x(movement1abs * -0.9);

        next.foot_bl.orientation = Quaternion::rotation_x(movement1abs * -1.1);

        next.foot_br.orientation = Quaternion::rotation_x(movement1abs * -1.1);

        next
    }
}
