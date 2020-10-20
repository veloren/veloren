use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use std::f32::consts::PI;

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
        (active_tool_kind, second_tool_kind, orientation, last_ori, _global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let spin = anim_time as f32;
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

        next.head.position = Vec3::new(0.0, skeleton_attr.head.0 + 3.0, skeleton_attr.head.1 - 1.0);
        next.head.orientation = Quaternion::rotation_x(-0.75);
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.position = Vec3::new(0.0, skeleton_attr.chest.0, -9.5 + skeleton_attr.chest.1);
        next.chest.orientation = Quaternion::rotation_x(-0.2);

        next.belt.position = Vec3::new(0.0, skeleton_attr.belt.0 + 1.0, skeleton_attr.belt.1 + 1.0);
        next.belt.orientation = Quaternion::rotation_x(0.55);

        next.back.position = Vec3::new(0.0, skeleton_attr.back.0, skeleton_attr.back.1);

        next.shorts.position = Vec3::new(
            0.0,
            skeleton_attr.shorts.0 + 4.5,
            skeleton_attr.shorts.1 + 2.5,
        );
        next.shorts.orientation = Quaternion::rotation_x(0.8);

        next.hand_l.position = Vec3::new(
            -skeleton_attr.hand.0,
            skeleton_attr.hand.1 + 1.0,
            skeleton_attr.hand.2 + 2.0,
        );

        next.hand_l.orientation = Quaternion::rotation_x(0.6);

        next.hand_r.position = Vec3::new(
            -1.0 + skeleton_attr.hand.0,
            skeleton_attr.hand.1 + 1.0,
            skeleton_attr.hand.2 + 2.0,
        );
        next.hand_r.orientation = Quaternion::rotation_x(0.6);

        next.foot_l.position = Vec3::new(
            1.0 - skeleton_attr.foot.0,
            skeleton_attr.foot.1 + 5.5,
            skeleton_attr.foot.2 - 5.0,
        );
        next.foot_l.orientation = Quaternion::rotation_x(0.9);

        next.foot_r.position = Vec3::new(
            skeleton_attr.foot.0,
            skeleton_attr.foot.1 + 5.5,
            skeleton_attr.foot.2 - 5.0,
        );
        next.foot_r.orientation = Quaternion::rotation_x(0.9);

        next.glider.position = Vec3::new(0.0, 0.0, 10.0);

        match active_tool_kind {
            Some(ToolKind::Dagger(_)) => {
                next.main.position = Vec3::new(-4.0, -5.0, 7.0);
                next.main.orientation =
                    Quaternion::rotation_y(0.25 * PI) * Quaternion::rotation_z(1.5 * PI);
            },
            Some(ToolKind::Shield(_)) => {
                next.main.position = Vec3::new(-0.0, -5.0, 3.0);
                next.main.orientation =
                    Quaternion::rotation_y(0.25 * PI) * Quaternion::rotation_z(-1.5 * PI);
            },
            _ => {
                next.main.position = Vec3::new(-7.0, -5.0, 15.0);
                next.main.orientation = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
            },
        }

        match second_tool_kind {
            Some(ToolKind::Dagger(_)) => {
                next.second.position = Vec3::new(4.0, -6.0, 7.0);
                next.second.orientation =
                    Quaternion::rotation_y(-0.25 * PI) * Quaternion::rotation_z(-1.5 * PI);
            },
            Some(ToolKind::Shield(_)) => {
                next.second.position = Vec3::new(0.0, -4.0, 3.0);
                next.second.orientation =
                    Quaternion::rotation_y(-0.25 * PI) * Quaternion::rotation_z(1.5 * PI);
            },
            _ => {
                next.second.position = Vec3::new(-7.0, -5.0, 15.0);
                next.second.orientation =
                    Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
            },
        }

        next.lantern.orientation = Quaternion::rotation_x(0.1) * Quaternion::rotation_y(0.1);
        next.lantern.scale = Vec3::one() * 0.65;
        next.hold.scale = Vec3::one() * 0.0;

        next.torso.position = Vec3::new(0.0, 0.0, 8.0) / 11.0 * skeleton_attr.scaler;
        next.torso.orientation =
            Quaternion::rotation_x(spin * -10.0) * Quaternion::rotation_z(tilt * -10.0);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next.second.scale = match (
            active_tool_kind.map(|tk| tk.hands()),
            second_tool_kind.map(|tk| tk.hands()),
        ) {
            (Some(Hands::OneHand), Some(Hands::OneHand)) => Vec3::one(),
            (_, _) => Vec3::zero(),
        };

        next
    }
}
