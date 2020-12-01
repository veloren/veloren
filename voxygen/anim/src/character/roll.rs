use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::ToolKind;

pub struct RollAnimation;

impl Animation for RollAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        Vec3<f32>,
        Vec3<f32>,
        f64,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_roll\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_roll")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, _second_tool_kind, orientation, last_ori, _global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let spin = anim_time as f32 * 1.1;

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

        next.head.position = Vec3::new(0.0, s_a.head.0 + 3.0, s_a.head.1 - 1.0);
        next.head.orientation = Quaternion::rotation_x(-0.75);

        next.chest.position = Vec3::new(0.0, s_a.chest.0, -9.5 + s_a.chest.1);
        next.chest.orientation = Quaternion::rotation_x(-0.2);

        next.belt.position = Vec3::new(0.0, s_a.belt.0 + 1.0, s_a.belt.1 + 1.0);
        next.belt.orientation = Quaternion::rotation_x(0.55);

        next.back.position = Vec3::new(0.0, s_a.back.0, s_a.back.1);

        next.shorts.position = Vec3::new(0.0, s_a.shorts.0 + 4.5, s_a.shorts.1 + 2.5);
        next.shorts.orientation = Quaternion::rotation_x(0.8);

        next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1 + 1.0, s_a.hand.2 + 2.0);

        next.hand_l.orientation = Quaternion::rotation_x(0.6);

        next.hand_r.position = Vec3::new(-1.0 + s_a.hand.0, s_a.hand.1 + 1.0, s_a.hand.2 + 2.0);
        next.hand_r.orientation = Quaternion::rotation_x(0.6);

        next.foot_l.position = Vec3::new(1.0 - s_a.foot.0, s_a.foot.1 + 5.5, s_a.foot.2 - 5.0);
        next.foot_l.orientation = Quaternion::rotation_x(0.9);

        next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1 + 5.5, s_a.foot.2 - 5.0);
        next.foot_r.orientation = Quaternion::rotation_x(0.9);

        next.torso.position = Vec3::new(0.0, 0.0, 8.0) / 11.0 * s_a.scaler;
        next.torso.orientation =
            Quaternion::rotation_x(spin * -10.0) * Quaternion::rotation_z(tilt * -10.0);

        next
    }
}
