use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::StageSection,
};
use std::f32::consts::PI;

pub struct Input {
    pub attack: bool,
}
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
        (active_tool_kind, second_tool_kind, _global_time, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
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

        fn slow (x: f32) -> f32 { (((5.0)
            / (1.1 + 3.9 * ((x * 12.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((x * 12.4).sin()) }

        fn short (x: f32) -> f32 { (((5.0)
            / (1.5 + 3.5 * ((x * 5.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((x * 5.0).sin()) }
        fn foothoril (x: f32) -> f32 { (x * 5.0 + PI * 1.45).sin() }
        fn foothorir (x: f32) -> f32 { (x * 5.0 + PI * (0.45)).sin() }

        fn footvertl (x: f32) -> f32 { (x * 5.0).sin() }
        fn footvertr (x: f32) -> f32 { (x * 5.0 + PI).sin() }

        fn footrotl (x: f32) -> f32 { (((1.0)
            / (0.05
                + (0.95)
                    * ((x * 5.0 + PI * 1.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((x * 5.0 + PI * 1.4).sin()) }

        fn footrotr (x: f32) -> f32 { (((1.0)
            / (0.05
                + (0.95)
                    * ((x * 5.0 + PI * 0.4).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((x * 5.0 + PI * 0.4).sin()) }

        fn shortalt (x: f32) -> f32 { (x * 5.0 + PI / 2.0).sin() }

        next.head.position = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1);

        next.hand_l.position = Vec3::new(-0.75, -1.0, 2.5);
        next.hand_l.orientation = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(-0.2);
        next.hand_l.scale = Vec3::one() * 1.02;
        next.hand_r.position = Vec3::new(0.75, -1.5, -0.5);
        next.hand_r.orientation = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(0.3);
        next.hand_r.scale = Vec3::one() * 1.02;
        next.main.position = Vec3::new(0.0, 0.0, 2.0);
        next.main.orientation = Quaternion::rotation_x(-0.1)
            * Quaternion::rotation_y(0.0)
            * Quaternion::rotation_z(0.0);

        match active_tool_kind {
            //TODO: Inventory
            Some(ToolKind::Sword(_)) => {

                next.head.position = Vec3::new(
                    0.0,
                    0.0 + skeleton_attr.head.0,
                    skeleton_attr.head.1 + movement2.min(1.0) * 1.0,
                );
                next.head.orientation =
                    Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(movement2.min(1.0) * -0.3 + movement3 * 0.3)
                    * Quaternion::rotation_z(movement1 * -0.9 + movement3 * 1.6);

                next.chest.position = Vec3::new(
                    0.0,
                    skeleton_attr.chest.0,
                    skeleton_attr.chest.1 + 2.0 + shortalt(movement2) * -2.5,
                );
                next.chest.orientation =
                    Quaternion::rotation_x(movement2.min(1.0) * -0.4 + movement3 * 0.4)
                    * Quaternion::rotation_y(movement2.min(1.0) * -0.2 + movement3 * 0.3)
                    * Quaternion::rotation_z(movement1 * 1.1 + movement3 * -2.2);

                next.control.position = Vec3::new(
                    -7.0 + movement1 * -5.0 + movement3 * -2.0,
                    7.0 + movement2.min(1.0) * -2.0,
                    2.0 + movement2.min(1.0) * 2.0
                );
                next.control.orientation =
                    Quaternion::rotation_x(movement1 * -1.0 + movement3 * -0.5)
                    * Quaternion::rotation_y(movement1 * 1.5 + movement3 * -2.5)
                    * Quaternion::rotation_z(0.0);
                next.control.scale = Vec3::one();

                next.shorts.orientation = Quaternion::rotation_z(short(movement2).min(1.0) * 0.25);

                next.belt.orientation = Quaternion::rotation_z(short(movement2).min(1.0) * 0.1);

                next.foot_l.position = Vec3::new(
                    -skeleton_attr.foot.0,
                    skeleton_attr.foot.1 + movement1 * -12.0 + foothoril(movement2) * -7.5,
                    skeleton_attr.foot.2 + ((footvertl(movement2) * -4.0).max(-1.0)),
                );
                next.foot_l.orientation = Quaternion::rotation_x(movement1 * -1.0 + footrotl(movement2) * -0.6);

                next.foot_r.position = Vec3::new(
                    skeleton_attr.foot.0,
                    skeleton_attr.foot.1 + foothorir(movement2) * -7.5,
                    skeleton_attr.foot.2 + ((footvertr(movement2) * -4.0).max(-1.0)),
                );
                next.foot_r.orientation =
                    Quaternion::rotation_x(-0.6 + footrotr(movement2) * -0.6)
                    * Quaternion::rotation_z(-0.2);
            },
            Some(ToolKind::Dagger(_)) => {
                next.head.position = Vec3::new(
                    0.0,
                    skeleton_attr.head.0,
                    -2.0 + skeleton_attr.head.1,
                );
                next.head.orientation = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0);
                next.head.scale = Vec3::one() * skeleton_attr.head_scale;

                next.chest.position = Vec3::new(0.0, 0.0, 7.0 + slow(anim_time as f32) * 2.0);
                next.chest.orientation =
                    Quaternion::rotation_x(-0.5) * Quaternion::rotation_z(-0.7);

                next.belt.position = Vec3::new(0.0, 1.0, -1.0);
                next.belt.orientation = Quaternion::rotation_x(0.2) * Quaternion::rotation_z(0.2);

                next.shorts.position = Vec3::new(0.0, 3.0, -3.0);
                next.shorts.orientation = Quaternion::rotation_x(0.4) * Quaternion::rotation_z(0.3);

                next.hand_l.position = Vec3::new(-0.75, -1.0, -2.5);
                next.hand_l.orientation = Quaternion::rotation_x(1.27);
                next.hand_l.scale = Vec3::one() * 1.04;
                next.hand_r.position = Vec3::new(0.75, -1.5, -5.5);
                next.hand_r.orientation = Quaternion::rotation_x(1.27);
                next.hand_r.scale = Vec3::one() * 1.05;
                next.main.position = Vec3::new(0.0, 6.0, -1.0);
                next.main.orientation = Quaternion::rotation_x(-0.3);
                next.main.scale = Vec3::one();

                next.control.position = Vec3::new(-8.0 - slow(anim_time as f32) * 0.5, 3.0, 3.0);
                next.control.orientation =
                    Quaternion::rotation_x(-0.3) * Quaternion::rotation_z(1.1 + slow(anim_time as f32) * 0.2);
                next.control.scale = Vec3::one();
                next.foot_l.position = Vec3::new(-1.4, 2.0, skeleton_attr.foot.2);
                next.foot_l.orientation = Quaternion::rotation_x(-0.8);

                next.foot_r.position = Vec3::new(5.4, -1.0, skeleton_attr.foot.2);
                next.foot_r.orientation = Quaternion::rotation_x(-0.8);
            },
            _ => {},
        }
        match second_tool_kind {
            //TODO: Inventory
            Some(ToolKind::Dagger(_)) => {
                next.head.position = Vec3::new(
                    0.0,
                    skeleton_attr.head.0,
                    -2.0 + skeleton_attr.head.1,
                );
                next.head.orientation = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0);
                next.head.scale = Vec3::one() * skeleton_attr.head_scale;

                next.chest.position = Vec3::new(0.0, 0.0, 7.0 + slow(anim_time as f32) * 2.0);
                next.chest.orientation = Quaternion::rotation_x(0.0);

                next.belt.position = Vec3::new(0.0, 1.0, -1.0);
                next.belt.orientation = Quaternion::rotation_x(0.0);

                next.shorts.position = Vec3::new(0.0, 3.0, -3.0);
                next.shorts.orientation = Quaternion::rotation_x(0.0);

                next.control.position = Vec3::new(0.0, 0.0, 0.0);
                next.control.orientation = Quaternion::rotation_x(0.0);
                next.control.scale = Vec3::one();

                next.control_l.position = Vec3::new(-8.0, -10.0, 0.0);

                next.hand_l.position = Vec3::new(0.0, 0.0, 0.0);
                next.hand_l.orientation = Quaternion::rotation_x(0.0);
                next.hand_l.scale = Vec3::one() * 1.04;

                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);
                next.main.scale = Vec3::one();

                next.control_r.position = Vec3::new(8.0, 10.0, 0.0);

                next.hand_r.position = Vec3::new(0.0, 0.0, 0.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.0);
                next.hand_r.scale = Vec3::one() * 1.05;

                next.second.position = Vec3::new(0.0, 6.0, -1.0);
                next.second.orientation = Quaternion::rotation_x(-0.3);
                next.second.scale = Vec3::one();

                next.foot_l.position = Vec3::new(-1.4, 2.0, skeleton_attr.foot.2);
                next.foot_l.orientation = Quaternion::rotation_x(-0.8);

                next.foot_r.position = Vec3::new(5.4, -1.0, skeleton_attr.foot.2);
                next.foot_r.orientation = Quaternion::rotation_x(-0.8);
            },
            _ => {},
        }

        next.lantern.position = Vec3::new(
            skeleton_attr.lantern.0,
            skeleton_attr.lantern.1,
            skeleton_attr.lantern.2,
        );
        next.lantern.orientation =
            Quaternion::rotation_x(slow(anim_time as f32) * -0.7 + 0.4) * Quaternion::rotation_y(slow(anim_time as f32) * 0.4);
        next.hold.scale = Vec3::one() * 0.0;

        next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

        next.control_l.scale = Vec3::one();

        next.control_r.scale = Vec3::one();

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