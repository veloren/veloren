use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
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
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_dash\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_dash")]
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

        let (movement1, movement2, movement3, movement4) = match stage_section {
            Some(StageSection::Buildup) => ((anim_time as f32).powf(0.25), 0.0, 0.0, 0.0),
            Some(StageSection::Charge) => (1.0, anim_time as f32, 0.0, 0.0),
            Some(StageSection::Swing) => (1.0, 1.0, anim_time as f32, 0.0),
            Some(StageSection::Recover) => (1.1, 1.0, 1.0, (anim_time as f32).powf(4.0)),
            _ => (0.0, 0.0, 0.0, 0.0),
        };

        fn slow(x: f32) -> f32 {
            (((5.0) / (1.1 + 3.9 * ((x * 12.4).sin()).powf(2.0 as f32))).sqrt())
                * ((x * 12.4).sin())
        }

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

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

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

                next.control.position = Vec3::new(
                    s_a.sc.0 + (movement1 * -5.0 + movement3 * -2.0) * (1.0 - movement4),
                    s_a.sc.1 + (movement2.min(1.0) * -2.0) * (1.0 - movement4),
                    s_a.sc.2 + (movement2.min(1.0) * 2.0) * (1.0 - movement4),
                );
                next.control.orientation = Quaternion::rotation_x(
                    s_a.sc.3 + (movement1 * -1.0 + movement3 * -0.5) * (1.0 - movement4),
                ) * Quaternion::rotation_y(
                    s_a.sc.4 + (movement1 * 1.5 + movement3 * -2.5) * (1.0 - movement4),
                );

                next.head.position =
                    Vec3::new(0.0, 0.0 + s_a.head.0, s_a.head.1 + movement2.min(1.0) * 1.0);
                next.head.orientation =
                    Quaternion::rotation_y(movement2.min(1.0) * -0.3 + movement3 * 0.3)
                        * (1.0 - movement4)
                        * Quaternion::rotation_z(movement1 * -0.9 + movement3 * 1.6)
                        * (1.0 - movement4);

                next.chest.position = Vec3::new(
                    0.0,
                    s_a.chest.0,
                    s_a.chest.1 + (2.0 + shortalt(movement2) * -2.5) * (1.0 - movement4),
                );
                next.chest.orientation = Quaternion::rotation_x(
                    (movement2.min(1.0) * -0.4 + movement3 * 0.4) * (1.0 - movement4),
                ) * Quaternion::rotation_y(
                    (movement2.min(1.0) * -0.2 + movement3 * 0.3) * (1.0 - movement4),
                ) * Quaternion::rotation_z(
                    (movement1 * 1.1 + movement3 * -2.2) * (1.0 - movement4),
                );

                next.shorts.orientation =
                    Quaternion::rotation_z((short(movement2).min(1.0) * 0.25) * (1.0 - movement4));

                next.belt.orientation =
                    Quaternion::rotation_z((short(movement2).min(1.0) * 0.1) * (1.0 - movement4));

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

        next.lantern.orientation = Quaternion::rotation_x(slow(anim_time as f32) * -0.7 + 0.4)
            * Quaternion::rotation_y(slow(anim_time as f32) * 0.4);

        next
    }
}
