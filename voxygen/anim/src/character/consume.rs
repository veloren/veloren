use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::ConsumableKind,
    states::{use_item::ItemUseKind, utils::StageSection},
};

pub struct ConsumeAnimation;

impl Animation for ConsumeAnimation {
    type Dependency<'a> = (f32, Option<StageSection>, Option<ItemUseKind>);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_consume\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_consume")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_global_time, stage_section, item_kind): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        match item_kind {
            Some(ItemUseKind::Consumable(ConsumableKind::Drink)) => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, (anim_time * 8.0).sin(), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powf(0.25)),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move2 = move2 * pullback;
                let move1 = move1 * pullback;
                next.head.orientation = Quaternion::rotation_x(move1 * 0.5 + move2 * -0.05);

                next.hand_r.position = Vec3::new(
                    s_a.hand.0 + move1 * -4.0,
                    s_a.hand.1 + move1 * 6.0,
                    s_a.hand.2 + move1 * 10.0 + move2 * -1.0,
                );
                next.hand_r.orientation = Quaternion::rotation_x(move1 * 2.3 + move2 * -0.2)
                    * Quaternion::rotation_y(move1 * 1.2);
                next.chest.orientation = Quaternion::rotation_x(move1 * 0.25);
                next.hand_l.position = Vec3::new(
                    -s_a.hand.0 + move1 * 3.0,
                    s_a.hand.1 + move1 * 2.0,
                    s_a.hand.2,
                );

                next.hand_l.orientation =
                    Quaternion::rotation_x(move1 * 0.8) * Quaternion::rotation_y(move1 * -0.5);
            },
            Some(ItemUseKind::Consumable(ConsumableKind::Food | ConsumableKind::ComplexFood)) => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, (anim_time * 12.0).sin(), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powf(0.25)),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move2 = move2 * pullback;
                let move1 = move1 * pullback;
                next.head.position =
                    Vec3::new(0.0, s_a.head.0 + move1 * 2.0, s_a.head.1 + move1 * 1.0);
                next.head.orientation =
                    Quaternion::rotation_x(move1 * -0.3) * Quaternion::rotation_z(move2 * -0.15);

                next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + move1 * -3.0);
                next.chest.orientation =
                    Quaternion::rotation_x(move1 * 0.3) * Quaternion::rotation_z(move2 * 0.05);

                next.belt.position = Vec3::new(0.0, s_a.belt.0 + move1 * 1.0, s_a.belt.1);
                next.belt.orientation = Quaternion::rotation_x(move1 * 0.2);

                next.back.position = Vec3::new(0.0, s_a.back.0, s_a.back.1);

                next.shorts.position = Vec3::new(0.0, s_a.shorts.0 + move1 * 3.0, s_a.shorts.1);
                next.shorts.orientation = Quaternion::rotation_x(move1 * 0.7);

                next.hand_l.position = Vec3::new(
                    -s_a.hand.0 + move1 * 3.0 + move2 * -1.0,
                    s_a.hand.1 + move1 * 5.0,
                    s_a.hand.2 + move1 * 3.0 + move2 * -2.0,
                );

                next.hand_l.orientation = Quaternion::rotation_x(move1 * 1.2)
                    * Quaternion::rotation_y(move1 * -0.5 + move2 * 0.3);

                next.hand_r.position = Vec3::new(
                    s_a.hand.0 + move1 * -3.0 + move2 * -1.0,
                    s_a.hand.1 + move1 * 5.0,
                    s_a.hand.2 + move1 * 3.0 + move2 * 2.0,
                );
                next.hand_r.orientation = Quaternion::rotation_x(move1 * 1.2)
                    * Quaternion::rotation_y(move1 * 0.5 + move2 * 0.3);

                next.foot_l.position = Vec3::new(
                    -s_a.foot.0,
                    s_a.foot.1 + move1 * 5.0,
                    s_a.foot.2 + move1 * 2.0,
                );
                next.foot_l.orientation = Quaternion::rotation_x(move1 * 1.2);

                next.foot_r.position = Vec3::new(
                    s_a.foot.0,
                    s_a.foot.1 + move1 * 5.0,
                    s_a.foot.2 + move1 * 2.0,
                );
                next.foot_r.orientation = Quaternion::rotation_x(move1 * 1.2);

                next.shoulder_l.position =
                    Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);

                next.shoulder_r.position =
                    Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
            },
            _ => {},
        }

        next
    }
}
