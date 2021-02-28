use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};
use std::f32::consts::PI;

pub struct RollAnimation;

type RollAnimationDependency = (
    Option<ToolKind>,
    Option<ToolKind>,
    Vec3<f32>,
    Vec3<f32>,
    f32,
    Option<StageSection>,
);

impl Animation for RollAnimation {
    type Dependency = RollAnimationDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_roll\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_roll")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, _second_tool_kind, orientation, last_ori, _global_time, stage_section): Self::Dependency,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let ori: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if ::vek::Vec2::new(ori, last_ori)
            .map(|o| o.magnitude_squared())
            .map(|m| m > 0.0001 && m.is_finite())
            .reduce_and()
            && ori.angle_between(last_ori).is_finite()
        {
            ori.angle_between(last_ori).min(0.05)
                * last_ori.determine_side(Vec2::zero(), ori).signum()
        } else {
            0.0
        };

        let (movement1base, movement2, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(2.0), 0.0, 0.0),
            Some(StageSection::Movement) => (1.0, anim_time.powf(0.75), 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powf(0.75)),
            _ => (0.0, 0.0, 0.0),
        };
        let movement1 = movement1base * (1.0 - movement3);
        next.head.position = Vec3::new(
            0.0,
            s_a.head.0 + 3.0 * movement1,
            s_a.head.1 - 1.0 * movement1,
        );
        next.head.orientation = Quaternion::rotation_x(-0.75 * movement1base + 0.75 * movement2);

        next.chest.position = Vec3::new(0.0, s_a.chest.0, -9.5 * movement1 + s_a.chest.1);
        next.chest.orientation = Quaternion::rotation_x(-0.2 * movement1);

        next.belt.position = Vec3::new(
            0.0,
            s_a.belt.0 + 1.0 * movement1,
            s_a.belt.1 + 1.0 * movement1,
        );
        next.belt.orientation = Quaternion::rotation_x(0.55 * movement1);

        next.shorts.position = Vec3::new(
            0.0,
            s_a.shorts.0 + 4.5 * movement1,
            s_a.shorts.1 + 2.5 * movement1,
        );
        next.shorts.orientation = Quaternion::rotation_x(0.8 * movement1);

        next.hand_l.position = Vec3::new(
            -s_a.hand.0,
            s_a.hand.1 + 1.0 * movement1,
            s_a.hand.2 + 2.0 * movement1,
        );

        next.hand_l.orientation = Quaternion::rotation_x(0.6 * movement1);

        next.hand_r.position = Vec3::new(
            -1.0 * movement1 + s_a.hand.0,
            s_a.hand.1 + 1.0 * movement1,
            s_a.hand.2 + 2.0 * movement1,
        );
        next.hand_r.orientation = Quaternion::rotation_x(0.6 * movement1);

        next.foot_l.position = Vec3::new(
            1.0 * movement1 - s_a.foot.0,
            s_a.foot.1 + 5.5 * movement1,
            s_a.foot.2 - 5.0 * movement1,
        );
        next.foot_l.orientation = Quaternion::rotation_x(0.9 * movement1);

        next.foot_r.position = Vec3::new(
            1.0 * movement1 + s_a.foot.0,
            s_a.foot.1 + 5.5 * movement1,
            s_a.foot.2 - 5.0 * movement1,
        );
        next.foot_r.orientation = Quaternion::rotation_x(0.9 * movement1);

        next.torso.position = Vec3::new(0.0, 0.0, 8.0 * movement1) / 11.0 * s_a.scaler;
        next.torso.orientation = Quaternion::rotation_x(movement1 * -0.4 + movement2 * -2.0 * PI)
            * Quaternion::rotation_z(tilt * -10.0);

        next
    }
}
