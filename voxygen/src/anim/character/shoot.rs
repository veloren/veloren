use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::ToolKind;
use vek::*;

pub struct ShootAnimation;

impl Animation for ShootAnimation {
    type Dependency = (Option<ToolKind>, f32, f64);
    type Skeleton = CharacterSkeleton;

    fn update_skeleton(
        skeleton: &Self::Skeleton,
        (active_tool_kind, velocity, _global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;

        let mut next = (*skeleton).clone();

        let lab = 1.0;

        let foot = (((5.0)
            / (0.2 + 4.8 * ((anim_time as f32 * lab as f32 * 8.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 8.0).sin());
        let foote = (((5.0)
            / (0.5 + 4.5 * ((anim_time as f32 * lab as f32 * 8.0 + 1.57).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 8.0).sin());

        let exp = ((anim_time as f32).powf(0.3 as f32)).min(1.2);

        next.head.offset = Vec3::new(0.0, -2.0 + skeleton_attr.head.0, skeleton_attr.head.1);
        next.head.ori = Quaternion::rotation_z(exp * -0.4)
            * Quaternion::rotation_x(0.0)
            * Quaternion::rotation_y(exp * 0.1);
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(
            0.0,
            skeleton_attr.chest.0 - exp * 1.5,
            skeleton_attr.chest.1,
        );
        next.chest.ori = Quaternion::rotation_z(0.4 + exp * 1.0)
            * Quaternion::rotation_x(0.0 + exp * 0.2)
            * Quaternion::rotation_y(exp * -0.08);

        next.belt.offset = Vec3::new(0.0, skeleton_attr.belt.0 + exp * 1.0, skeleton_attr.belt.1);
        next.belt.ori = next.chest.ori * -0.1;

        next.shorts.offset = Vec3::new(
            0.0,
            skeleton_attr.shorts.0 + exp * 1.0,
            skeleton_attr.shorts.1,
        );
        next.shorts.ori = next.chest.ori * -0.08;

        match active_tool_kind {
            //TODO: Inventory
            Some(ToolKind::Staff(_)) => {
                next.l_hand.offset = Vec3::new(1.5, 0.5, -4.0);
                next.l_hand.ori = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(-0.3);
                next.l_hand.scale = Vec3::one() * 1.05;
                next.r_hand.offset = Vec3::new(8.0, 4.0, 2.0);
                next.r_hand.ori = Quaternion::rotation_x(1.8)
                    * Quaternion::rotation_y(0.5)
                    * Quaternion::rotation_z(-0.27);
                next.r_hand.scale = Vec3::one() * 1.05;
                next.main.offset = Vec3::new(9.2, 8.4, 13.2);
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(3.14 + 0.3)
                    * Quaternion::rotation_z(0.9);

                next.control.offset = Vec3::new(-7.0, 6.0, 6.0 - exp * 5.0);
                next.control.ori = Quaternion::rotation_x(exp * 1.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(exp * 1.5);
                next.control.scale = Vec3::one();
            },
            Some(ToolKind::Bow(_)) => {
                next.l_hand.offset = Vec3::new(1.0 - exp * 2.0, -4.0 - exp * 7.0, -1.0 + exp * 6.0);
                next.l_hand.ori = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6 + exp * 0.8)
                    * Quaternion::rotation_z(-0.3 + exp * 0.9);
                next.l_hand.scale = Vec3::one() * 1.05;
                next.r_hand.offset = Vec3::new(3.0, -1.0, -5.0);
                next.r_hand.ori = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6)
                    * Quaternion::rotation_z(-0.3);
                next.r_hand.scale = Vec3::one() * 1.05;
                next.main.offset = Vec3::new(3.0, 2.0, -13.0);
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.3)
                    * Quaternion::rotation_z(-0.6);

                next.control.offset = Vec3::new(-9.0, 6.0, 8.0);
                next.control.ori = Quaternion::rotation_x(exp * 0.4)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.control.scale = Vec3::one();
            },
            _ => {},
        }
        if velocity > 0.5 {
            next.l_foot.offset = Vec3::new(
                -skeleton_attr.foot.0 - foot * 1.0 + exp * -1.0,
                foote * 0.8 + exp * 1.5,
                skeleton_attr.foot.2,
            );
            next.l_foot.ori = Quaternion::rotation_x(exp * 0.5)
                * Quaternion::rotation_z(exp * 0.4)
                * Quaternion::rotation_y(0.15);
            next.l_foot.scale = Vec3::one();

            next.r_foot.offset = Vec3::new(
                skeleton_attr.foot.0 + foot * 1.0 + exp * 1.0,
                foote * -0.8 + exp * -1.0,
                skeleton_attr.foot.2,
            );
            next.r_foot.ori = Quaternion::rotation_x(exp * -0.5)
                * Quaternion::rotation_z(exp * 0.4)
                * Quaternion::rotation_y(0.0);
            next.r_foot.scale = Vec3::one();
            next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
            next.torso.ori = Quaternion::rotation_x(-0.15);
            next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
        } else {
            next.l_foot.offset = Vec3::new(
                -skeleton_attr.foot.0,
                -2.5,
                skeleton_attr.foot.2 + exp * 2.5,
            );
            next.l_foot.ori =
                Quaternion::rotation_x(exp * -0.2 - 0.2) * Quaternion::rotation_z(exp * 1.0);

            next.r_foot.offset =
                Vec3::new(skeleton_attr.foot.0, 3.5 - exp * 2.0, skeleton_attr.foot.2);
            next.r_foot.ori = Quaternion::rotation_x(exp * 0.1) * Quaternion::rotation_z(exp * 0.5);
            next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
            next.torso.ori = Quaternion::rotation_z(0.0);
            next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
        }
        next.back.offset = Vec3::new(0.0, -2.8, 7.25);
        next.back.ori = Quaternion::rotation_x(-0.3);
        next.back.scale = Vec3::one() * 1.02;

        next.l_shoulder.offset = Vec3::new(-5.0, 0.0, 4.7);
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(5.0, 0.0, 4.7);
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.glider.offset = Vec3::new(0.0, 5.0, 0.0);
        next.glider.ori = Quaternion::rotation_y(0.0);
        next.glider.scale = Vec3::one() * 0.0;

        next.lantern.offset = Vec3::new(
            skeleton_attr.lantern.0,
            skeleton_attr.lantern.1,
            skeleton_attr.lantern.2,
        );
        next.lantern.ori =
            Quaternion::rotation_x(exp * -0.7 + 0.4) * Quaternion::rotation_y(exp * 0.4);
        next.lantern.scale = Vec3::one() * 0.65;

        next.l_control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.l_control.ori = Quaternion::rotation_x(0.0);
        next.l_control.scale = Vec3::one();

        next.r_control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.r_control.ori = Quaternion::rotation_x(0.0);
        next.r_control.scale = Vec3::one();
        next
    }
}
