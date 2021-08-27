use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
use std::f32::consts::PI;

pub struct CollectAnimation;

impl Animation for CollectAnimation {
    #[allow(clippy::type_complexity)]
    type Dependency<'a> = (Vec3<f32>, f32, Option<StageSection>, Vec3<i32>);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_collect\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_collect")]
    #[allow(clippy::single_match)] // TODO: Pending review in #587
    fn update_skeleton_inner<'a>(
        skeleton: &Self::Skeleton,
        (position, _global_time, stage_section, sprite_pos): Self::Dependency<'a>,
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
        let pullback = 1.0 - move3;

        let move1 = movement1 * pullback;

        println!("{} pos z", position.z);
        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_x(move1 * 0.2);

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + move2 * 0.15);
        next.chest.orientation = Quaternion::rotation_x(move1 * -1.0 + move2alt * 0.015);

        next.belt.position = Vec3::new(0.0, s_a.belt.0 + move1 * 1.0, s_a.belt.1 + move1 * -0.0);
        next.belt.orientation = Quaternion::rotation_x(move1 * 0.2);

        next.back.position = Vec3::new(0.0, s_a.back.0, s_a.back.1);

        next.shorts.position =
            Vec3::new(0.0, s_a.shorts.0 + move1 * 2.0, s_a.shorts.1 + move1 * -0.0);
        next.shorts.orientation = Quaternion::rotation_x(move1 * 0.3);

        next.hand_l.position = Vec3::new(
            -s_a.hand.0 + move1 * 4.0 - move2alt * 1.0,
            s_a.hand.1 + move1 * 8.0 + move2 * 1.0,
            s_a.hand.2 + move1 * 5.0,
        );

        next.hand_l.orientation = Quaternion::rotation_x(move1 * 1.9)
            * Quaternion::rotation_y(move1 * -0.3 + move2alt * -0.2);

        next.hand_r.position = Vec3::new(
            s_a.hand.0 + move1 * -4.0 - move2 * 1.0,
            s_a.hand.1 + move1 * 8.0 + move2alt * -1.0,
            s_a.hand.2 + move1 * 5.0,
        );
        next.hand_r.orientation =
            Quaternion::rotation_x(move1 * 1.9) * Quaternion::rotation_y(move1 * 0.3 + move2 * 0.3);

        next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1 + move1 * 2.0, s_a.foot.2);
        next.foot_l.orientation = Quaternion::rotation_x(move1 * -0.2);

        next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1 + move1 * -4.0, s_a.foot.2);
        next.foot_r.orientation = Quaternion::rotation_x(move1 * -0.8);

        next.shoulder_l.position = Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);

        next.shoulder_r.position = Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);

        next
    }
}
