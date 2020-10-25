use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use std::f32::consts::PI;

pub struct EquipAnimation;

impl Animation for EquipAnimation {
    type Dependency = (Option<ToolKind>, Option<ToolKind>, f32, f64);
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_equip\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_equip")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, velocity, _global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        _s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();
        let lab = 1.0;

        let equip_slow = 1.0 + (anim_time as f32 * 12.0 + PI).cos();
        let equip_slowa = 1.0 + (anim_time as f32 * 12.0 + PI / 4.0).cos();
        next.hand_l.orientation = Quaternion::rotation_x(0.0)
            * Quaternion::rotation_y(-2.3)
            * Quaternion::rotation_z(-1.57);
        next.hand_r.orientation = Quaternion::rotation_x(0.0)
            * Quaternion::rotation_y(-2.3)
            * Quaternion::rotation_z(1.57);
        next.control.position = Vec3::new(equip_slowa * -1.5, 0.0, equip_slow * 1.5);

        match active_tool_kind {
            Some(ToolKind::Sword(_)) => {
                next.hand_l.position = Vec3::new(-8.0, -5.0, 17.0);
                next.hand_r.position = Vec3::new(-6.0, -4.5, 14.0);
            },
            Some(ToolKind::Axe(_)) => {
                next.hand_l.position = Vec3::new(-7.0, -5.0, 17.0);
                next.hand_r.position = Vec3::new(-5.0, -4.5, 14.0);
            },
            Some(ToolKind::Hammer(_)) => {
                next.hand_l.position = Vec3::new(-5.0, -5.0, 13.0);
                next.hand_r.position = Vec3::new(-3.0, -4.5, 10.0);
            },
            Some(ToolKind::Staff(_)) | Some(ToolKind::Sceptre(_)) => {
                next.hand_l.position = Vec3::new(-3.0, -5.0, 8.0);
                next.hand_r.position = Vec3::new(-1.75, -4.5, 5.0);
            },
            Some(ToolKind::Bow(_)) => {
                next.hand_l.position = Vec3::new(-3.0, -5.0, 9.0);
                next.hand_r.position = Vec3::new(-1.75, -4.5, 7.0);
            },
            _ => {},
        }

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
