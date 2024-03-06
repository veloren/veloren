use super::{
    super::{vek::*, Animation},
    QuadrupedLowSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
//use std::ops::Rem;
use std::f32::consts::PI;

pub struct DashAnimation;

impl Animation for DashAnimation {
    type Dependency<'a> = (Option<&'a str>, f32, f32, Option<StageSection>, f32);
    type Skeleton = QuadrupedLowSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_low_dash\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_low_dash")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_id, _velocity, global_time, stage_section, timer): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        match ability_id {
            Some("common.abilities.custom.rocksnapper.dash") => {
                let (movement1, charge, movement2, movement3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0, 0.0),
                    Some(StageSection::Charge) => (1.0, anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, 1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0, 0.0),
                };
                let subtract = global_time - timer;
                let check = subtract - subtract.trunc();
                let mirror = (check - 0.5).signum();
                let twitch1 = (mirror * movement1.sqrt() * 9.5).sin();
                fn quintic(x: f32) -> f32 { x.powf(0.2) }
                let quick_movement1 = movement1.powf(0.2);
                //let quick_movement3 = movement3.powf(0.2);
                let quick_movement3 = elastic(movement3);
                next.head_upper.position = Vec3::new(
                    0.0,
                    s_a.head_upper.0 + (-1.0 * quick_movement1 + quick_movement3) * 10.0,
                    s_a.head_upper.1,
                );
                next.head_upper.scale = Vec3::one() * (1.0 - movement1 + quick_movement3);
                next.head_lower.position = Vec3::new(
                    0.0,
                    s_a.head_lower.0 + (-1.0 * quick_movement1 + quick_movement3) * 10.0,
                    s_a.head_lower.1,
                );
                next.head_lower.scale = Vec3::one() * (1.0 - movement1 + quick_movement3);
                next.foot_fl.position = Vec3::new(
                    -s_a.feet_f.0 + (quick_movement1 - quick_movement3) * 8.0,
                    s_a.feet_f.1 + (-1.0 * quick_movement1 + quick_movement3) * 8.0,
                    s_a.feet_f.2 + (quick_movement1 - quick_movement3) * 8.0,
                );
                next.foot_fl.scale = Vec3::one() * (1.0 - movement1 + quick_movement3);
                next.foot_fr.position = Vec3::new(
                    s_a.feet_f.0 - (quick_movement1 - quick_movement3) * 8.0,
                    s_a.feet_f.1 + (-1.0 * quick_movement1 + quick_movement3) * 8.0,
                    s_a.feet_f.2 + (quick_movement1 - quick_movement3) * 8.0,
                );
                next.foot_fr.scale = Vec3::one() * (1.0 - movement1 + quick_movement3);
                next.foot_bl.position = Vec3::new(
                    -s_a.feet_b.0 + (quick_movement1 - quick_movement3) * 8.0,
                    s_a.feet_b.1 + (quick_movement1 - quick_movement3) * 8.0,
                    s_a.feet_b.2 + (quick_movement1 - quick_movement3) * 8.0,
                );
                next.foot_bl.scale = Vec3::one() * (1.0 - movement1 + quick_movement3);
                next.foot_br.position = Vec3::new(
                    s_a.feet_b.0 - (quick_movement1 - quick_movement3) * 8.0,
                    s_a.feet_b.1 + (quick_movement1 - quick_movement3) * 8.0,
                    s_a.feet_b.2 + (quick_movement1 - quick_movement3) * 8.0,
                );
                next.foot_br.scale = Vec3::one() * (1.0 - movement1 + quick_movement3);
                next.tail_front.position = Vec3::new(
                    0.0,
                    s_a.tail_front.0 + (quick_movement1 - quick_movement3) * 20.0,
                    s_a.tail_front.1,
                );
                next.tail_front.scale = Vec3::one() * (1.0 - movement1 + quick_movement3);
                next.tail_rear.position = Vec3::new(
                    0.0,
                    s_a.tail_rear.0 + (quick_movement1 - quick_movement3) * 20.0,
                    s_a.tail_rear.1,
                );
                next.tail_rear.scale = Vec3::one() * (1.0 - movement1 + movement3);

                fn bounce(x: f32) -> f32 {
                    if x < (1.0 / 2.75) {
                        7.5625 * x.powi(2)
                    } else if x < (2.0 / 2.75) {
                        7.5625 * (x - (1.5 / 2.75)).powi(2) + 0.75
                    } else if x < (2.5 / 2.75) {
                        7.5625 * (x - (2.25 / 2.75)).powi(2) + 0.9375
                    } else {
                        7.5625 * (x - (2.625 / 2.75)).powi(2) + 0.984375
                    }
                }

                fn elastic(x: f32) -> f32 {
                    fn f(x: f32, a: f32, b: f32) -> f32 {
                        let p = 0.8;
                        b + a * 2.0_f32.powf(a * 10.0 * x) * ((4.0 * PI * x) / p).cos()
                    }
                    f(x, -1.0, 1.0) / f(1.0, -1.0, 1.0)
                }

                next.chest.position = Vec3::new(
                    0.0,
                    0.0,
                    s_a.chest.1 - bounce(movement1) * 5.0 + elastic(movement3) * 5.0,
                );
                let smooth_end_charge = if charge < 0.5 {
                    charge
                } else {
                    3.0 * charge.powi(2) - 2.0 * charge.powi(3)
                };
                next.chest.orientation =
                    Quaternion::rotation_z(2.0 * PI * movement1 + 4.0 * PI * charge);
            },
            _ => {
                let (movement1, chargemovementbase, movement2, movement3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.sqrt(), 0.0, 0.0, 0.0),
                    Some(StageSection::Charge) => (1.0, 1.0, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, 1.0, anim_time.powi(4), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - movement3;
                let subtract = global_time - timer;
                let check = subtract - subtract.trunc();
                let mirror = (check - 0.5).signum();
                let twitch1 = (mirror * movement1 * 9.5).sin();
                let twitch1fast = (mirror * movement1 * 25.0).sin();
                //let twitch3 = (mirror * movement3 * 4.0).sin();
                //let movement1 = mirror * movement1 * pullback;
                //let movement2 = mirror * movement2 * pullback;
                let movement1abs = movement1 * pullback;
                let movement2abs = movement2 * pullback;
                let short = ((1.0
                    / (0.72 + 0.28 * ((anim_time * 16.0_f32 + PI * 0.25).sin()).powi(2)))
                .sqrt())
                    * ((anim_time * 16.0_f32 + PI * 0.25).sin())
                    * chargemovementbase
                    * pullback;
                let shortalt =
                    (anim_time * 16.0_f32 + PI * 0.25).sin() * chargemovementbase * pullback;

                next.head_upper.orientation =
                    Quaternion::rotation_x(movement1abs * 0.4 + movement2abs * 0.3)
                        * Quaternion::rotation_z(short * -0.06 + twitch1 * -0.3);

                next.head_lower.orientation =
                    Quaternion::rotation_x(movement1abs * -0.4 + movement2abs * -0.5)
                        * Quaternion::rotation_z(short * 0.15 + twitch1 * 0.3);

                next.jaw.orientation = Quaternion::rotation_x(
                    twitch1fast * 0.2
                        + movement1abs * -0.3
                        + movement2abs * 1.2
                        + chargemovementbase * -0.5,
                );
                next.chest.orientation =
                    Quaternion::rotation_z(twitch1 * 0.06) * Quaternion::rotation_y(short * 0.06);

                next.tail_front.orientation = Quaternion::rotation_x(
                    0.15 + movement1abs * -0.4 + movement2abs * 0.2 + chargemovementbase * 0.2,
                ) * Quaternion::rotation_z(shortalt * 0.15);

                next.tail_rear.orientation =
                    Quaternion::rotation_x(
                        -0.12 + movement1abs * -0.4 + movement2abs * 0.2 + chargemovementbase * 0.2,
                    ) * Quaternion::rotation_z(shortalt * 0.15 + twitch1fast * 0.3);
            },
        }
        next
    }
}
