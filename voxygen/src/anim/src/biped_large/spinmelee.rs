use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};
use std::f32::consts::PI;

pub struct SpinMeleeAnimation;

impl Animation for SpinMeleeAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        Vec3<f32>,
        f64,
        Option<StageSection>,
    );
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_spinmelee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_spinmelee")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, _second_tool_kind, velocity, _global_time, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let lab = 1.0;
        let (_movement1, movement2, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time as f32, 0.0, 0.0),
            Some(StageSection::Swing) => (1.0, anim_time as f32, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time as f32),
            _ => (0.0, 0.0, 0.0),
        };
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
            0.5 * ((anim_time as f32).powi(2))
        } else {
            lab as f32 * anim_time as f32 * 0.9
        };

        //feet
        let slowersmooth = (anim_time as f32 * lab as f32 * 4.0).sin();
        let quick = (anim_time as f32 * lab as f32 * 8.0).sin();

        match active_tool_kind {
            Some(ToolKind::Sword) => {
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position = Vec3::new(s_a.shr.0, s_a.shr.1, s_a.shr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.shr.3) * Quaternion::rotation_y(s_a.shr.4);

                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation =
                    Quaternion::rotation_x(s_a.sc.3 - PI / 2.0 + movement3 * PI / 2.0)
                        * Quaternion::rotation_z(s_a.sc.5 - PI / 2.0 + movement3 * PI / 2.0);
                next.torso.orientation = Quaternion::rotation_z(movement2 * PI * 2.0);

                next.upper_torso.position = Vec3::new(0.0, s_a.upper_torso.0, s_a.upper_torso.1);
                next.upper_torso.orientation = Quaternion::rotation_y(0.3 + movement3 * -0.3);
                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
                next.head.orientation = Quaternion::rotation_x(-0.15 + movement3 * 0.15);
                next.lower_torso.orientation = Quaternion::rotation_x(0.2);
            },
            Some(ToolKind::Axe) => {
                next.hand_l.position = Vec3::new(-0.5, 0.0, 4.0);
                next.hand_l.orientation =
                    Quaternion::rotation_x(PI / 2.0) * Quaternion::rotation_y(PI);
                next.hand_r.position = Vec3::new(0.5, 0.0, -2.5);
                next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0);
                next.main.position = Vec3::new(-0.0, -2.0, -1.0);
                next.main.orientation = Quaternion::rotation_x(0.0);

                next.control.position = Vec3::new(0.0, 16.0, 3.0);
                next.control.orientation =
                    Quaternion::rotation_x(-1.4) * Quaternion::rotation_z(1.4);

                next.head.orientation =
                    Quaternion::rotation_x(-0.15) * Quaternion::rotation_y(0.08);
                next.upper_torso.position =
                    Vec3::new(0.0, s_a.upper_torso.0 - 3.0, s_a.upper_torso.1 - 2.0);
                next.upper_torso.orientation = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(-0.1)
                    * Quaternion::rotation_y(0.3);

                next.lower_torso.position = Vec3::new(0.0, 3.0, -2.5);
                next.lower_torso.orientation = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(0.7)
                    * Quaternion::rotation_y(0.0);
                next.torso.position = Vec3::new(
                    -xshift * (anim_time as f32).min(0.6),
                    -yshift * (anim_time as f32).min(0.6),
                    0.0,
                );
                next.torso.orientation = Quaternion::rotation_z(spin * -16.0)
                    * Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0);
                if velocity.z.abs() > 0.1 {
                    next.foot_l.position = Vec3::new(-s_a.foot.0, 8.0, s_a.foot.2 + 2.0);
                    next.foot_l.orientation =
                        Quaternion::rotation_x(1.0) * Quaternion::rotation_z(0.0);

                    next.foot_r.position = Vec3::new(s_a.foot.0, 8.0, s_a.foot.2 + 2.0);
                    next.foot_r.orientation = Quaternion::rotation_x(1.0);
                } else if speed < 0.5 {
                    next.foot_l.position = Vec3::new(-s_a.foot.0, 2.0 + quick * -6.0, s_a.foot.2);
                    next.foot_l.orientation = Quaternion::rotation_x(0.5 + slowersmooth * 0.2);

                    next.foot_r.position = Vec3::new(s_a.foot.0, 4.0, s_a.foot.2);
                    next.foot_r.orientation = Quaternion::rotation_x(0.5 - slowersmooth * 0.2)
                        * Quaternion::rotation_y(-0.4);
                } else {
                    next.foot_l.position = Vec3::new(-s_a.foot.0, 2.0 + quick * -6.0, s_a.foot.2);
                    next.foot_l.orientation = Quaternion::rotation_x(0.5 + slowersmooth * 0.2);

                    next.foot_r.position = Vec3::new(s_a.foot.0, 2.0 + quick * 6.0, s_a.foot.2);
                    next.foot_r.orientation = Quaternion::rotation_x(0.5 - slowersmooth * 0.2);
                };
            },
            _ => {},
        }

        next
    }
}
