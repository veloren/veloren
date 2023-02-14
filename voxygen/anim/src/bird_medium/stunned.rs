use super::{
    super::{vek::*, Animation},
    BirdMediumSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct StunnedAnimation;

impl Animation for StunnedAnimation {
    type Dependency<'a> = (f32, Option<StageSection>, f32);
    type Skeleton = BirdMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"bird_medium_stunned\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "bird_medium_stunned")]
    fn update_skeleton_inner<'a>(
        skeleton: &Self::Skeleton,
        (global_time, stage_section, timer): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let wave_slow_cos = (anim_time * 4.5).cos();

        let (movement1base, movement2, twitch) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.1), 0.0, anim_time),
            Some(StageSection::Recover) => (1.0, anim_time.powf(4.0), 1.0),
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - movement2;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let twitch2 = mirror * (twitch * 20.0).sin() * pullback;
        let movement1abs = movement1base * pullback;

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + wave_slow_cos * 0.06);
        next.chest.orientation = Quaternion::rotation_x(movement1base * 0.5);

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation =
            Quaternion::rotation_z(twitch2 * 0.8) * Quaternion::rotation_x(wave_slow_cos * 0.01);

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);

        next.wing_in_l.position = Vec3::new(-s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);
        next.wing_in_r.position = Vec3::new(s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);

        next.wing_in_l.orientation = Quaternion::rotation_y(wave_slow_cos * 0.06 + twitch2 * 0.8)
            * Quaternion::rotation_z(0.2 - movement1abs);
        next.wing_in_r.orientation = Quaternion::rotation_y(wave_slow_cos * 0.06 - twitch2 * 0.8)
            * Quaternion::rotation_z(-0.2 + movement1abs);

        next.wing_out_l.position = Vec3::new(-s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
        next.wing_out_r.position = Vec3::new(s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
        next.wing_out_l.orientation = Quaternion::rotation_y(-0.2) * Quaternion::rotation_z(0.2);
        next.wing_out_r.orientation = Quaternion::rotation_y(0.2) * Quaternion::rotation_z(-0.2);

        next.leg_l.position = Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2);
        next.leg_l.orientation = Quaternion::rotation_x(movement1abs * 0.8);
        next.leg_r.position = Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2);

        next
    }
}
