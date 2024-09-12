use super::{
    super::{
        util::{bounce, elastic},
        vek::*,
        Animation,
    },
    QuadrupedLowSkeleton, SkeletonAttr,
};
use common::{comp::body::parts::HeadState, states::utils::StageSection};
//use std::ops::Rem;
use std::f32::consts::PI;

pub struct DashAnimation;

impl Animation for DashAnimation {
    type Dependency<'a> = (
        Option<&'a str>,
        f32,
        f32,
        Option<StageSection>,
        f32,
        [HeadState; 3],
    );
    type Skeleton = QuadrupedLowSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_low_dash\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_low_dash")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_id, _velocity, global_time, stage_section, timer, heads): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        match ability_id {
            Some("common.abilities.custom.rocksnapper.dash") => {
                let (buildup, charge, _action, recover) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0, 0.0),
                    Some(StageSection::Charge) => (1.0, anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, 1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0, 0.0),
                };
                let quick_buildup = buildup.powf(0.2);
                let elastic_recover = elastic(recover);
                next.head_c_upper.position = Vec3::new(
                    0.0,
                    s_a.head_upper.0 + (-1.0 * quick_buildup + elastic_recover) * 10.0,
                    s_a.head_upper.1,
                );
                next.head_c_upper.scale = Vec3::one() * (1.0 - buildup + elastic_recover);
                next.head_c_lower.position = Vec3::new(
                    0.0,
                    s_a.head_lower.0 + (-1.0 * quick_buildup + elastic_recover) * 10.0,
                    s_a.head_lower.1,
                );
                next.head_c_lower.scale = Vec3::one()
                    * (1.0 - buildup + elastic_recover)
                    * heads[1].is_attached() as i32 as f32;
                next.foot_fl.position = Vec3::new(
                    -s_a.feet_f.0 + (quick_buildup - elastic_recover) * 8.0,
                    s_a.feet_f.1 + (-1.0 * quick_buildup + elastic_recover) * 8.0,
                    s_a.feet_f.2 + (quick_buildup - elastic_recover) * 8.0,
                );
                next.foot_fl.scale = Vec3::one() * (1.0 - buildup + elastic_recover);
                next.foot_fr.position = Vec3::new(
                    s_a.feet_f.0 - (quick_buildup - elastic_recover) * 8.0,
                    s_a.feet_f.1 + (-1.0 * quick_buildup + elastic_recover) * 8.0,
                    s_a.feet_f.2 + (quick_buildup - elastic_recover) * 8.0,
                );
                next.foot_fr.scale = Vec3::one() * (1.0 - buildup + elastic_recover);
                next.foot_bl.position = Vec3::new(
                    -s_a.feet_b.0 + (quick_buildup - elastic_recover) * 8.0,
                    s_a.feet_b.1 + (quick_buildup - elastic_recover) * 8.0,
                    s_a.feet_b.2 + (quick_buildup - elastic_recover) * 8.0,
                );
                next.foot_bl.scale = Vec3::one() * (1.0 - buildup + elastic_recover);
                next.foot_br.position = Vec3::new(
                    s_a.feet_b.0 - (quick_buildup - elastic_recover) * 8.0,
                    s_a.feet_b.1 + (quick_buildup - elastic_recover) * 8.0,
                    s_a.feet_b.2 + (quick_buildup - elastic_recover) * 8.0,
                );
                next.foot_br.scale = Vec3::one() * (1.0 - buildup + elastic_recover);
                next.tail_front.position = Vec3::new(
                    0.0,
                    s_a.tail_front.0 + (quick_buildup - elastic_recover) * 20.0,
                    s_a.tail_front.1,
                );
                next.tail_front.scale = Vec3::one() * (1.0 - buildup + elastic_recover);
                next.tail_rear.position = Vec3::new(
                    0.0,
                    s_a.tail_rear.0 + (quick_buildup - elastic_recover) * 20.0,
                    s_a.tail_rear.1,
                );
                next.tail_rear.scale = Vec3::one() * (1.0 - buildup + recover);

                next.chest.position = Vec3::new(
                    0.0,
                    0.0,
                    s_a.chest.1 - bounce(buildup) * 5.0 + elastic(recover) * 5.0,
                );
                next.chest.orientation =
                    Quaternion::rotation_z(2.0 * PI * buildup + 4.0 * PI * charge);
            },
            _ => {
                let (buildup, chargemovementbase, action, recover) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.sqrt(), 0.0, 0.0, 0.0),
                    Some(StageSection::Charge) => (1.0, 1.0, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, 1.0, anim_time.powi(4), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - recover;
                let subtract = global_time - timer;
                let check = subtract - subtract.trunc();
                let mirror = (check - 0.5).signum();
                let twitch1 = (mirror * buildup * 9.5).sin();
                let twitch1fast = (mirror * buildup * 25.0).sin();
                let buildup_abs = buildup * pullback;
                let action_abs = action * pullback;
                let short = ((1.0
                    / (0.72 + 0.28 * ((anim_time * 16.0_f32 + PI * 0.25).sin()).powi(2)))
                .sqrt())
                    * ((anim_time * 16.0_f32 + PI * 0.25).sin())
                    * chargemovementbase
                    * pullback;
                let shortalt =
                    (anim_time * 16.0_f32 + PI * 0.25).sin() * chargemovementbase * pullback;

                // Central head
                next.head_c_upper.orientation =
                    Quaternion::rotation_x(buildup_abs * 0.4 + action_abs * 0.3)
                        * Quaternion::rotation_z(short * -0.06 + twitch1 * -0.3);

                next.head_c_lower.orientation =
                    Quaternion::rotation_x(buildup_abs * -0.4 + action_abs * -0.5)
                        * Quaternion::rotation_z(short * 0.15 + twitch1 * 0.3);

                next.jaw_c.orientation = Quaternion::rotation_x(
                    twitch1fast * 0.2
                        + buildup_abs * -0.3
                        + action_abs * 1.2
                        + chargemovementbase * -0.5,
                );

                // Left head
                next.head_l_upper.orientation =
                    Quaternion::rotation_x(buildup_abs * 0.4 + action_abs * 0.3)
                        * Quaternion::rotation_z(short * -0.06 + twitch1 * -0.3);

                next.head_l_lower.orientation =
                    Quaternion::rotation_x(buildup_abs * -0.4 + action_abs * -0.5)
                        * Quaternion::rotation_z(short * 0.15 + twitch1 * 0.3);

                next.jaw_l.orientation = Quaternion::rotation_x(
                    twitch1fast * 0.2
                        + buildup_abs * -0.3
                        + action_abs * 1.2
                        + chargemovementbase * -0.5,
                );

                // Right head
                next.head_r_upper.orientation =
                    Quaternion::rotation_x(buildup_abs * 0.4 + action_abs * 0.3)
                        * Quaternion::rotation_z(short * -0.06 + twitch1 * -0.3);

                next.head_r_lower.orientation =
                    Quaternion::rotation_x(buildup_abs * -0.4 + action_abs * -0.5)
                        * Quaternion::rotation_z(short * 0.15 + twitch1 * 0.3);

                next.jaw_r.orientation = Quaternion::rotation_x(
                    twitch1fast * 0.2
                        + buildup_abs * -0.3
                        + action_abs * 1.2
                        + chargemovementbase * -0.5,
                );

                next.chest.orientation =
                    Quaternion::rotation_z(twitch1 * 0.06) * Quaternion::rotation_y(short * 0.06);

                next.tail_front.orientation = Quaternion::rotation_x(
                    0.15 + buildup_abs * -0.4 + action_abs * 0.2 + chargemovementbase * 0.2,
                ) * Quaternion::rotation_z(shortalt * 0.15);

                next.tail_rear.orientation =
                    Quaternion::rotation_x(
                        -0.12 + buildup_abs * -0.4 + action_abs * 0.2 + chargemovementbase * 0.2,
                    ) * Quaternion::rotation_z(shortalt * 0.15 + twitch1fast * 0.3);
            },
        }
        next
    }
}
