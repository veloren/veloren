use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};
use std::f32::consts::PI;

pub struct DashAnimation;

impl Animation for DashAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        f64,
        Option<StageSection>,
    );
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_dash\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_dash")]
    #[allow(clippy::single_match)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, _second_tool_kind, _global_time, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (movement1, movement2, movement3, _movement4) = match stage_section {
            Some(StageSection::Buildup) => (anim_time as f32, 0.0, 0.0, 0.0),
            Some(StageSection::Charge) => (1.0, anim_time as f32, 0.0, 0.0),
            Some(StageSection::Swing) => (1.0, 1.0, anim_time as f32, 0.0),
            Some(StageSection::Recover) => (1.1, 1.0, 1.0, anim_time as f32),
            _ => (0.0, 0.0, 0.0, 0.0),
        };

        fn short(x: f32) -> f32 {
            (((5.0) / (1.5 + 3.5 * ((x * 5.0).sin()).powf(2.0 as f32))).sqrt()) * ((x * 5.0).sin())
        }
        fn foothoril(x: f32) -> f32 { (x * 5.0 + PI * 1.45).sin() }
        fn foothorir(x: f32) -> f32 { (x * 5.0 + PI * (0.45)).sin() }

        fn footvertl(x: f32) -> f32 { (x * 5.0).sin() }
        fn footvertr(x: f32) -> f32 { (x * 5.0 + PI).sin() }

        fn footrotl(x: f32) -> f32 {
            (((1.0) / (0.05 + (0.95) * ((x * 5.0 + PI * 1.4).sin()).powf(2.0 as f32))).sqrt())
                * ((x * 5.0 + PI * 1.4).sin())
        }

        fn footrotr(x: f32) -> f32 {
            (((1.0) / (0.05 + (0.95) * ((x * 5.0 + PI * 0.4).sin()).powf(2.0 as f32))).sqrt())
                * ((x * 5.0 + PI * 0.4).sin())
        }

        fn shortalt(x: f32) -> f32 { (x * 5.0 + PI / 2.0).sin() }

        next.hand_l.position = Vec3::new(-0.75, -1.0, 2.5);
        next.hand_l.orientation = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(-0.2);
        next.hand_r.position = Vec3::new(0.75, -1.5, -0.5);
        next.hand_r.orientation = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(0.3);
        next.main.position = Vec3::new(0.0, 0.0, 2.0);
        next.main.orientation = Quaternion::rotation_x(-0.1);

        match active_tool_kind {
            //TODO: Inventory
            Some(ToolKind::Sword(_)) => {
                next.head.position =
                    Vec3::new(0.0, 0.0 + s_a.head.0, s_a.head.1 + movement2.min(1.0) * 1.0);
                next.head.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(movement2.min(1.0) * -0.3 + movement3 * 0.3)
                    * Quaternion::rotation_z(movement1 * -0.9 + movement3 * 1.6);

                next.upper_torso.position = Vec3::new(
                    0.0,
                    s_a.upper_torso.0,
                    s_a.upper_torso.1 + 2.0 + shortalt(movement2) * -2.5,
                );
                next.upper_torso.orientation =
                    Quaternion::rotation_x(movement2.min(1.0) * -0.4 + movement3 * 0.4)
                        * Quaternion::rotation_y(movement2.min(1.0) * -0.2 + movement3 * 0.3)
                        * Quaternion::rotation_z(movement1 * 1.1 + movement3 * -2.2);

                next.control.position = Vec3::new(
                    -7.0 + movement1 * -5.0 + movement3 * -2.0,
                    7.0 + movement2.min(1.0) * -2.0,
                    2.0 + movement2.min(1.0) * 2.0,
                );
                next.control.orientation =
                    Quaternion::rotation_x(movement1 * -1.0 + movement3 * -0.5)
                        * Quaternion::rotation_y(movement1 * 1.5 + movement3 * -2.5)
                        * Quaternion::rotation_z(0.0);

                next.lower_torso.orientation =
                    Quaternion::rotation_z(short(movement2).min(1.0) * 0.25);

                next.foot_l.position = Vec3::new(
                    -s_a.foot.0,
                    s_a.foot.1 + movement1 * -12.0 + foothoril(movement2) * -7.5,
                    s_a.foot.2 + ((footvertl(movement2) * -4.0).max(-1.0)),
                );
                next.foot_l.orientation =
                    Quaternion::rotation_x(movement1 * -1.0 + footrotl(movement2) * -0.6);

                next.foot_r.position = Vec3::new(
                    s_a.foot.0,
                    s_a.foot.1 + foothorir(movement2) * -7.5,
                    s_a.foot.2 + ((footvertr(movement2) * -4.0).max(-1.0)),
                );
                next.foot_r.orientation = Quaternion::rotation_x(-0.6 + footrotr(movement2) * -0.6)
                    * Quaternion::rotation_z(-0.2);
            },
            _ => {},
        }

        next
    }
}
