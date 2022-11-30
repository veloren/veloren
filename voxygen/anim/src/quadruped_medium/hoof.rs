use super::{
    super::{vek::*, Animation},
    QuadrupedMediumSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
use std::f32::consts::PI;

pub struct HoofAnimation;

impl Animation for HoofAnimation {
    type Dependency<'a> = (f32, f32, Option<StageSection>, f32);
    type Skeleton = QuadrupedMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_medium_hoof\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_medium_hoof")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, global_time, stage_section, timer): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = (Vec2::<f32>::from(velocity).magnitude()).min(24.0);

        let (movement1base, movement2base, movement3, twitch) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time.powf(0.25), 0.0, anim_time),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4), 1.0),
            _ => (0.0, 0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - movement3;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let movement1 = movement1base * mirror * pullback;
        let movement2 = movement2base * mirror * pullback;
        let movement1abs = movement1base * pullback;
        let movement2abs = movement2base * pullback;
        let twitchleft = (twitch * 16.0).sin() * pullback * mirror;
        let twitchright = (twitch * 8.0 + PI / 2.0).sin() * pullback * mirror;

        next.head.orientation = Quaternion::rotation_x(movement1abs * -0.6 + movement2abs * 0.6)
            * Quaternion::rotation_y(movement1 * 0.35 + movement2 * -0.15)
            * Quaternion::rotation_z(movement1 * 0.15 + movement2 * -0.5);

        next.neck.orientation = Quaternion::rotation_x(movement1abs * -0.8 + movement2abs * -0.2)
            * Quaternion::rotation_y(movement1 * 0.0)
            * Quaternion::rotation_z(movement1 * 0.10 + movement1 * -0.15);

        next.jaw.orientation = Quaternion::rotation_x(movement1abs * -0.7 + movement2abs * 0.7);

        next.tail.orientation = Quaternion::rotation_x(movement1 * 0.5 + movement2 * -0.8);
        next.torso_front.position = Vec3::new(
            0.0,
            s_a.torso_front.0 + movement1abs * -6.0,
            s_a.torso_front.1 + movement1abs * 9.0,
        );
        next.torso_front.orientation = Quaternion::rotation_x(movement1abs * 1.2);

        next.torso_back.orientation = Quaternion::rotation_x(movement1abs * -0.8);

        next.ears.orientation = Quaternion::rotation_x(0.0);
        next.leg_fl.orientation = Quaternion::rotation_x(movement1abs * 0.8 + twitchleft * 0.5)
            * Quaternion::rotation_y(movement1 * -0.3 + movement2 * 0.3)
            * Quaternion::rotation_z(movement1 * -0.35 + movement2 * 0.45);

        next.leg_fr.orientation = Quaternion::rotation_x(movement1abs * 0.8 + twitchright * 0.5)
            * Quaternion::rotation_y(movement1 * -0.3 + movement2 * 0.3)
            * Quaternion::rotation_z(movement1 * -0.35 + movement2 * 0.45);

        next.foot_fl.orientation =
            Quaternion::rotation_x(movement1abs * -0.9 + twitchleft * 0.5 + movement2abs * 0.6);

        next.foot_fr.orientation =
            Quaternion::rotation_x(movement1abs * -0.9 + twitchright * 0.5 + movement2abs * 0.6);
        if speed < 0.5 {
            next.leg_bl.orientation = Quaternion::rotation_x(movement1abs * -0.2);

            next.leg_br.orientation = Quaternion::rotation_x(movement1abs * -0.2);

            next.foot_bl.orientation = Quaternion::rotation_x(movement1abs * -0.2);

            next.foot_br.orientation = Quaternion::rotation_x(movement1abs * -0.2);
        };
        next
    }
}
