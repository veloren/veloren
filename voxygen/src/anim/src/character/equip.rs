use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use std::f32::consts::PI;

pub struct EquipAnimation;

impl Animation for EquipAnimation {
    type Dependency = (Option<ToolKind>, Option<ToolKind>, f32, f64);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_equip\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_equip")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, velocity, _global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();
        let lab = 1.0;

        let short = (((5.0)
            / (1.5 + 3.5 * ((anim_time as f32 * lab as f32 * 16.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 16.0).sin());

        let equip_slow = 1.0 + (anim_time as f32 * 12.0 + PI).cos();
        let equip_slowa = 1.0 + (anim_time as f32 * 12.0 + PI / 4.0).cos();

        match active_tool_kind {
            Some(ToolKind::Sword(_)) => {
                next.hand_l.position = Vec3::new(-0.75, -1.0, -2.5);
                next.hand_l.orientation =
                    Quaternion::rotation_x(1.57) * Quaternion::rotation_y(-0.2);
                next.hand_r.position = Vec3::new(0.75, -1.5, -5.5);
                next.hand_r.orientation =
                    Quaternion::rotation_x(1.57) * Quaternion::rotation_y(0.3);
                next.main.position = Vec3::new(0.0, 0.0, -6.0);
                next.main.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.control.position =
                    Vec3::new(-3.0 + equip_slowa * -1.5, -5.0, 12.0 + equip_slow * 1.5);
                next.control.orientation =
                    Quaternion::rotation_y(2.5) * Quaternion::rotation_z(1.57);
            },
            Some(ToolKind::Axe(_)) => {
                next.hand_l.position = Vec3::new(-4.0, 3.0, 6.0);
                next.hand_l.orientation = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_z(3.14 - 0.3)
                    * Quaternion::rotation_y(-0.8);
                next.hand_r.position = Vec3::new(-2.5, 9.0, 4.0);
                next.hand_r.orientation = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_z(3.14 - 0.3)
                    * Quaternion::rotation_y(-0.8);
                next.main.position = Vec3::new(-6.0, 10.0, -1.0);
                next.main.orientation = Quaternion::rotation_x(1.27)
                    * Quaternion::rotation_y(-0.3)
                    * Quaternion::rotation_z(-0.8);

                next.control.position = Vec3::new(0.0, 0.0, 0.0);
                next.control.orientation =
                    Quaternion::rotation_x(0.2) * Quaternion::rotation_y(-0.3);
            },
            Some(ToolKind::Hammer(_)) => {
                next.hand_l.position = Vec3::new(-7.0, 5.5, 3.5);
                next.hand_l.orientation =
                    Quaternion::rotation_x(0.0) * Quaternion::rotation_y(0.32);
                next.hand_r.position = Vec3::new(8.0, 7.75, 0.0);
                next.hand_r.orientation =
                    Quaternion::rotation_x(0.0) * Quaternion::rotation_y(0.22);
                next.main.position = Vec3::new(6.0, 7.0, 0.0);
                next.main.orientation =
                    Quaternion::rotation_y(-1.35) * Quaternion::rotation_z(1.57);

                next.control.position =
                    Vec3::new(-3.0 + equip_slowa * -1.5, -12.0, 12.0 + equip_slow * 1.5);
                next.control.orientation =
                    Quaternion::rotation_x(0.0) * Quaternion::rotation_y(1.35 + 2.5);
            },
            Some(ToolKind::Staff(_)) | Some(ToolKind::Sceptre(_)) => {
                next.hand_l.position = Vec3::new(0.0, 0.0, -4.0);
                next.hand_l.orientation =
                    Quaternion::rotation_x(1.27) * Quaternion::rotation_y(0.0);
                next.hand_r.position = Vec3::new(0.0, 0.0, 2.0);
                next.hand_r.orientation =
                    Quaternion::rotation_x(1.57) * Quaternion::rotation_y(0.2);
                next.main.position = Vec3::new(0.0, 0.0, 13.2);
                next.main.orientation = Quaternion::rotation_y(3.14);

                next.control.position = Vec3::new(-4.0, 7.0, 4.0);
                next.control.orientation = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.15)
                    * Quaternion::rotation_z(0.0);
            },
            Some(ToolKind::Bow(_)) => {
                next.hand_l.position = Vec3::new(2.0, 1.5, 0.0);
                next.hand_l.orientation = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6)
                    * Quaternion::rotation_z(-0.3);
                next.hand_l.scale = Vec3::one() * 1.04;
                next.hand_r.position = Vec3::new(5.9, 4.5, -5.0);
                next.hand_r.orientation = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6)
                    * Quaternion::rotation_z(-0.3);
                next.hand_r.scale = Vec3::one() * 1.04;
                next.main.position = Vec3::new(3.0, 2.0, -13.0);
                next.main.orientation = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.3)
                    * Quaternion::rotation_z(-0.6);

                next.control.position = Vec3::new(-7.0, 6.0, 6.0);
                next.control.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
            },
            _ => {},
        }
        next.hold.scale = Vec3::one() * 0.0;

        if velocity > 0.5 {
            next.torso.orientation = Quaternion::rotation_x(-0.2);
        } else {
            next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1 + short * 0.2);

            next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2);
            next.foot_l.orientation = Quaternion::rotation_x(0.0);

            next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2);
            next.foot_r.orientation = Quaternion::rotation_x(0.0);
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
