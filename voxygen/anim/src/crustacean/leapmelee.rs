use super::{
    super::{vek::*, Animation},
    CrustaceanSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct LeapMeleeAnimation;

impl Animation for LeapMeleeAnimation {
    type Dependency<'a> = (Option<&'a str>, f32, f32, Option<StageSection>, f32);
    type Skeleton = CrustaceanSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"crustacean_leapmelee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "crustacean_leapmelee")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_id, _velocity, global_time, stage_section, timer): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

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
        let _twitch3 = (movement3base * 5.0).sin() * mirror;

        let twitch1abs = twitch1 * mirror;

        next.leg_fl.position =
            Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2 - movement2abs * 3.0);
        next.leg_fr.position =
            Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2 - movement2abs * 3.0);
        next.leg_cl.position =
            Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2 - movement2abs * 3.0);
        next.leg_cr.position =
            Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2 - movement2abs * 3.0);
        next.leg_bl.position =
            Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2 - movement2abs * 3.0);
        next.leg_br.position =
            Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2 - movement2abs * 3.0);

        match ability_id {
            Some("common.abilities.custom.karkatha.leap") => {
                next.chest.orientation = Quaternion::rotation_x(
                    movement1abs * 0.3 + movement2abs * -0.3 + movement3abs * 0.3,
                ) * Quaternion::rotation_y(twitch1abs * -0.3);

                next.arm_r.orientation =
                    Quaternion::rotation_z(movement1abs * 0.7 - movement3abs * 1.2);
                next.arm_l.orientation =
                    Quaternion::rotation_z(movement1abs * -0.3 + movement3abs * 0.5);
            },

            Some("common.abilities.custom.karkatha.spinleap") => {
                next.chest.orientation = Quaternion::rotation_x(
                    movement1abs * 0.3 + movement2abs * -0.3 + movement3abs * 0.3,
                ) * Quaternion::rotation_y(twitch1abs * -0.3)
                    * Quaternion::rotation_z(
                        movement1abs * -6.0 + movement2abs * 6.0 + movement3abs * -7.0,
                    );

                next.pincer_r0.orientation = Quaternion::rotation_z(movement2abs * 1.2);

                next.pincer_r0.position =
                    Vec3::new(0.0 - movement2abs * 12.0, 0.0 + movement2abs * 12.0, 0.0);

                next.pincer_l0.orientation = Quaternion::rotation_z(movement2abs * -1.2);

                next.pincer_l0.position =
                    Vec3::new(0.0 + movement2abs * 12.0, 0.0 + movement2abs * 12.0, 0.0);
            },
            _ => {},
        }

        next
    }
}
