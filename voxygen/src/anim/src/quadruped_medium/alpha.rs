use super::{
    super::{vek::*, Animation},
    QuadrupedMediumSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct AlphaAnimation;

impl Animation for AlphaAnimation {
    type Dependency = (f32, f64, Option<StageSection>);
    type Skeleton = QuadrupedMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_medium_alpha\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_medium_alpha")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, _global_time, stage_section): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = (Vec2::<f32>::from(velocity).magnitude()).min(24.0);

        let (movement1, movement2, movement3) = match stage_section {
            Some(StageSection::Buildup) => ((anim_time as f32).powf(0.25), 0.0, 0.0),
            Some(StageSection::Swing) => (1.0, anim_time as f32, 0.0),
            Some(StageSection::Recover) => (0.0, 1.0, (anim_time as f32).powf(4.0)),
            _ => (0.0, 0.0, 0.0),
        };

        if let Some(stage_section) = stage_section {
            match stage_section {
                StageSection::Buildup | StageSection::Recover | StageSection::Swing => {
                    let twitch1 = (movement1 * 20.0).sin();
                    let twitch2 = (movement3 * 5.0).sin();
                    let twitchmovement = twitch1 + twitch2;

                    next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
                    next.head.orientation =
                        Quaternion::rotation_x(
                            (movement1 * 0.4 + movement2 * 0.4) * (1.0 - movement3),
                        ) * Quaternion::rotation_y(twitchmovement * 0.2 * (1.0 - movement3));

                    next.neck.position = Vec3::new(0.0, s_a.neck.0, s_a.neck.1);
                    next.neck.orientation =
                        Quaternion::rotation_x(movement1 * -0.7 * (1.0 - movement3))
                            * Quaternion::rotation_y(twitchmovement * 0.1);
                    next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
                    next.jaw.orientation = Quaternion::rotation_x(twitchmovement * 0.1);
                    next.jaw.scale = Vec3::one() * 1.02;

                    next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
                    next.tail.orientation = Quaternion::rotation_z(twitchmovement * 1.0);

                    next.torso_front.position =
                        Vec3::new(0.0, s_a.torso_front.0, s_a.torso_front.1) * s_a.scaler / 11.0;
                    next.torso_front.orientation =
                        Quaternion::rotation_x(movement1 * 0.2 * (1.0 - movement3))
                            * Quaternion::rotation_y(twitchmovement * -0.1);

                    next.torso_back.position = Vec3::new(0.0, s_a.torso_back.0, s_a.torso_back.1);
                    next.torso_back.orientation =
                        Quaternion::rotation_x(movement1 * -0.3 * (1.0 - movement3))
                            * Quaternion::rotation_y(twitchmovement * 0.1);

                    next.ears.position = Vec3::new(0.0, s_a.ears.0, s_a.ears.1);
                    next.ears.orientation =
                        Quaternion::rotation_x(twitchmovement * 0.1 * (1.0 - movement3));
                    if speed < 0.5 {
                        next.leg_fl.position = Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
                        next.leg_fl.orientation =
                            Quaternion::rotation_x(movement1 * 0.6 * (1.0 - movement3))
                                * Quaternion::rotation_y(twitchmovement * 0.1);

                        next.leg_fr.position = Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
                        next.leg_fr.orientation =
                            Quaternion::rotation_x(movement1 * 0.6 * (1.0 - movement3))
                                * Quaternion::rotation_y(twitchmovement * 0.1);

                        next.leg_bl.position = Vec3::new(-s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
                        next.leg_bl.orientation =
                            Quaternion::rotation_x(movement1 * 0.5 * (1.0 - movement3));

                        next.leg_br.position = Vec3::new(s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
                        next.leg_br.orientation =
                            Quaternion::rotation_x(movement1 * 0.5 * (1.0 - movement3));

                        next.foot_fl.position =
                            Vec3::new(-s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2);
                        next.foot_fl.orientation =
                            Quaternion::rotation_x(movement1 * -0.5 * (1.0 - movement3));

                        next.foot_fr.position = Vec3::new(s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2);
                        next.foot_fr.orientation =
                            Quaternion::rotation_x(movement1 * -0.5 * (1.0 - movement3));

                        next.foot_bl.position =
                            Vec3::new(-s_a.feet_b.0, s_a.feet_b.1, s_a.feet_b.2);
                        next.foot_bl.orientation =
                            Quaternion::rotation_x(movement1 * -1.0 * (1.0 - movement3));

                        next.foot_br.position = Vec3::new(s_a.feet_b.0, s_a.feet_b.1, s_a.feet_b.2);
                        next.foot_br.orientation =
                            Quaternion::rotation_x(movement1 * -1.0 * (1.0 - movement3));
                    };
                },
                StageSection::Charge => {
                    next.jaw.orientation = Quaternion::rotation_x(-1.0);
                },
                _ => {},
            }
        }
        next
    }
}
