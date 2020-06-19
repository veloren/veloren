use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::ToolKind;
use vek::*;

pub struct Input {
    pub attack: bool,
}
pub struct DashAnimation;

impl Animation for DashAnimation {
    type Dependency = (Option<ToolKind>, f64);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_dash\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_dash")]
    #[allow(clippy::single_match)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, _global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();
        let lab = 1.0;

        let foot = (((5.0)
            / (1.1 + 3.9 * ((anim_time as f32 * lab as f32 * 25.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 25.0).sin());

        let slow = (((5.0)
            / (1.1 + 3.9 * ((anim_time as f32 * lab as f32 * 12.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 12.4).sin());

        match active_tool_kind {
            //TODO: Inventory
            Some(ToolKind::Sword(_)) => {
                next.head.offset = Vec3::new(
                    0.0,
                    -2.0 + skeleton_attr.head.0,
                    -2.0 + skeleton_attr.head.1,
                );
                next.head.ori = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0);
                next.head.scale = Vec3::one() * skeleton_attr.head_scale;

                next.chest.offset = Vec3::new(0.0, 0.0, 7.0 + slow * 2.0);
                next.chest.ori = Quaternion::rotation_x(-0.5) * Quaternion::rotation_z(-0.7);

                next.belt.offset = Vec3::new(0.0, 1.0, -1.0);
                next.belt.ori = Quaternion::rotation_x(0.2) * Quaternion::rotation_z(0.2);

                next.shorts.offset = Vec3::new(0.0, 3.0, -3.0);
                next.shorts.ori = Quaternion::rotation_x(0.4) * Quaternion::rotation_z(0.3);

                next.l_hand.offset = Vec3::new(-0.75, -1.0, -2.5);
                next.l_hand.ori = Quaternion::rotation_x(1.27);
                next.l_hand.scale = Vec3::one() * 1.04;
                next.r_hand.offset = Vec3::new(0.75, -1.5, -5.5);
                next.r_hand.ori = Quaternion::rotation_x(1.27);
                next.r_hand.scale = Vec3::one() * 1.05;
                next.main.offset = Vec3::new(0.0, 6.0, -1.0);
                next.main.ori = Quaternion::rotation_x(-0.3);
                next.main.scale = Vec3::one();

                next.control.offset = Vec3::new(-8.0 - slow * 0.5, 3.0 - foot * 0.6, 3.0);
                next.control.ori =
                    Quaternion::rotation_x(-0.3) * Quaternion::rotation_z(1.1 + slow * 0.2);
                next.control.scale = Vec3::one();
                next.l_foot.offset = Vec3::new(-1.4, foot * 3.0 + 2.0, skeleton_attr.foot.2);
                next.l_foot.ori = Quaternion::rotation_x(foot * -0.4 - 0.8);

                next.r_foot.offset = Vec3::new(5.4, foot * -3.0 - 1.0, skeleton_attr.foot.2);
                next.r_foot.ori = Quaternion::rotation_x(foot * 0.4 - 0.8);
            },
            _ => {},
        }

        next.lantern.offset = Vec3::new(
            skeleton_attr.lantern.0,
            skeleton_attr.lantern.1,
            skeleton_attr.lantern.2,
        );
        next.lantern.ori =
            Quaternion::rotation_x(slow * -0.7 + 0.4) * Quaternion::rotation_y(slow * 0.4);

        next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
        next.torso.ori =
            Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0) * Quaternion::rotation_y(0.0);
        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next.l_control.scale = Vec3::one();

        next.r_control.scale = Vec3::one();
        next
    }
}
