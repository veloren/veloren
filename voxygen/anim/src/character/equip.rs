use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::ToolKind;
use core::f32::consts::PI;

pub struct EquipAnimation;

impl Animation for EquipAnimation {
    type Dependency<'a> = (Option<ToolKind>, Option<ToolKind>, f32, f32);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_equip\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_equip")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, _second_tool_kind, _velocity, _global_time): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        _s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();
        let equip_slow = 1.0 + (anim_time * 12.0 + PI).cos();
        let equip_slowa = 1.0 + (anim_time * 12.0 + PI / 4.0).cos();
        next.hand_l.orientation = Quaternion::rotation_y(-2.3) * Quaternion::rotation_z(PI / 2.0);
        next.hand_r.orientation = Quaternion::rotation_y(-2.3) * Quaternion::rotation_z(PI / 2.0);
        next.control.position = Vec3::new(equip_slowa * -1.5, 0.0, equip_slow * 1.5);

        match active_tool_kind {
            Some(ToolKind::Sword) => {
                next.hand_l.position = Vec3::new(-8.0, -5.0, 17.0);
                next.hand_r.position = Vec3::new(-6.0, -4.5, 14.0);
            },
            Some(ToolKind::Axe) => {
                next.hand_l.position = Vec3::new(-7.0, -5.0, 17.0);
                next.hand_r.position = Vec3::new(-5.0, -4.5, 14.0);
            },
            Some(ToolKind::Hammer | ToolKind::Pick) => {
                next.hand_l.position = Vec3::new(-5.0, -5.0, 13.0);
                next.hand_r.position = Vec3::new(-3.0, -4.5, 10.0);
            },
            Some(ToolKind::Staff) | Some(ToolKind::Sceptre) => {
                next.hand_l.position = Vec3::new(-3.0, -5.0, 8.0);
                next.hand_r.position = Vec3::new(-1.75, -4.5, 5.0);
            },
            Some(ToolKind::Bow) => {
                next.hand_l.position = Vec3::new(-3.0, -5.0, 9.0);
                next.hand_r.position = Vec3::new(-1.75, -4.5, 7.0);
            },
            Some(ToolKind::Instrument) => {
                next.hand_l.position = Vec3::new(-5.0, -5.0, 8.0);
            },
            _ => {},
        }

        next
    }
}
