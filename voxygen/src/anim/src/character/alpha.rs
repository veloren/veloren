use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::StageSection,
};
use std::f32::consts::PI;

pub struct AlphaAnimation;

impl Animation for AlphaAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        f32,
        f64,
        Option<StageSection>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_alpha\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_alpha")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, velocity, _global_time, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let lab = 1.0;

        let (movement1, movement2, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time as f32, 0.0, 0.0),
            Some(StageSection::Swing) => (1.0, anim_time as f32, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time as f32),
            _ => (0.0, 0.0, 0.0),
        };

        let foot = (((1.0)
            / (0.2
                + 0.8
                    * ((anim_time as f32 * lab as f32 * 2.0 * velocity).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 2.0 * velocity).sin());
        let slowersmooth = (anim_time as f32 * lab as f32 * 4.0).sin();
        let push = anim_time as f32 * lab as f32 * 4.0;
        let slow = (((5.0)
            / (0.4 + 4.6 * ((anim_time as f32 * lab as f32 * 9.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 9.0).sin());
        let quick = (((5.0)
            / (0.4 + 4.6 * ((anim_time as f32 * lab as f32 * 18.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 18.0).sin());
        let axe = (((1.0)
            / (0.05 + 0.95 * ((anim_time as f32 * lab as f32 * 8.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 8.0).sin());
        let staff = (((1.0)
            / (0.05 + 0.95 * ((anim_time as f32 * lab as f32 * 10.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 10.0).sin());
        let slower = (((1.0)
            / (0.0001 + 0.999 * ((anim_time as f32 * lab as f32 * 4.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 4.0).sin());

        if let Some(ToolKind::Sword(_)) = active_tool_kind {
            next.hand_l.position = Vec3::new(-0.75, -1.0, 2.5);
            next.hand_l.orientation = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(-0.2);
            next.hand_l.scale = Vec3::one() * 1.05;
            next.hand_r.position = Vec3::new(0.75, -1.5, -0.5);
            next.hand_r.orientation = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(0.3);
            next.hand_r.scale = Vec3::one() * 1.05;
            next.main.position = Vec3::new(0.0, 0.0, 2.0);
            next.main.orientation = Quaternion::rotation_x(-0.1)
                * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(0.0);

            next.control.position = Vec3::new(
                -7.0,
                7.0 + movement1 * -4.0 + movement2 * 16.0 + movement3 * -4.0,
                2.0 + movement1 * 1.0,
            );
            next.control.orientation = Quaternion::rotation_x(movement1 * -0.5)
                * Quaternion::rotation_y(movement1 * -1.0 + movement2 * -0.6 + movement3 * 1.0)
                * Quaternion::rotation_z(movement1 * -1.2 + movement2 * 1.3);

            next.chest.orientation = Quaternion::rotation_z(
                movement1 * 1.5 + (movement2 * 1.75).sin() * -3.0 + movement3 * 0.5,
            );

            next.head.position = Vec3::new(0.0, skeleton_attr.head.0 + 0.0, skeleton_attr.head.1);
            next.head.orientation = Quaternion::rotation_z(
                movement1 * -0.9 + (movement2 * 1.75).sin() * 2.5 + movement3 * -0.5,
            );
        }
        match active_tool_kind {
            //TODO: Inventory
            Some(ToolKind::Dagger(_)) => {
                next.head.position =
                    Vec3::new(0.0, -2.0 + skeleton_attr.head.0, skeleton_attr.head.1);
                next.head.orientation = Quaternion::rotation_z(slow * -0.25)
                    * Quaternion::rotation_x(0.0 + slow * 0.15)
                    * Quaternion::rotation_y(slow * -0.15);

                next.chest.position = Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1);
                next.chest.orientation = Quaternion::rotation_z(slow * 0.4)
                    * Quaternion::rotation_x(0.0 + slow * -0.2)
                    * Quaternion::rotation_y(slow * 0.2);

                next.belt.position = Vec3::new(0.0, skeleton_attr.belt.0, skeleton_attr.belt.1);
                next.belt.orientation = next.chest.orientation * -0.3;

                next.shorts.position =
                    Vec3::new(0.0, skeleton_attr.shorts.0, skeleton_attr.shorts.1);
                next.shorts.orientation = next.chest.orientation * -0.45;

                next.hand_l.position = Vec3::new(0.0, 0.0, 0.0);
                next.hand_l.orientation = Quaternion::rotation_x(0.0);

                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);

                next.control_l.position = Vec3::new(-10.0 + push * 5.0, 6.0 + push * 5.0, 2.0);
                next.control_l.orientation = Quaternion::rotation_x(-1.4 + slow * 0.4)
                    * Quaternion::rotation_y(slow * -1.3)
                    * Quaternion::rotation_z(1.4 + slow * -0.5);
                next.control_l.scale = Vec3::one();

                next.hand_r.position = Vec3::new(0.0, 0.0, 0.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.0);
                next.hand_r.scale = Vec3::one() * 1.12;

                next.second.position = Vec3::new(0.0, 0.0, 0.0);
                next.second.orientation = Quaternion::rotation_x(0.0);

                next.control_r.position = Vec3::new(8.0, 0.0, 0.0);
                next.control_r.orientation = Quaternion::rotation_x(0.0);
                next.control_r.scale = Vec3::one();

                // next.control_r.position = Vec3::new(-10.0 + push * 5.0, 6.0 + push * 5.0,
                // 2.0); next.control_r.orientation =
                // Quaternion::rotation_x(-1.4 + slow * 0.4)
                //     * Quaternion::rotation_y(slow * -1.3)
                //     * Quaternion::rotation_z(1.4 + slow * -0.5);
                // next.control_r.scale = Vec3::one();

                // next.hand_r.position = Vec3::new(0.75, -1.5, -5.5);
                // next.hand_r.orientation = Quaternion::rotation_x(1.27);
                // next.hand_r.scale = Vec3::one() * 1.05;

                // next.control.position = Vec3::new(-10.0 + push * 5.0, 6.0 + push * 5.0, 2.0);
                // next.control.orientation = Quaternion::rotation_x(-1.4 + slow * 0.4)
                //     * Quaternion::rotation_y(slow * -1.3)
                //     * Quaternion::rotation_z(1.4 + slow * -0.5);
                // next.control.scale = Vec3::one();

                next.foot_l.position = Vec3::new(
                    -skeleton_attr.foot.0,
                    slow * -3.0 + quick * 3.0 - 4.0,
                    skeleton_attr.foot.2,
                );
                next.foot_l.orientation = Quaternion::rotation_x(slow * 0.6)
                    * Quaternion::rotation_y((slow * -0.2).max(0.0));
                next.foot_l.scale = Vec3::one();

                next.foot_r.position = Vec3::new(
                    skeleton_attr.foot.0,
                    slow * 3.0 + quick * -3.0 + 5.0,
                    skeleton_attr.foot.2,
                );
                next.foot_r.orientation = Quaternion::rotation_x(slow * -0.6)
                    * Quaternion::rotation_y((slow * 0.2).min(0.0));
                next.foot_r.scale = Vec3::one();

                next.lantern.orientation =
                    Quaternion::rotation_x(slow * -0.7 + 0.4) * Quaternion::rotation_y(slow * 0.4);

                next.torso.position = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                next.torso.orientation = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0);
                next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
            },
            Some(ToolKind::Axe(_)) => {
                next.head.position =
                    Vec3::new(0.0, 0.0 + skeleton_attr.head.0, skeleton_attr.head.1);
                next.head.orientation = Quaternion::rotation_z(0.1 + axe * 0.2)
                    * Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.2);
                next.head.scale = Vec3::one() * skeleton_attr.head_scale;

                next.chest.position = Vec3::new(0.0, 0.0, 7.0);
                next.chest.orientation = Quaternion::rotation_z(0.2 + axe * 0.2);
                next.chest.scale = Vec3::one();

                next.belt.position = Vec3::new(0.0, 0.0, -2.0);
                next.belt.orientation = Quaternion::rotation_z(0.2 + axe * -0.1);

                next.shorts.position = Vec3::new(0.0, 0.0, -5.0);
                next.shorts.orientation = Quaternion::rotation_z(0.2 + axe * -0.2);

                next.hand_l.position = Vec3::new(-0.5, 0.0, 4.0);
                next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
                    * Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_y(0.0);
                next.hand_l.scale = Vec3::one() * 1.08;
                next.hand_r.position = Vec3::new(0.5, 0.0, -2.5);
                next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
                    * Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_y(0.0);
                next.hand_r.scale = Vec3::one() * 1.06;
                next.main.position = Vec3::new(-0.0, -2.0, -1.0);
                next.main.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);

                next.control.position = Vec3::new(2.0 + axe * -7.0, 11.0, 3.0);
                next.control.orientation = Quaternion::rotation_x(1.6)
                    * Quaternion::rotation_y(-2.0 + axe * 0.5)
                    * Quaternion::rotation_z(PI * 0.4);
                next.lantern.orientation =
                    Quaternion::rotation_x(0.4) * Quaternion::rotation_y(0.0);

                next.torso.position = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                next.torso.orientation = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0);
                next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
            },
            Some(ToolKind::Hammer(_)) => {
                next.hand_l.position = Vec3::new(-12.0, 0.0, 0.0);
                next.hand_l.orientation =
                    Quaternion::rotation_x(-0.0) * Quaternion::rotation_y(0.0);
                next.hand_l.scale = Vec3::one() * 1.08;
                next.hand_r.position = Vec3::new(3.0, 0.0, 0.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.0) * Quaternion::rotation_y(0.0);
                next.hand_r.scale = Vec3::one() * 1.06;
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(-1.57)
                    * Quaternion::rotation_z(1.57);

                next.head.position =
                    Vec3::new(0.0, -2.0 + skeleton_attr.head.0, skeleton_attr.head.1);
                next.head.orientation = Quaternion::rotation_z(slower * 0.03)
                    * Quaternion::rotation_x(slowersmooth * 0.1)
                    * Quaternion::rotation_y(slower * 0.05 + slowersmooth * 0.06)
                    * Quaternion::rotation_z((slowersmooth * -0.4).max(0.0));
                next.head.scale = Vec3::one() * skeleton_attr.head_scale;

                next.chest.position = Vec3::new(0.0, 0.0, 7.0);
                next.chest.orientation =
                    Quaternion::rotation_z(slower * 0.18 + slowersmooth * 0.15)
                        * Quaternion::rotation_x(0.0 + slower * 0.18 + slowersmooth * 0.15)
                        * Quaternion::rotation_y(slower * 0.18 + slowersmooth * 0.15);

                next.belt.position = Vec3::new(0.0, 0.0, -2.0);
                next.belt.orientation =
                    Quaternion::rotation_z(slower * -0.1 + slowersmooth * -0.075)
                        * Quaternion::rotation_x(0.0 + slower * -0.1)
                        * Quaternion::rotation_y(slower * -0.1);

                next.shorts.position = Vec3::new(0.0, 0.0, -5.0);
                next.shorts.orientation =
                    Quaternion::rotation_z(slower * -0.1 + slowersmooth * -0.075)
                        * Quaternion::rotation_x(0.0 + slower * -0.1)
                        * Quaternion::rotation_y(slower * -0.1);

                next.lantern.orientation = Quaternion::rotation_x(slower * -0.7 + 0.4)
                    * Quaternion::rotation_y(slower * 0.4);

                next.torso.position = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                next.torso.orientation = Quaternion::rotation_z(0.0);
                next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

                if velocity > 0.5 {
                    next.foot_l.position =
                        Vec3::new(-skeleton_attr.foot.0, foot * -6.0, skeleton_attr.foot.2);
                    next.foot_l.orientation = Quaternion::rotation_x(foot * -0.4)
                        * Quaternion::rotation_z((slower * 0.3).max(0.0));
                    next.foot_l.scale = Vec3::one();

                    next.foot_r.position =
                        Vec3::new(skeleton_attr.foot.0, foot * 6.0, skeleton_attr.foot.2);
                    next.foot_r.orientation = Quaternion::rotation_x(foot * 0.4)
                        * Quaternion::rotation_z((slower * 0.3).max(0.0));
                    next.foot_r.scale = Vec3::one();
                    next.torso.position = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                    next.torso.orientation =
                        Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.15);
                    next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
                } else {
                    next.foot_l.position = Vec3::new(
                        -skeleton_attr.foot.0,
                        -2.5,
                        skeleton_attr.foot.2 + (slower * 2.5).max(0.0),
                    );
                    next.foot_l.orientation = Quaternion::rotation_x(slower * -0.2 - 0.2)
                        * Quaternion::rotation_z((slower * 1.0).max(0.0));
                    next.foot_l.scale = Vec3::one();

                    next.foot_r.position = Vec3::new(
                        skeleton_attr.foot.0,
                        3.5 - slower * 2.0,
                        skeleton_attr.foot.2,
                    );
                    next.foot_r.orientation = Quaternion::rotation_x(slower * 0.1)
                        * Quaternion::rotation_z((slower * 0.5).max(0.0));
                    next.foot_r.scale = Vec3::one();
                    next.torso.position = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                    next.torso.orientation = Quaternion::rotation_z(0.0);
                    next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
                }

                next.control.scale = Vec3::one();
                next.control.position = Vec3::new(-8.0, 7.0, 1.0);
                next.control.orientation = Quaternion::rotation_x(-1.5 + slower * 1.5)
                    * Quaternion::rotation_y(slowersmooth * 0.35 - 0.3)
                    * Quaternion::rotation_z(1.4 + slowersmooth * 0.2);
                next.control.scale = Vec3::one();

                next.torso.position = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                next.torso.orientation = Quaternion::rotation_z(0.0);
                next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
            },
            Some(ToolKind::Staff(_)) | Some(ToolKind::Sceptre(_)) => {
                next.head.orientation =
                    Quaternion::rotation_x(staff * 0.2) * Quaternion::rotation_z(staff * 0.2);
                next.hand_l.position = Vec3::new(11.0, 5.0, -4.0);
                next.hand_l.orientation =
                    Quaternion::rotation_x(1.27) * Quaternion::rotation_y(0.0);
                next.hand_l.scale = Vec3::one() * 1.05;
                next.hand_r.position = Vec3::new(12.0, 5.5, 2.0);
                next.hand_r.orientation =
                    Quaternion::rotation_x(1.57) * Quaternion::rotation_y(0.2);
                next.hand_r.scale = Vec3::one() * 1.05;
                next.main.position = Vec3::new(12.0, 8.5, 13.2);
                next.main.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(3.14)
                    * Quaternion::rotation_z(0.0);
                next.chest.orientation = Quaternion::rotation_z(staff * 0.3);
                next.belt.orientation = Quaternion::rotation_z(staff * 0.2);
                next.shorts.orientation = Quaternion::rotation_z(staff * 0.4);

                next.control.position = Vec3::new(-20.0, 5.0 + staff * 3.0, 1.0);
                next.control.orientation = Quaternion::rotation_x(staff * 1.2)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.control.scale = Vec3::one();
            },
            Some(ToolKind::Debug(_)) => {
                next.hand_l.position = Vec3::new(-7.0, 4.0, 3.0);
                next.hand_l.orientation = Quaternion::rotation_x(1.27)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.hand_l.scale = Vec3::one() * 1.01;
                next.main.position = Vec3::new(-5.0, 5.0, 23.0);
                next.main.orientation = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(PI)
                    * Quaternion::rotation_y(0.0);
                next.main.scale = Vec3::one();
                next.torso.position = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
                next.torso.orientation = Quaternion::rotation_x(0.0);
                next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;
            },
            _ => {},
        }

        next.lantern.scale = Vec3::one() * 0.65;
        next.shoulder_l.scale = Vec3::one() * 1.1;
        next.shoulder_r.scale = Vec3::one() * 1.1;
        next.glider.position = Vec3::new(0.0, 0.0, 10.0);
        next.glider.scale = Vec3::one() * 0.0;
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
