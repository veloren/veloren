use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::StageSection,
};

pub struct BetaAnimation;

impl Animation for BetaAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        f32,
        f64,
        Option<StageSection>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_beta\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_beta")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, _velocity, _global_time, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let lab = 1.0;

        let (movement1, movement2, movement3) = match stage_section {
            Some(StageSection::Buildup) => {
                (anim_time as f32, 0.0, 0.0)
            },
            Some(StageSection::Cast) => {
                (1.0, anim_time as f32, 0.0)
            },
            Some(StageSection::Recover) => {
                (1.0, 1.0, anim_time as f32)
            },
            _ => (0.0, 0.0, 0.0),
        };

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

        if let Some(ToolKind::Sword(_)) = active_tool_kind {
            next.hand_l.position = Vec3::new(-0.75, -1.0, 2.5);
            next.hand_l.orientation = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(-0.2);
            next.hand_l.scale = Vec3::one() * 1.04;
            next.hand_r.position = Vec3::new(0.75, -1.5, -0.5);
            next.hand_r.orientation = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(0.3);
            next.hand_r.scale = Vec3::one() * 1.05;
            next.main.position = Vec3::new(0.0, 0.0, 2.0);
            next.main.orientation = Quaternion::rotation_x(-0.1)
                * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(0.0);

            next.head.position = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1);

            next.control.position = Vec3::new(
                -8.0 + movement1 * -5.0 + (movement2 as f32 * 2.5).sin() * 30.0 + movement3 * -5.0,
                1.0 - (movement1 as f32 * 8.0).sin() * 0.8 + movement1 * 2.0  + movement3 * 2.0,
                2.0 - (movement1 as f32 * 8.0).sin() * 0.4,
            );
            next.control.orientation = Quaternion::rotation_x(-1.57)
                * Quaternion::rotation_y(0.0 + movement1 * 1.5 + (movement2 as f32 * 2.5).sin() * 0.5)
                * Quaternion::rotation_z(1.0 + (movement2 as f32 * 2.5).sin() * 1.0);
            next.chest.orientation = Quaternion::rotation_y(-0.1)
                * Quaternion::rotation_z(0.4 + movement1 * 1.5 + (movement2 as f32 * 2.5).sin() * -0.5 + movement3 * 1.0);
            next.head.orientation = Quaternion::rotation_y(0.1)
                * Quaternion::rotation_z(-0.1 + movement1 * -1.1 + (movement2 as f32 * 2.5).sin() * -0.5);
        }

        match active_tool_kind {
            Some(ToolKind::Hammer(_)) => {
                next.head.position =
                    Vec3::new(0.0, -2.0 + skeleton_attr.head.0, skeleton_attr.head.1);
                next.head.orientation = Quaternion::rotation_z(slow * -0.18)
                    * Quaternion::rotation_x(-0.1 + slow * -0.28)
                    * Quaternion::rotation_y(0.2 + slow * 0.18);
                next.head.scale = Vec3::one() * skeleton_attr.head_scale;

                next.chest.position = Vec3::new(0.0 + foot * 2.0, 0.0, 7.0);
                next.chest.orientation = Quaternion::rotation_z(slow * 0.2)
                    * Quaternion::rotation_x(slow * 0.2)
                    * Quaternion::rotation_y(slow * -0.1);

                next.belt.position = Vec3::new(0.0, 0.0, -2.0);
                next.belt.orientation = Quaternion::rotation_z(slow * 0.1)
                    * Quaternion::rotation_x(slow * 0.1)
                    * Quaternion::rotation_y(slow * -0.04);

                next.shorts.position = Vec3::new(0.0, 0.0, -5.0);
                next.shorts.orientation = Quaternion::rotation_z(slow * 0.1)
                    * Quaternion::rotation_x(slow * 0.1)
                    * Quaternion::rotation_y(slow * -0.05);

                next.hand_l.position = Vec3::new(-0.75, -1.0, -2.5);
                next.hand_l.orientation = Quaternion::rotation_x(1.27);
                next.hand_l.scale = Vec3::one() * 1.04;
                next.hand_r.position = Vec3::new(0.75, -1.5, -5.5);
                next.hand_r.orientation = Quaternion::rotation_x(1.27);
                next.hand_r.scale = Vec3::one() * 1.05;
                next.main.position = Vec3::new(0.0, 6.0, -1.0);
                next.main.orientation = Quaternion::rotation_x(-0.3);

                next.control.position = Vec3::new(-8.0 + slow * 1.5, 1.5 + slow * 1.0, 0.0);
                next.control.orientation = Quaternion::rotation_x(-1.4)
                    * Quaternion::rotation_y(slow * 2.0 + 0.7)
                    * Quaternion::rotation_z(1.7 - slow * 0.4 + fast * 0.6);
                next.control.scale = Vec3::one();
                next.foot_l.position = Vec3::new(
                    -skeleton_attr.foot.0,
                    footquick * -9.5,
                    skeleton_attr.foot.2,
                );
                next.foot_l.orientation = Quaternion::rotation_x(footquick * 0.3)
                    * Quaternion::rotation_y(footquick * -0.6);

                next.foot_r.position =
                    Vec3::new(skeleton_attr.foot.0, footquick * 9.5, skeleton_attr.foot.2);
                next.foot_r.orientation = Quaternion::rotation_x(footquick * -0.3)
                    * Quaternion::rotation_y(footquick * 0.2);
                next.torso.position = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
            },
            Some(ToolKind::Axe(_)) => {
                next.head.position =
                    Vec3::new(0.0, -2.0 + skeleton_attr.head.0, skeleton_attr.head.1);
                next.head.orientation = Quaternion::rotation_z(slow * -0.18)
                    * Quaternion::rotation_x(-0.1 + slow * -0.28)
                    * Quaternion::rotation_y(0.2 + slow * 0.18);
                next.head.scale = Vec3::one() * skeleton_attr.head_scale;

                next.chest.position = Vec3::new(0.0 + foot * 2.0, 0.0, 7.0);
                next.chest.orientation = Quaternion::rotation_z(slow * 0.2)
                    * Quaternion::rotation_x(slow * 0.2)
                    * Quaternion::rotation_y(slow * -0.1);

                next.belt.position = Vec3::new(0.0, 0.0, -2.0);
                next.belt.orientation = Quaternion::rotation_z(slow * 0.1)
                    * Quaternion::rotation_x(slow * 0.1)
                    * Quaternion::rotation_y(slow * -0.04);

                next.shorts.position = Vec3::new(0.0, 0.0, -5.0);
                next.shorts.orientation = Quaternion::rotation_z(slow * 0.1)
                    * Quaternion::rotation_x(slow * 0.1)
                    * Quaternion::rotation_y(slow * -0.05);

                next.hand_l.position = Vec3::new(-0.75, -1.0, -2.5);
                next.hand_l.orientation = Quaternion::rotation_x(1.27);
                next.hand_l.scale = Vec3::one() * 1.04;
                next.hand_r.position = Vec3::new(0.75, -1.5, -5.5);
                next.hand_r.orientation = Quaternion::rotation_x(1.27);
                next.hand_r.scale = Vec3::one() * 1.05;
                next.main.position = Vec3::new(0.0, 6.0, -1.0);
                next.main.orientation = Quaternion::rotation_x(-0.3);

                next.control.position = Vec3::new(-8.0 + slow * 1.5, 1.5 + slow * 1.0, 0.0);
                next.control.orientation = Quaternion::rotation_x(-1.4)
                    * Quaternion::rotation_y(slow * 2.0 + 0.7)
                    * Quaternion::rotation_z(1.7 - slow * 0.4 + fast * 0.6);
                next.control.scale = Vec3::one();
                next.foot_l.position = Vec3::new(
                    -skeleton_attr.foot.0,
                    footquick * -9.5,
                    skeleton_attr.foot.2,
                );
                next.foot_l.orientation = Quaternion::rotation_x(footquick * 0.3)
                    * Quaternion::rotation_y(footquick * -0.6);

                next.foot_r.position =
                    Vec3::new(skeleton_attr.foot.0, footquick * 9.5, skeleton_attr.foot.2);
                next.foot_r.orientation = Quaternion::rotation_x(footquick * -0.3)
                    * Quaternion::rotation_y(footquick * 0.2);
                next.torso.position = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
            },
            _ => {},
        }

        next.shoulder_l.position = Vec3::new(
            -skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.shoulder_l.orientation = Quaternion::rotation_x(0.0);
        next.shoulder_l.scale = Vec3::one() * 1.1;

        next.shoulder_r.position = Vec3::new(
            skeleton_attr.shoulder.0,
            skeleton_attr.shoulder.1,
            skeleton_attr.shoulder.2,
        );
        next.shoulder_r.orientation = Quaternion::rotation_x(0.0);
        next.shoulder_r.scale = Vec3::one() * 1.1;

        next.glider.position = Vec3::new(0.0, 0.0, 10.0);
        next.glider.scale = Vec3::one() * 0.0;

        next.lantern.position = Vec3::new(
            skeleton_attr.lantern.0,
            skeleton_attr.lantern.1,
            skeleton_attr.lantern.2,
        );
        next.lantern.orientation =
            Quaternion::rotation_x(slow * -0.7 + 0.4) * Quaternion::rotation_y(slow * 0.4);
        next.lantern.scale = Vec3::one() * 0.65;
        next.hold.scale = Vec3::one() * 0.0;

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
