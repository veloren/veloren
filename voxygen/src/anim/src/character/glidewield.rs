use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use std::f32::consts::PI;

pub struct GlideWieldAnimation;

type GlideWieldAnimationDependency = (
    Option<ToolKind>,
    Option<ToolKind>,
    Vec3<f32>,
    Vec3<f32>,
    Vec3<f32>,
    f64,
);

impl Animation for GlideWieldAnimation {
    type Dependency = GlideWieldAnimationDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_glidewield\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_glidewield")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, velocity, _orientation, _last_ori, _global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();
        *rate = 1.0;

        let lab = 1.0;

        let shorte = (((5.0)
            / (4.0 + 1.0 * ((anim_time as f32 * lab as f32 * 16.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 16.0).sin());

        next.hand_l.position = Vec3::new(
            -2.0 - skeleton_attr.hand.0,
            skeleton_attr.hand.1,
            skeleton_attr.hand.2 + 15.0,
        );
        next.hand_l.orientation = Quaternion::rotation_x(3.35);
        next.hand_l.scale = Vec3::one();

        next.hand_r.position = Vec3::new(
            2.0 + skeleton_attr.hand.0,
            skeleton_attr.hand.1,
            skeleton_attr.hand.2 + 15.0,
        );
        next.hand_r.orientation = Quaternion::rotation_x(3.35);
        next.hand_r.scale = Vec3::one();

        if speed > 0.5 {
            next.glider.orientation = Quaternion::rotation_x(0.8);
            next.glider.position = Vec3::new(0.0, -10.0, 15.0);
            next.glider.scale = Vec3::one() * 1.0;

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
            next.lantern.orientation =
                Quaternion::rotation_x(shorte * 0.7 + 0.4) * Quaternion::rotation_y(shorte * 0.4);
        } else {
            next.glider.orientation = Quaternion::rotation_x(0.35);
            next.glider.position = Vec3::new(0.0, -9.0, 17.0);
            next.glider.scale = Vec3::one() * 1.0;

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
