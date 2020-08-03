use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use std::f32::consts::PI;

pub struct SpinMeleeAnimation;

impl Animation for SpinMeleeAnimation {
    type Dependency = (Option<ToolKind>, Option<ToolKind>, Vec3<f32>, f64);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_spinmelee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_spinmelee")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, velocity, _global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let lab = 1.0;
        let speed = Vec2::<f32>::from(velocity).magnitude();
        let mut next = (*skeleton).clone();
        //torso movement
        let xshift = if velocity.z.abs() < 0.1 {
            ((anim_time as f32 - 1.1) * lab as f32 * 3.0).sin()
        } else {
            0.0
        };
        let yshift = if velocity.z.abs() < 0.1 {
            ((anim_time as f32 - 1.1) * lab as f32 * 3.0 + PI / 2.0).sin()
        } else {
            0.0
        };

        let spin = if anim_time < 1.1 && velocity.z.abs() < 0.1 {
            0.5 * ((anim_time as f32).powf(2.0))
        } else {
            lab as f32 * anim_time as f32 * 0.9
        };

        //feet
        let slowersmooth = (anim_time as f32 * lab as f32 * 4.0).sin();
        let quick = (anim_time as f32 * lab as f32 * 8.0).sin();

        if let Some(ToolKind::Axe(_)) = active_tool_kind {
            next.l_hand.offset = Vec3::new(-0.5, 0.0, 4.0);
            next.l_hand.ori = Quaternion::rotation_x(PI / 2.0)
                * Quaternion::rotation_z(0.0)
                * Quaternion::rotation_y(PI);
            next.l_hand.scale = Vec3::one() * 1.08;
            next.r_hand.offset = Vec3::new(0.5, 0.0, -2.5);
            next.r_hand.ori = Quaternion::rotation_x(PI / 2.0)
                * Quaternion::rotation_z(0.0)
                * Quaternion::rotation_y(0.0);
            next.r_hand.scale = Vec3::one() * 1.06;
            next.main.offset = Vec3::new(-0.0, -2.0, -1.0);
            next.main.ori = Quaternion::rotation_x(0.0)
                * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(0.0);

            next.control.offset = Vec3::new(0.0, 16.0, 3.0);
            next.control.ori = Quaternion::rotation_x(-1.4)
                * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(1.4);
            next.control.scale = Vec3::one();

            next.head.offset = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1);
            next.head.ori = Quaternion::rotation_z(0.0)
                * Quaternion::rotation_x(-0.15)
                * Quaternion::rotation_y(0.08);
            next.chest.offset = Vec3::new(
                0.0,
                skeleton_attr.chest.0 - 3.0,
                skeleton_attr.chest.1 - 2.0,
            );
            next.chest.ori = Quaternion::rotation_z(0.0)
                * Quaternion::rotation_x(-0.1)
                * Quaternion::rotation_y(0.3);
            next.chest.scale = Vec3::one();

            next.belt.offset = Vec3::new(0.0, 1.0, -1.0);
            next.belt.ori = Quaternion::rotation_z(0.0)
                * Quaternion::rotation_x(0.4)
                * Quaternion::rotation_y(0.0);
            next.belt.scale = Vec3::one() * 0.98;
            next.shorts.offset = Vec3::new(0.0, 3.0, -2.5);
            next.shorts.ori = Quaternion::rotation_z(0.0)
                * Quaternion::rotation_x(0.7)
                * Quaternion::rotation_y(0.0);
            next.shorts.scale = Vec3::one();
            next.torso.offset = Vec3::new(
                -xshift * (anim_time as f32).min(0.6),
                -yshift * (anim_time as f32).min(0.6),
                0.0,
            ) * skeleton_attr.scaler;
            next.torso.ori = Quaternion::rotation_z(spin * -16.0)
                * Quaternion::rotation_x(0.0)
                * Quaternion::rotation_y(0.0);
            next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
        }
        if velocity.z.abs() > 0.1 {
            next.l_foot.offset = Vec3::new(-skeleton_attr.foot.0, 8.0, skeleton_attr.foot.2 + 2.0);
            next.l_foot.ori = Quaternion::rotation_x(1.0) * Quaternion::rotation_z(0.0);
            next.l_foot.scale = Vec3::one();

            next.r_foot.offset = Vec3::new(skeleton_attr.foot.0, 8.0, skeleton_attr.foot.2 + 2.0);
            next.r_foot.ori = Quaternion::rotation_x(1.0);
            next.r_foot.scale = Vec3::one();
        } else if speed < 0.5 {
            next.l_foot.offset = Vec3::new(
                -skeleton_attr.foot.0,
                2.0 + quick * -6.0,
                skeleton_attr.foot.2,
            );
            next.l_foot.ori =
                Quaternion::rotation_x(0.5 + slowersmooth * 0.2) * Quaternion::rotation_z(0.0);
            next.l_foot.scale = Vec3::one();

            next.r_foot.offset = Vec3::new(skeleton_attr.foot.0, 4.0, skeleton_attr.foot.2);
            next.r_foot.ori =
                Quaternion::rotation_x(0.5 - slowersmooth * 0.2) * Quaternion::rotation_y(-0.4);
            next.r_foot.scale = Vec3::one();
        } else {
            next.l_foot.offset = Vec3::new(
                -skeleton_attr.foot.0,
                2.0 + quick * -6.0,
                skeleton_attr.foot.2,
            );
            next.l_foot.ori =
                Quaternion::rotation_x(0.5 + slowersmooth * 0.2) * Quaternion::rotation_z(0.0);
            next.l_foot.scale = Vec3::one();

            next.r_foot.offset = Vec3::new(
                skeleton_attr.foot.0,
                2.0 + quick * 6.0,
                skeleton_attr.foot.2,
            );
            next.r_foot.ori =
                Quaternion::rotation_x(0.5 - slowersmooth * 0.2) * Quaternion::rotation_z(0.0);
            next.r_foot.scale = Vec3::one();
        };
        next.lantern.offset = Vec3::new(
            skeleton_attr.lantern.0,
            skeleton_attr.lantern.1,
            skeleton_attr.lantern.2,
        );
        next.lantern.ori = Quaternion::rotation_z(0.0)
            * Quaternion::rotation_x(0.7)
            * Quaternion::rotation_y(-0.8);
        next.glider.offset = Vec3::new(0.0, 0.0, 10.0);
        next.glider.scale = Vec3::one() * 0.0;
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
