use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::ToolKind;
use std::{f32::consts::PI, ops::Mul};

pub struct ClimbAnimation;

impl Animation for ClimbAnimation {
    type Dependency<'a> = (
        Option<ToolKind>,
        Option<ToolKind>,
        Vec3<f32>,
        Vec3<f32>,
        f32,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_climb\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_climb")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, _second_tool_kind, velocity, _orientation, global_time): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let lateral = Vec2::<f32>::from(velocity).magnitude();
        let speed = velocity.z;
        *rate = speed;
        let constant: f32 = 1.0;
        let smooth = (anim_time * constant * 1.5).sin();
        let smootha = (anim_time * constant * 1.5 + PI / 2.0).sin();
        let drop = (anim_time * constant * 4.0 + PI / 2.0).sin();
        let dropa = (anim_time * constant * 4.0).sin();

        let quick = ((5.0 / (0.6 + 4.0 * ((anim_time * constant * 1.5).sin()).powi(2))).sqrt())
            * ((anim_time * constant * 1.5).sin());
        let quicka =
            ((5.0 / (0.6 + 4.0 * ((anim_time * constant * 1.5 + PI / 2.0).sin()).powi(2))).sqrt())
                * ((anim_time * constant * 1.5 + PI / 2.0).sin());
        let head_look = Vec2::new(
            (global_time / 2.0 + anim_time / 2.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.3,
            (global_time / 2.0 + anim_time / 2.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.15,
        );
        let stagnant = if speed > -0.7 { 1.0 } else { 0.0 }; //sets static position when there is no movement

        if speed > 0.7 || lateral > 0.1 {
            next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1 + smootha * 0.2);
            next.head.orientation = Quaternion::rotation_z(smooth * 0.1)
                * Quaternion::rotation_x(0.6)
                * Quaternion::rotation_y(quick * 0.1);

            next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + smootha * 1.1);
            next.chest.orientation = Quaternion::rotation_z(quick * 0.25)
                * Quaternion::rotation_x(-0.15)
                * Quaternion::rotation_y(quick * -0.12);

            next.belt.position = Vec3::new(0.0, s_a.belt.0 + 1.0, s_a.belt.1);

            next.back.orientation = Quaternion::rotation_x(-0.2);

            next.shorts.position = Vec3::new(0.0, s_a.shorts.0 + 1.0, s_a.shorts.1);
            next.shorts.orientation = Quaternion::rotation_z(quick * 0.0)
                * Quaternion::rotation_x(0.1)
                * Quaternion::rotation_y(quick * 0.10);

            next.hand_l.position = Vec3::new(
                -s_a.hand.0,
                4.0 + s_a.hand.1 + quicka * 1.5,
                5.0 + s_a.hand.2 - quick * 4.0,
            );
            next.hand_l.orientation = Quaternion::rotation_x(2.2 + quicka * 0.5);

            next.hand_r.position = Vec3::new(
                s_a.hand.0,
                5.0 + s_a.hand.1 - quicka * 1.5,
                5.0 + s_a.hand.2 + quick * 4.0,
            );
            next.hand_r.orientation = Quaternion::rotation_x(2.2 - quicka * 0.5);

            next.foot_l.position =
                Vec3::new(-s_a.foot.0, 5.0 + s_a.foot.1, s_a.foot.2 + quick * 2.5);
            next.foot_l.orientation = Quaternion::rotation_x(0.2 - quicka * 0.5);

            next.foot_r.position =
                Vec3::new(s_a.foot.0, 4.0 + s_a.foot.1, s_a.foot.2 - quick * 2.5);
            next.foot_r.orientation = Quaternion::rotation_x(0.2 + quicka * 0.5);

            next.shoulder_l.orientation = Quaternion::rotation_x(smootha * 0.15);

            next.shoulder_r.orientation = Quaternion::rotation_x(smooth * 0.15);

            next.lantern.orientation =
                Quaternion::rotation_x(smooth * -0.3) * Quaternion::rotation_y(smooth * -0.3);

            next.torso.position = Vec3::new(0.0, -2.2 + smooth * -0.88, 4.4);
        } else {
            next.head.position = Vec3::new(0.0, -1.0 - stagnant + s_a.head.0, s_a.head.1);
            next.head.orientation = Quaternion::rotation_x(
                -0.25 * (1.0 - stagnant) + stagnant * 2.0 * head_look.x.abs(),
            ) * Quaternion::rotation_z(stagnant * 3.5 * head_look.x.abs());

            next.chest.position = Vec3::new(0.0, 1.0 + s_a.chest.0, s_a.chest.1);
            next.chest.orientation = Quaternion::rotation_z(0.6 * stagnant)
                * Quaternion::rotation_x((0.2 + drop * 0.05) * (1.0 - stagnant));

            next.belt.position = Vec3::new(0.0, s_a.belt.0 + 0.5, s_a.belt.1);
            next.belt.orientation = Quaternion::rotation_x(0.1 + dropa * 0.1);

            next.back.orientation = Quaternion::rotation_x(
                -0.2 + dropa * 0.1 - 0.15 * (1.0 - stagnant) + stagnant * 0.1,
            );

            next.shorts.position = Vec3::new(0.0, s_a.shorts.0 + 1.0, s_a.shorts.1);
            next.shorts.orientation = Quaternion::rotation_x(0.1 + dropa * 0.12 * (1.0 - stagnant));

            next.hand_l.position = Vec3::new(
                -s_a.hand.0,
                7.5 + stagnant * -5.0 + s_a.hand.1,
                7.0 + stagnant * -7.0 + s_a.hand.2 + dropa * -1.0 * (1.0 - stagnant),
            );
            next.hand_l.orientation = Quaternion::rotation_x(2.2 + stagnant * -1.4)
                * Quaternion::rotation_y((0.3 + dropa * 0.1) * (1.0 - stagnant));

            next.hand_r.position = Vec3::new(
                s_a.hand.0,
                7.5 + stagnant * -2.5 + s_a.hand.1,
                5.0 + s_a.hand.2 + drop * -1.0 * (1.0 - stagnant),
            );
            next.hand_r.orientation = Quaternion::rotation_x(2.2)
                * Quaternion::rotation_y(-0.3 + drop * 0.1 * (1.0 - stagnant));

            next.foot_l.position = Vec3::new(
                -s_a.foot.0,
                4.0 + stagnant * 3.0 + s_a.foot.1,
                1.0 + s_a.foot.2 + drop * -2.0 * (1.0 - stagnant),
            );
            next.foot_l.orientation = Quaternion::rotation_x(0.55 + drop * 0.1 * (1.0 - stagnant));

            next.foot_r.position = Vec3::new(
                s_a.foot.0,
                2.0 + stagnant * 4.0 + s_a.foot.1,
                -2.0 + s_a.foot.2 + smooth * 1.0 * (1.0 - stagnant),
            );
            next.foot_r.orientation =
                Quaternion::rotation_x(0.2 + smooth * 0.15 * (1.0 - stagnant));

            next.torso.position = Vec3::new(0.0, -2.2, 4.4);
        };

        next
    }
}
