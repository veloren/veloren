use super::{
    super::{vek::*, Animation},
    QuadrupedSmallSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
//use std::ops::Rem;

pub struct StunnedAnimation;

impl Animation for StunnedAnimation {
    type Dependency<'a> = (f32, f32, Option<StageSection>, f32);
    type Skeleton = QuadrupedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_small_stunned\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_small_stunned")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, global_time, stage_section, timer): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (movement1base, movement2, twitch) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
            Some(StageSection::Recover) => {
                (1.0, anim_time.powf(3.0), ((1.0 - anim_time) * 10.0).sin())
            },
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - movement2;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let movement1 = mirror * movement1base * pullback;
        let movement1abs = movement1base * pullback;
        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + movement1abs * -1.5);
        next.head.orientation = Quaternion::rotation_x(movement1abs * -0.2)
            * Quaternion::rotation_y(movement1 * -0.6)
            * Quaternion::rotation_z(movement1 * 0.4 + twitch * 0.2 * mirror);

        next.chest.orientation =
            Quaternion::rotation_x(movement1abs * -0.2) * Quaternion::rotation_z(0.0);

        next.leg_fl.position = Vec3::new(-s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2);
        next.leg_fl.orientation = Quaternion::rotation_x(movement1abs * 0.8);

        next.leg_fr.position = Vec3::new(s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2);
        next.leg_fr.orientation = Quaternion::rotation_x(movement1abs * 0.8);

        next.leg_bl.position = Vec3::new(-s_a.feet_b.0, s_a.feet_b.1, s_a.feet_b.2);
        next.leg_bl.orientation = Quaternion::rotation_x(movement1abs * -0.2);

        next.leg_br.position = Vec3::new(s_a.feet_b.0, s_a.feet_b.1, s_a.feet_b.2);
        next.leg_br.orientation = Quaternion::rotation_x(movement1abs * -0.2);

        next.tail.orientation =
            Quaternion::rotation_x(movement1abs * 0.5) * Quaternion::rotation_z(movement1 * -0.4);

        next
    }
}
