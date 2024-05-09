use super::{
    super::{
        util::{elastic, out_and_in},
        vek::*,
        Animation,
    },
    QuadrupedLowSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
use core::f32::consts::PI;

pub struct LeapShockAnimation;

impl Animation for LeapShockAnimation {
    type Dependency<'a> = (Option<&'a str>, Vec3<f32>, f32, Option<StageSection>);
    type Skeleton = QuadrupedLowSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_low_leapshockwave\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_low_leapshockwave")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_id, _velocity, _global_time, stage_section): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let (buildup, movement, action, recover) = match stage_section {
            Some(StageSection::Buildup) => (anim_time, 0.0, 0.0, 0.0),
            Some(StageSection::Movement) => (1.0, anim_time, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, 1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0, 0.0),
        };
        match ability_id {
            Some("common.abilities.custom.rocksnapper.leapshockwave") => {
                let elastic_recover = elastic(recover);
                next.head_upper.scale = Vec3::one() * (1.0 - movement + elastic_recover);
                next.head_upper.position = Vec3::new(
                    0.0,
                    s_a.head_upper.0 + (-1.0 * movement + elastic_recover) * 10.0,
                    s_a.head_upper.1,
                );
                next.head_lower.scale = Vec3::one() * (1.0 - movement + elastic_recover);
                next.head_lower.position = Vec3::new(
                    0.0,
                    s_a.head_lower.0 + (-1.0 * movement + elastic_recover) * 15.0,
                    s_a.head_lower.1,
                );
                next.tail_front.scale = Vec3::one() * (1.0 - movement + elastic_recover);
                next.tail_rear.position = Vec3::new(
                    0.0,
                    s_a.tail_rear.0 + (movement - elastic_recover) * 20.0,
                    s_a.tail_rear.1,
                );
                next.tail_rear.scale = Vec3::one() * (1.0 - movement + recover);
                next.foot_fl.scale = Vec3::one() * (1.0 - movement + elastic_recover);
                next.foot_fl.position = Vec3::new(
                    -s_a.feet_f.0 + (movement - elastic_recover) * 8.0,
                    s_a.feet_f.1 + (-1.0 * movement + elastic_recover) * 8.0,
                    s_a.feet_f.2 - out_and_in(buildup) * 15.0 + (movement - elastic_recover) * 8.0,
                );
                next.foot_fr.scale = Vec3::one() * (1.0 - movement + elastic_recover);
                next.foot_fr.position = Vec3::new(
                    s_a.feet_f.0 - (movement - elastic_recover) * 8.0,
                    s_a.feet_f.1 - (movement - elastic_recover) * 8.0,
                    s_a.feet_f.2 - out_and_in(buildup) * 15.0 + (movement - elastic_recover) * 8.0,
                );
                next.foot_bl.scale = Vec3::one() * (1.0 - movement + elastic_recover);
                next.foot_bl.position = Vec3::new(
                    -s_a.feet_b.0 + (movement - elastic_recover) * 8.0,
                    s_a.feet_b.1 + (movement - elastic_recover) * 8.0,
                    s_a.feet_b.2 - out_and_in(buildup) * 15.0 + (movement - elastic_recover) * 8.0,
                );
                next.foot_br.scale = Vec3::one() * (1.0 - movement + elastic_recover);
                next.foot_br.position = Vec3::new(
                    s_a.feet_b.0 - (movement - elastic_recover) * 8.0,
                    s_a.feet_b.1 + (movement - elastic_recover) * 8.0,
                    s_a.feet_b.2 - out_and_in(buildup) * 15.0 + (movement - elastic_recover) * 8.0,
                );
                next.chest.position = Vec3::new(
                    0.0,
                    0.0,
                    s_a.chest.1
                        + out_and_in(buildup) * 15.0
                        + ((action - 1.0).powi(2) - 1.0) * 15.0
                        + elastic_recover * 15.0,
                );
                next.chest.orientation = Quaternion::rotation_z(4.0 * PI * movement);
            },
            _ => {
                let (movement1base, movement2base, _movement3base, movement4) = match stage_section
                {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0, 0.0),
                    Some(StageSection::Movement) => (1.0, anim_time.powf(0.1), 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, 1.0, anim_time.powf(0.1), 0.0),
                    Some(StageSection::Recover) => (0.0, 1.0, 1.0, anim_time.powi(4)),
                    _ => (0.0, 0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - movement4;
                let movement1abs = movement1base * pullback;
                let movement2abs = movement2base * pullback;

                next.chest.scale = Vec3::one() * s_a.scaler;

                next.chest.position =
                    Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + movement1abs * -0.25);
                next.chest.orientation = Quaternion::rotation_x(movement2abs * 0.15)
                    * Quaternion::rotation_z((movement1abs * 4.0 * PI).sin() * 0.08);
            },
        }

        next
    }
}
