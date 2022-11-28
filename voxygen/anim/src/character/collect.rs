use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
use std::f32::consts::PI;

pub struct CollectAnimation;

impl Animation for CollectAnimation {
    type Dependency<'a> = (Vec3<f32>, f32, Option<StageSection>, Vec3<f32>);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_collect\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_collect")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (position, _global_time, stage_section, sprite_pos): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (movement1, move2, move2alt, move3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0, 0.0),
            Some(StageSection::Action) => (
                1.0,
                (anim_time * 12.0).sin(),
                (anim_time * 9.0 + PI / 2.0).sin(),
                0.0,
            ),
            Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time.powi(4)),
            _ => (0.0, 0.0, 0.0, 0.0),
        };
        let z_diff = (sprite_pos.z - position.z).round();
        let z_diff = if z_diff > 0.0 { z_diff / 9.0 } else { 0.0 };
        let squat = (1.0 - z_diff).powf(4.0);

        let pullback = 1.0 - move3;

        let move1 = movement1 * pullback * squat;
        let move1_nosquat = movement1 * pullback;
        let upshift = if squat < 0.35 {
            move1_nosquat * 0.3
        } else {
            0.0
        };
        next.head.orientation = Quaternion::rotation_x(move1_nosquat * 0.2 + upshift * 1.3);

        next.chest.position = Vec3::new(
            0.0,
            s_a.chest.0 + upshift * 3.0,
            s_a.chest.1 + move2 * 0.15 + upshift * 3.0,
        );
        next.chest.orientation = Quaternion::rotation_x(move1 * -1.0 + move2alt * 0.015);

        next.belt.position = Vec3::new(0.0, s_a.belt.0 + move1 * 1.0, s_a.belt.1 + move1 * -0.0);
        next.belt.orientation = Quaternion::rotation_x(move1 * 0.2);

        next.back.position = Vec3::new(0.0, s_a.back.0, s_a.back.1);

        next.shorts.position =
            Vec3::new(0.0, s_a.shorts.0 + move1 * 2.0, s_a.shorts.1 + move1 * -0.0);
        next.shorts.orientation = Quaternion::rotation_x(move1 * 0.3);

        next.hand_l.position = Vec3::new(
            -s_a.hand.0 + move1_nosquat * 4.0 - move2alt * 1.0,
            s_a.hand.1 + move1_nosquat * 8.0 + move2 * 1.0 + upshift * -5.0,
            s_a.hand.2 + move1_nosquat * 5.0 + upshift * 15.0,
        );

        next.hand_l.orientation = Quaternion::rotation_x(move1_nosquat * 1.9 + upshift * 2.0)
            * Quaternion::rotation_y(move1_nosquat * -0.3 + move2alt * -0.2);

        next.hand_r.position = Vec3::new(
            s_a.hand.0 + move1_nosquat * -4.0 - move2 * 1.0,
            s_a.hand.1 + move1_nosquat * 8.0 + move2alt * -1.0 + upshift * -5.0,
            s_a.hand.2 + move1_nosquat * 5.0 + upshift * 15.0,
        );
        next.hand_r.orientation = Quaternion::rotation_x(move1_nosquat * 1.9 + upshift * 2.0)
            * Quaternion::rotation_y(move1_nosquat * 0.3 + move2 * 0.3);

        next.foot_l.position = Vec3::new(
            -s_a.foot.0,
            s_a.foot.1 + move1 * 2.0 + upshift * -3.5,
            s_a.foot.2 + upshift * 2.0,
        );
        next.foot_l.orientation = Quaternion::rotation_x(move1 * -0.2 + upshift * -2.2);

        next.foot_r.position = Vec3::new(
            s_a.foot.0,
            s_a.foot.1 + move1 * -4.0 + upshift * -0.5,
            s_a.foot.2 + upshift * 2.0,
        );
        next.foot_r.orientation = Quaternion::rotation_x(move1 * -0.8 + upshift * -1.2);
        next
    }
}
