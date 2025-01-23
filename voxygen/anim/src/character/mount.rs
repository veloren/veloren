use super::{
    super::{Animation, vek::*},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use std::{f32::consts::PI, ops::Mul};

pub struct MountAnimation;

impl Animation for MountAnimation {
    type Dependency<'a> = (
        Option<ToolKind>,
        Option<ToolKind>,
        (Option<Hands>, Option<Hands>),
        f32,
        Vec3<f32>,
        Vec3<f32>,
        Vec3<f32>,
        Vec3<f32>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_mount\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_mount")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            active_tool_kind,
            second_tool_kind,
            hands,
            global_time,
            velocity,
            avg_vel,
            orientation,
            last_ori,
        ): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let head_look = Vec2::new(
            (global_time * 0.05 + anim_time / 15.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.25,
            (global_time * 0.05 + anim_time / 15.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.125,
        );

        let ori: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let speed = (Vec2::<f32>::from(velocity).magnitude()).min(24.0);
        let canceler = (speed / 24.0).powf(0.6);
        let _x_tilt = avg_vel.z.atan2(avg_vel.xy().magnitude()) * canceler;
        let tilt = if vek::Vec2::new(ori, last_ori)
            .map(|o| o.magnitude_squared())
            .map(|m| m > 0.001 && m.is_finite())
            .reduce_and()
            && ori.angle_between(last_ori).is_finite()
        {
            ori.angle_between(last_ori).min(0.2)
                * last_ori.determine_side(Vec2::zero(), ori).signum()
        } else {
            0.0
        } * 1.3;

        let bob = (anim_time * 12.0).sin();

        next.head.scale = Vec3::one() * s_a.head_scale;
        next.chest.scale = Vec3::one() * 1.01;
        next.hand_l.scale = Vec3::one() * 1.04;
        next.hand_r.scale = Vec3::one() * 1.04;
        next.back.scale = Vec3::one() * 1.02;
        next.belt.scale = Vec3::one() * 1.02;
        next.hold.scale = Vec3::one() * 0.0;
        next.lantern.scale = Vec3::one() * 0.65;
        next.shoulder_l.scale = Vec3::one() * 1.1;
        next.shoulder_r.scale = Vec3::one() * 1.1;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_z(head_look.x + tilt * -2.0)
            * Quaternion::rotation_x((0.35 + head_look.y + tilt.abs() * 1.2).abs());

        // Don't offset the chest when mounting, we don't want the position while
        // mounting to change with scale.
        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 * 0.5);
        next.chest.orientation =
            Quaternion::rotation_x(-0.4 + tilt.abs() * -1.5 - bob * speed * 0.0)
                * Quaternion::rotation_y(tilt * 2.0);

        next.belt.position = Vec3::new(0.0, s_a.belt.0 + 0.5, s_a.belt.1 + 0.5);
        next.belt.orientation = Quaternion::rotation_x(0.2) * Quaternion::rotation_y(tilt * -0.5);

        next.back.position = Vec3::new(0.0, s_a.back.0, s_a.back.1);

        next.shorts.position = Vec3::new(0.0, s_a.shorts.0 + 1.0, s_a.shorts.1 + 1.0);
        next.shorts.orientation = Quaternion::rotation_x(0.3) * Quaternion::rotation_y(tilt * -1.0);

        next.hand_l.position = Vec3::new(
            -s_a.hand.0 + 3.0,
            s_a.hand.1 + 6.0,
            s_a.hand.2 + 2.0 + (bob + 1.0) * speed * 0.1,
        );
        next.hand_l.orientation =
            Quaternion::rotation_x(PI * 0.4) * Quaternion::rotation_z(-PI / 2.0 + 1.0);

        next.hand_r.position = Vec3::new(
            s_a.hand.0 - 3.0,
            s_a.hand.1 + 6.0,
            s_a.hand.2 + 2.0 + (bob + 1.0) * speed * 0.1,
        );
        next.hand_r.orientation =
            Quaternion::rotation_x(PI * 0.4) * Quaternion::rotation_z(PI / 2.0 - 1.0);

        next.foot_l.position = Vec3::new(
            -s_a.foot.0 - 2.0,
            4.0 + s_a.foot.1,
            s_a.foot.2 - s_a.chest.1 * 0.5,
        );
        next.foot_l.orientation = Quaternion::rotation_x(0.5) * Quaternion::rotation_y(0.5);

        next.foot_r.position = Vec3::new(
            s_a.foot.0 + 2.0,
            4.0 + s_a.foot.1,
            s_a.foot.2 - s_a.chest.1 * 0.5,
        );
        next.foot_r.orientation = Quaternion::rotation_x(0.5) * Quaternion::rotation_y(-0.5);

        next.shoulder_l.position = Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_l.orientation = Quaternion::rotation_x(0.0);

        next.shoulder_r.position = Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_r.orientation = Quaternion::rotation_x(0.0);

        next.do_hold_lantern(s_a, anim_time, 0.0, speed * 0.1 + 0.1, 0.0, tilt);

        next.glider.position = Vec3::new(0.0, 0.0, 10.0);
        next.glider.scale = Vec3::one() * 0.0;
        next.hold.position = Vec3::new(0.4, -0.3, -5.8);

        next.do_tools_on_back(hands, active_tool_kind, second_tool_kind);

        next
    }
}
