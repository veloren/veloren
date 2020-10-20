use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use std::{f32::consts::PI, ops::Mul};

pub struct ClimbAnimation;

impl Animation for ClimbAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        Vec3<f32>,
        Vec3<f32>,
        f64,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_climb\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_climb")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, velocity, _orientation, global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let lateral = Vec2::<f32>::from(velocity).magnitude();
        let speed = velocity.z;
        *rate = speed;
        let constant = 1.0;
        let smooth = (anim_time as f32 * constant as f32 * 1.5).sin();
        let smootha = (anim_time as f32 * constant as f32 * 1.5 + PI / 2.0).sin();
        let drop = (anim_time as f32 * constant as f32 * 4.0 + PI / 2.0).sin();
        let dropa = (anim_time as f32 * constant as f32 * 4.0).sin();

        let quick = (((5.0)
            / (0.6 + 4.0 * ((anim_time as f32 * constant as f32 * 1.5).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * constant as f32 * 1.5).sin());
        let quicka = (((5.0)
            / (0.6
                + 4.0
                    * ((anim_time as f32 * constant as f32 * 1.5 + PI / 2.0).sin())
                        .powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * constant as f32 * 1.5 + PI / 2.0).sin());
        let head_look = Vec2::new(
            ((global_time + anim_time) as f32 / 2.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.3,
            ((global_time + anim_time) as f32 / 2.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.15,
        );
        let stagnant = if speed > -0.7 { 1.0 } else { 0.0 }; //sets static position when there is no movement

        if speed > 0.7 || lateral > 0.1 {
            next.head.position = Vec3::new(
                0.0,
                -4.0 + skeleton_attr.head.0,
                skeleton_attr.head.1 + smootha * 0.2,
            );
            next.head.orientation = Quaternion::rotation_z(smooth * 0.1)
                * Quaternion::rotation_x(0.6)
                * Quaternion::rotation_y(quick * 0.1);

            next.chest.position = Vec3::new(
                0.0,
                skeleton_attr.chest.0,
                skeleton_attr.chest.1 + smootha * 1.1,
            );
            next.chest.orientation = Quaternion::rotation_z(quick * 0.25)
                * Quaternion::rotation_x(-0.15)
                * Quaternion::rotation_y(quick * -0.12);

            next.belt.position = Vec3::new(0.0, skeleton_attr.belt.0 + 1.0, skeleton_attr.belt.1);

            next.back.orientation = Quaternion::rotation_x(-0.2);

            next.shorts.position =
                Vec3::new(0.0, skeleton_attr.shorts.0 + 1.0, skeleton_attr.shorts.1);
            next.shorts.orientation = Quaternion::rotation_z(quick * 0.0)
                * Quaternion::rotation_x(0.1)
                * Quaternion::rotation_y(quick * 0.10);

            next.hand_l.position = Vec3::new(
                -skeleton_attr.hand.0,
                4.0 + skeleton_attr.hand.1 + quicka * 1.5,
                5.0 + skeleton_attr.hand.2 - quick * 4.0,
            );
            next.hand_l.orientation = Quaternion::rotation_x(2.2 + quicka * 0.5);

            next.hand_r.position = Vec3::new(
                skeleton_attr.hand.0,
                5.0 + skeleton_attr.hand.1 - quicka * 1.5,
                5.0 + skeleton_attr.hand.2 + quick * 4.0,
            );
            next.hand_r.orientation = Quaternion::rotation_x(2.2 - quicka * 0.5);

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
                    next.main.orientation =
                        Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
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
            next.foot_l.position = Vec3::new(
                -skeleton_attr.foot.0,
                5.0 + skeleton_attr.foot.1,
                skeleton_attr.foot.2 + quick * 2.5,
            );
            next.foot_l.orientation = Quaternion::rotation_x(0.2 - quicka * 0.5);

            next.foot_r.position = Vec3::new(
                skeleton_attr.foot.0,
                4.0 + skeleton_attr.foot.1,
                skeleton_attr.foot.2 - quick * 2.5,
            );
            next.foot_r.orientation = Quaternion::rotation_x(0.2 + quicka * 0.5);

            next.shoulder_l.orientation = Quaternion::rotation_x(smootha * 0.15);

            next.shoulder_r.orientation = Quaternion::rotation_x(smooth * 0.15);

            next.glider.position = Vec3::new(0.0, 0.0, 10.0);
            next.glider.scale = Vec3::one() * 0.0;

            next.main.position = Vec3::new(-7.0, -5.0, 18.0);
            next.main.orientation =
                Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57 + smootha * 0.25);

            next.second.position = Vec3::new(0.0, 0.0, 0.0);
            next.second.orientation = Quaternion::rotation_y(0.0);

            next.lantern.orientation =
                Quaternion::rotation_x(smooth * -0.3) * Quaternion::rotation_y(smooth * -0.3);

            next.torso.position = Vec3::new(0.0, -0.2 + smooth * -0.08, 0.4) * skeleton_attr.scaler;
        } else {
            next.head.position = Vec3::new(
                0.0,
                -1.0 - stagnant + skeleton_attr.head.0,
                skeleton_attr.head.1,
            );
            next.head.orientation = Quaternion::rotation_x(
                -0.25 * (1.0 - stagnant) + stagnant * 2.0 * head_look.x.abs(),
            ) * Quaternion::rotation_z(stagnant * 3.5 * head_look.x.abs());

            next.chest.position =
                Vec3::new(0.0, 1.0 + skeleton_attr.chest.0, skeleton_attr.chest.1);
            next.chest.orientation = Quaternion::rotation_z(0.6 * stagnant)
                * Quaternion::rotation_x((0.2 + drop * 0.05) * (1.0 - stagnant));

            next.belt.position = Vec3::new(0.0, skeleton_attr.belt.0 + 0.5, skeleton_attr.belt.1);
            next.belt.orientation = Quaternion::rotation_x(0.1 + dropa * 0.1);

            next.back.orientation = Quaternion::rotation_x(
                -0.2 + dropa * 0.1 - 0.15 * (1.0 - stagnant) + stagnant * 0.1,
            );

            next.shorts.position =
                Vec3::new(0.0, skeleton_attr.shorts.0 + 1.0, skeleton_attr.shorts.1);
            next.shorts.orientation = Quaternion::rotation_x(0.1 + dropa * 0.12 * (1.0 - stagnant));

            next.hand_l.position = Vec3::new(
                -skeleton_attr.hand.0,
                7.5 + stagnant * -5.0 + skeleton_attr.hand.1,
                7.0 + stagnant * -7.0 + skeleton_attr.hand.2 + dropa * -1.0 * (1.0 - stagnant),
            );
            next.hand_l.orientation = Quaternion::rotation_x(2.2 + stagnant * -1.4)
                * Quaternion::rotation_y((0.3 + dropa * 0.1) * (1.0 - stagnant));

            next.hand_r.position = Vec3::new(
                skeleton_attr.hand.0,
                7.5 + stagnant * -2.5 + skeleton_attr.hand.1,
                5.0 + skeleton_attr.hand.2 + drop * -1.0 * (1.0 - stagnant),
            );
            next.hand_r.orientation = Quaternion::rotation_x(2.2)
                * Quaternion::rotation_y(-0.3 + drop * 0.1 * (1.0 - stagnant));

            next.foot_l.position = Vec3::new(
                -skeleton_attr.foot.0,
                4.0 + stagnant * 3.0 + skeleton_attr.foot.1,
                1.0 + skeleton_attr.foot.2 + drop * -2.0 * (1.0 - stagnant),
            );
            next.foot_l.orientation = Quaternion::rotation_x(0.55 + drop * 0.1 * (1.0 - stagnant));

            next.foot_r.position = Vec3::new(
                skeleton_attr.foot.0,
                2.0 + stagnant * 4.0 + skeleton_attr.foot.1,
                -2.0 + skeleton_attr.foot.2 + smooth * 1.0 * (1.0 - stagnant),
            );
            next.foot_r.orientation =
                Quaternion::rotation_x(0.2 + smooth * 0.15 * (1.0 - stagnant));

            next.glider.position = Vec3::new(0.0, 0.0, 10.0);
            next.glider.scale = Vec3::one() * 0.0;

            next.main.position = Vec3::new(-7.0, -5.0, 18.0);
            next.main.orientation = Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);

            next.torso.position = Vec3::new(0.0, -0.2, 0.4) * skeleton_attr.scaler;
        };

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
