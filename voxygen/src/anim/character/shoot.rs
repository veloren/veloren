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
            / (1.1 + 3.9 * ((anim_time as f32 * lab as f32 * 15.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 15.0).sin());

        let quick = (((5.0)
            / (0.5 + 4.5 * ((anim_time as f32 * lab as f32 * 2.0).cos()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 2.0).cos());
        let sloweralt = (((5.0)
            / (0.1 + 4.9 * ((anim_time as f32 * lab as f32 * 4.0).cos()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 4.0).cos());

        next.head.offset = Vec3::new(
            0.0 + skeleton_attr.neck_right,
            -2.0 + skeleton_attr.neck_forward - quick * 1.5,
            skeleton_attr.neck_height + 14.0,
        );
        next.head.ori = Quaternion::rotation_z(quick * 0.15)
            * Quaternion::rotation_x(quick * 0.09)
            * Quaternion::rotation_y(0.0);
        next.head.scale = Vec3::one() * skeleton_attr.head_scale;

        next.chest.offset = Vec3::new(0.0, 0.0 - quick * 1.5, 7.0);
        next.chest.ori = Quaternion::rotation_z(quick * 0.35)
            * Quaternion::rotation_x(quick * 0.09)
            * Quaternion::rotation_y(0.0);
        next.chest.scale = Vec3::one();

        next.belt.offset = Vec3::new(0.0, 0.0 + quick * 1.0, -2.0);
        next.belt.ori = next.chest.ori;
        next.belt.scale = Vec3::one();

        next.shorts.offset = Vec3::new(0.0, quick * 1.0, -5.0);
        next.shorts.ori = next.chest.ori;
        next.shorts.scale = Vec3::one();

        match active_tool_kind {
            //TODO: Inventory
            Some(ToolKind::Staff(_)) => {
                next.l_hand.offset = Vec3::new(1.0, -2.0, -5.0);
                next.l_hand.ori = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(-0.3);
                next.l_hand.scale = Vec3::one() * 1.05;
                next.r_hand.offset = Vec3::new(9.0, 1.0, 0.0);
                next.r_hand.ori = Quaternion::rotation_x(1.8)
                    * Quaternion::rotation_y(0.5)
                    * Quaternion::rotation_z(-0.27);
                next.r_hand.scale = Vec3::one() * 1.05;
                next.main.offset = Vec3::new(11.0, 9.0, 10.0);
                next.main.ori = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(3.14 + 0.3)
                    * Quaternion::rotation_z(0.9);
                next.main.scale = Vec3::one();

                next.control.offset = Vec3::new(-7.0, 6.0, 6.0 - quick * 5.0);
                next.control.ori = Quaternion::rotation_x(quick * 1.3)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(quick * 1.5);
                next.control.scale = Vec3::one();
            },
            Some(ToolKind::Bow(_)) => {
                next.l_hand.offset = Vec3::new(
                    1.0 - sloweralt * 2.0,
                    -4.0 - sloweralt * 7.0,
                    -1.0 + sloweralt * 6.0,
                );
                next.l_hand.ori = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6 + sloweralt * 0.8)
                    * Quaternion::rotation_z(-0.3 + sloweralt * 0.9);
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
                next.main.scale = Vec3::one();

                next.control.offset = Vec3::new(-7.0, 6.0, 6.0);
                next.control.ori = Quaternion::rotation_x((sloweralt * 0.4).max(0.4))
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.control.scale = Vec3::one();
            },
            _ => {},
        }
        if velocity > 0.5 {
            next.l_foot.offset = Vec3::new(-3.4, foot * -2.0, 8.0);
            next.l_foot.ori = Quaternion::rotation_x(foot * -0.4);
            next.l_foot.scale = Vec3::one();

            next.r_foot.offset = Vec3::new(3.4, foot * 2.0, 8.0);
            next.r_foot.ori = Quaternion::rotation_x(foot * 0.4);
            next.r_foot.scale = Vec3::one();
            next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
            next.torso.ori = Quaternion::rotation_z(0.0)
                * Quaternion::rotation_x(-0.15)
                * Quaternion::rotation_y(0.0);
            next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
        } else {
            next.l_foot.offset = Vec3::new(-3.4, -2.5, 8.0 + (quick * 2.5).max(0.0));
            next.l_foot.ori = Quaternion::rotation_x(quick * -0.2 - 0.2)
                * Quaternion::rotation_z((quick * 1.0).max(0.0));
            next.l_foot.scale = Vec3::one();

            next.r_foot.offset = Vec3::new(3.4, 3.5 - quick * 2.0, 8.0);
            next.r_foot.ori = Quaternion::rotation_x(quick * 0.1)
                * Quaternion::rotation_z((quick * 0.5).max(0.0));
            next.r_foot.scale = Vec3::one();
            next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
            next.torso.ori = Quaternion::rotation_z(0.0)
                * Quaternion::rotation_x(0.0)
                * Quaternion::rotation_y(0.0);
            next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
        }

        next.l_shoulder.offset = Vec3::new(-5.0, 0.0, 4.7);
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(5.0, 0.0, 4.7);
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.glider.offset = Vec3::new(0.0, 5.0, 0.0);
        next.glider.ori = Quaternion::rotation_y(0.0);
        next.glider.scale = Vec3::one() * 0.0;

        next.lantern.offset = Vec3::new(0.0, 0.0, 0.0);
        next.lantern.ori = Quaternion::rotation_x(0.0);
        next.lantern.scale = Vec3::one() * 0.0;

        next.l_control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.l_control.ori = Quaternion::rotation_x(0.0);
        next.l_control.scale = Vec3::one();

        next.r_control.offset = Vec3::new(0.0, 0.0, 0.0);
        next.r_control.ori = Quaternion::rotation_x(0.0);
        next.r_control.scale = Vec3::one();
        next
    }
}
