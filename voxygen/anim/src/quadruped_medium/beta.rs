use super::{
    super::{vek::*, Animation},
    QuadrupedMediumSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct BetaAnimation;

impl Animation for BetaAnimation {
    type Dependency<'a> = (f32, f32, Option<StageSection>, f32);
    type Skeleton = QuadrupedMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_medium_beta\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_medium_beta")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, global_time, stage_section, timer): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = (Vec2::<f32>::from(velocity).magnitude()).min(24.0);

        let (movement1base, movement2base, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time.sqrt(), 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - movement3;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let movement1 = movement1base * mirror * pullback;
        let movement1abs = movement1base * pullback;
        let movement2 = movement2base * mirror * pullback;
        let movement2abs = movement2base * pullback;
        let twitch1 = (movement1 * 10.0).sin() * pullback;
        let twitch2 = (movement2abs * -8.0).sin();
        let twitchmovement = twitch1 + twitch2;

        next.head.orientation = Quaternion::rotation_x(movement1abs * -0.4 + movement2abs * 1.1)
            * Quaternion::rotation_y(movement1 * -0.35 + movement2 * 0.25)
            * Quaternion::rotation_z(movement1 * -0.25 + movement2 * 0.5);

        next.neck.orientation = Quaternion::rotation_x(movement1abs * 0.0 + movement2abs * -0.2)
            * Quaternion::rotation_y(movement1 * 0.0)
            * Quaternion::rotation_z(movement1 * -0.10 + movement1 * 0.15);

        next.jaw.orientation = Quaternion::rotation_x(movement1abs * -0.5 + twitch2 * -0.4);

        next.tail.orientation = Quaternion::rotation_z(
            movement1 * 0.5 + movement2 * -0.8 + twitchmovement * 0.2 * mirror,
        );
        next.torso_front.position = Vec3::new(
            0.0,
            s_a.torso_front.0 + movement1abs * -4.0,
            s_a.torso_front.1,
        );
        next.torso_front.orientation = Quaternion::rotation_y(movement1 * -0.25 * movement2 * 0.25)
            * Quaternion::rotation_z(movement1 * 0.35 + movement2 * -0.45);

        next.torso_back.orientation = Quaternion::rotation_y(movement1 * 0.25 + movement1 * -0.25)
            * Quaternion::rotation_z(movement1 * -0.4 + movement2 * 0.65);

        next.ears.orientation = Quaternion::rotation_x(twitchmovement * 0.2);
        if speed < 0.5 {
            next.leg_fl.orientation =
                Quaternion::rotation_x(movement1abs * 0.8 + movement2abs * -0.6)
                    * Quaternion::rotation_y(movement1 * -0.3 + movement2 * 0.3)
                    * Quaternion::rotation_z(movement1 * -0.35 + movement2 * 0.45);

            next.leg_fr.orientation =
                Quaternion::rotation_x(movement1abs * 0.8 + movement2abs * -0.6)
                    * Quaternion::rotation_y(movement1 * -0.3 + movement2 * 0.3)
                    * Quaternion::rotation_z(movement1 * -0.35 + movement2 * 0.45);

            next.leg_bl.orientation = Quaternion::rotation_x(movement1 * 0.1 + movement2 * -0.3);

            next.leg_br.orientation = Quaternion::rotation_x(movement1 * -0.1 + movement2 * 0.3);

            next.foot_fl.orientation =
                Quaternion::rotation_x(movement1abs * -0.9 + movement2abs * 0.6);

            next.foot_fr.orientation =
                Quaternion::rotation_x(movement1abs * -0.9 + movement2abs * 0.6);

            next.foot_bl.orientation =
                Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * -0.3);

            next.foot_br.orientation =
                Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * -0.3);
        };
        next
    }
}
