use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};

pub struct ShootAnimation;

impl Animation for ShootAnimation {
    type Dependency = (Option<ToolKind>, Option<ToolKind>, f32, f64);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_shoot\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_shoot")]
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
        let foot = (((5.0)
            / (0.2 + 4.8 * ((anim_time as f32 * lab as f32 * 8.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 8.0).sin());
        let foote = (((5.0)
            / (0.5 + 4.5 * ((anim_time as f32 * lab as f32 * 8.0 + 1.57).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 8.0).sin());

        let exp = ((anim_time as f32).powf(0.3 as f32)).min(1.2);

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_z(exp * -0.4)
            * Quaternion::rotation_x(0.0)
            * Quaternion::rotation_y(exp * 0.1);
        next.head.scale = Vec3::one() * s_a.head_scale;

        next.chest.position = Vec3::new(0.0, s_a.chest.0 - exp * 1.5, s_a.chest.1);
        next.chest.orientation = Quaternion::rotation_z(0.4 + exp * 1.0)
            * Quaternion::rotation_x(0.0 + exp * 0.2)
            * Quaternion::rotation_y(exp * -0.08);

        next.belt.position = Vec3::new(0.0, s_a.belt.0 + exp * 1.0, s_a.belt.1);
        next.belt.orientation = next.chest.orientation * -0.1;

        next.shorts.position = Vec3::new(0.0, s_a.shorts.0 + exp * 1.0, s_a.shorts.1);
        next.shorts.orientation = next.chest.orientation * -0.08;

        match active_tool_kind {
            //TODO: Inventory
            Some(ToolKind::Staff(_)) | Some(ToolKind::Sceptre(_)) => {
                next.hand_l.position = Vec3::new(11.0, 5.0, -4.0);
                next.hand_l.orientation =
                    Quaternion::rotation_x(1.27) * Quaternion::rotation_y(0.0);
                next.hand_r.position = Vec3::new(12.0, 5.5, 2.0);
                next.hand_r.orientation =
                    Quaternion::rotation_x(1.57) * Quaternion::rotation_y(0.2);
                next.main.position = Vec3::new(12.0, 8.5, 13.2);
                next.main.orientation = Quaternion::rotation_y(3.14);

                next.control.position = Vec3::new(-7.0, 6.0, 6.0 - exp * 5.0);
                next.control.orientation =
                    Quaternion::rotation_x(exp * 1.3) * Quaternion::rotation_z(exp * 1.5);
            },
            Some(ToolKind::Bow(_)) => {
                next.hand_l.position =
                    Vec3::new(1.0 - exp * 2.0, -4.0 - exp * 4.0, -1.0 + exp * 6.0);
                next.hand_l.orientation = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6 + exp * 0.8)
                    * Quaternion::rotation_z(-0.3 + exp * 0.9);
                next.hand_r.position = Vec3::new(4.9, 3.0, -4.0);
                next.hand_r.orientation = Quaternion::rotation_x(1.20)
                    * Quaternion::rotation_y(-0.6)
                    * Quaternion::rotation_z(-0.3);
                next.main.position = Vec3::new(3.0, 2.0, -13.0);
                next.main.orientation = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.3)
                    * Quaternion::rotation_z(-0.6);

                next.control.position = Vec3::new(-9.0, 6.0, 8.0);
                next.control.orientation = Quaternion::rotation_x(exp * 0.4);
            },
            _ => {},
        }
        if velocity > 0.5 {
            next.foot_l.position = Vec3::new(
                -s_a.foot.0 - foot * 1.0 + exp * -1.0,
                foote * 0.8 + exp * 1.5,
                s_a.foot.2,
            );
            next.foot_l.orientation = Quaternion::rotation_x(exp * 0.5)
                * Quaternion::rotation_z(exp * 0.4)
                * Quaternion::rotation_y(0.15);

            next.foot_r.position = Vec3::new(
                s_a.foot.0 + foot * 1.0 + exp * 1.0,
                foote * -0.8 + exp * -1.0,
                s_a.foot.2,
            );
            next.foot_r.orientation = Quaternion::rotation_x(exp * -0.5)
                * Quaternion::rotation_z(exp * 0.4)
                * Quaternion::rotation_y(0.0);
            next.torso.orientation = Quaternion::rotation_x(-0.15);
        } else {
            next.foot_l.position = Vec3::new(-s_a.foot.0, -2.5, s_a.foot.2 + exp * 2.5);
            next.foot_l.orientation =
                Quaternion::rotation_x(exp * -0.2 - 0.2) * Quaternion::rotation_z(exp * 1.0);

            next.foot_r.position = Vec3::new(s_a.foot.0, 3.5 - exp * 2.0, s_a.foot.2);
            next.foot_r.orientation =
                Quaternion::rotation_x(exp * 0.1) * Quaternion::rotation_z(exp * 0.5);
            next.torso.position = Vec3::new(0.0, 0.0, 0.1) * s_a.scaler;
            next.torso.orientation = Quaternion::rotation_z(0.0);
            next.torso.scale = Vec3::one() / 11.0 * s_a.scaler;
        }
        next.back.orientation = Quaternion::rotation_x(-0.3);

        next.lantern.orientation =
            Quaternion::rotation_x(exp * -0.7 + 0.4) * Quaternion::rotation_y(exp * 0.4);

        next.hold.position = Vec3::new(17.5, -25.0, -10.5);
        next.hold.orientation = Quaternion::rotation_x(-1.6)
            * Quaternion::rotation_y(-0.1)
            * Quaternion::rotation_z(0.0);

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
