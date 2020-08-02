use super::{super::Animation, CharacterSkeleton, SkeletonAttr};
use common::comp::item::{Hands, ToolKind};
use vek::*;

pub struct BetaAnimation;

impl Animation for BetaAnimation {
    type Dependency = (Option<ToolKind>, Option<ToolKind>, f32, f64);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_beta\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_beta")]
    #[allow(clippy::unnested_or_patterns)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, _velocity, _global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let lab = 1.0;

        let fast = (((5.0)
            / (1.1 + 3.9 * ((anim_time as f32 * lab as f32 * 28.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 28.0).sin());
        let footquick = (((5.0)
            / (0.4 + 4.6 * ((anim_time as f32 * lab as f32 * 14.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 14.0).sin());
        let foot = (((5.0)
            / (1.1 + 3.9 * ((anim_time as f32 * lab as f32 * 14.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 14.0).sin());
        let slow = (((5.0)
            / (0.6 + 4.4 * ((anim_time as f32 * lab as f32 * 14.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 14.0).sin());

        match active_tool_kind {
            //TODO: Inventory
            Some(ToolKind::Axe(_))
            | Some(ToolKind::Hammer(_))
            | Some(ToolKind::Sword(_))
            | Some(ToolKind::Dagger(_)) => {
                //INTENTION: SWORD
                next.head.offset =
                    Vec3::new(0.0, -2.0 + skeleton_attr.head.0, skeleton_attr.head.1);
                next.head.ori = Quaternion::rotation_z(slow * -0.18)
                    * Quaternion::rotation_x(-0.1 + slow * -0.28)
                    * Quaternion::rotation_y(0.2 + slow * 0.18);
                next.head.scale = Vec3::one() * skeleton_attr.head_scale;

                next.chest.offset = Vec3::new(0.0 + foot * 2.0, 0.0, 7.0);
                next.chest.ori = Quaternion::rotation_z(slow * 0.2)
                    * Quaternion::rotation_x(slow * 0.2)
                    * Quaternion::rotation_y(slow * -0.1);

                next.belt.offset = Vec3::new(0.0, 0.0, -2.0);
                next.belt.ori = Quaternion::rotation_z(slow * 0.1)
                    * Quaternion::rotation_x(slow * 0.1)
                    * Quaternion::rotation_y(slow * -0.04);

                next.shorts.offset = Vec3::new(0.0, 0.0, -5.0);
                next.shorts.ori = Quaternion::rotation_z(slow * 0.1)
                    * Quaternion::rotation_x(slow * 0.1)
                    * Quaternion::rotation_y(slow * -0.05);

                next.l_hand.offset = Vec3::new(-0.75, -1.0, -2.5);
                next.l_hand.ori = Quaternion::rotation_x(1.27);
                next.l_hand.scale = Vec3::one() * 1.04;
                next.r_hand.offset = Vec3::new(0.75, -1.5, -5.5);
                next.r_hand.ori = Quaternion::rotation_x(1.27);
                next.r_hand.scale = Vec3::one() * 1.05;
                next.main.offset = Vec3::new(0.0, 6.0, -1.0);
                next.main.ori = Quaternion::rotation_x(-0.3);

                next.control.offset = Vec3::new(-8.0 + slow * 1.5, 1.5 + slow * 1.0, 0.0);
                next.control.ori = Quaternion::rotation_x(-1.4)
                    * Quaternion::rotation_y(slow * 2.0 + 0.7)
                    * Quaternion::rotation_z(1.7 - slow * 0.4 + fast * 0.6);
                next.control.scale = Vec3::one();
                next.l_foot.offset = Vec3::new(
                    -skeleton_attr.foot.0,
                    footquick * -9.5,
                    skeleton_attr.foot.2,
                );
                next.l_foot.ori = Quaternion::rotation_x(footquick * 0.3)
                    * Quaternion::rotation_y(footquick * -0.6);

                next.r_foot.offset =
                    Vec3::new(skeleton_attr.foot.0, footquick * 9.5, skeleton_attr.foot.2);
                next.r_foot.ori = Quaternion::rotation_x(footquick * -0.3)
                    * Quaternion::rotation_y(footquick * 0.2);
                next.torso.offset = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
            },
            _ => {},
        }

        next.l_shoulder.offset = Vec3::new(
            -skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.l_shoulder.ori = Quaternion::rotation_x(0.0);
        next.l_shoulder.scale = Vec3::one() * 1.1;

        next.r_shoulder.offset = Vec3::new(
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.r_shoulder.ori = Quaternion::rotation_x(0.0);
        next.r_shoulder.scale = Vec3::one() * 1.1;

        next.glider.offset = Vec3::new(0.0, 0.0, 10.0);
        next.glider.scale = Vec3::one() * 0.0;

        next.lantern.offset = Vec3::new(
            skeleton_attr.lantern.0,
            skeleton_attr.lantern.1,
            skeleton_attr.lantern.2,
        );
        next.lantern.ori =
            Quaternion::rotation_x(slow * -0.7 + 0.4) * Quaternion::rotation_y(slow * 0.4);
        next.lantern.scale = Vec3::one() * 0.65;

        next.l_control.scale = Vec3::one();

        next.r_control.scale = Vec3::one();

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
