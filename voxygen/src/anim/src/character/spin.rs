use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::wielding::StageSection,
};
use std::f32::consts::PI;

pub struct Input {
    pub attack: bool,
}
pub struct SpinAnimation;

impl Animation for SpinAnimation {
    type Dependency = (Option<ToolKind>, Option<ToolKind>, f64, Option<StageSection>);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_spin\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_spin")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, _global_time, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let lab = 1.0;

        let foot = (((5.0)
            / (1.1 + 3.9 * ((anim_time as f32 * lab as f32 * 10.32).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 10.32).sin());

        let decel = (anim_time as f32 * 16.0 * lab as f32).min(PI / 2.0).sin();

        let spin = (anim_time as f32 * 2.8 * lab as f32).sin();
        let spinhalf = (anim_time as f32 * 1.4 * lab as f32).sin();

        let build = (anim_time as f32 * 8.0).sin();
        let recover = (anim_time as f32 * 8.0).sin();

        let movement = anim_time as f32 * 1.0;
        let test = (anim_time as f32 * 8.0).sin();
        let test2 = (anim_time as f32 * 1.0).sin();

        if let Some(ToolKind::Sword(_)) = active_tool_kind {
            next.l_hand.position = Vec3::new(-0.75, -1.0, 2.5);
            next.l_hand.orientation = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(-0.2);
            next.l_hand.scale = Vec3::one() * 1.04;
            next.r_hand.position = Vec3::new(0.75, -1.5, -0.5);
            next.r_hand.orientation = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(0.3);
            next.r_hand.scale = Vec3::one() * 1.05;
            next.main.position = Vec3::new(0.0, 0.0, 2.0);
            next.main.orientation = Quaternion::rotation_x(-0.1)
                * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(0.0);

            if let Some(stage_section) = stage_section {
                match stage_section {
                    StageSection::Buildup => {
                        //println!("{:.3} build", anim_time);
                        next.control.position = Vec3::new(5.0, 11.0 + build * 0.6, 2.0 + build * 0.6);
                        next.control.orientation = Quaternion::rotation_x(-1.57)
                            * Quaternion::rotation_y(2.8)
                            * Quaternion::rotation_z(1.0);
                        next.chest.orientation = Quaternion::rotation_y(movement * -0.1)
                            * Quaternion::rotation_z(build * 0.05 + movement * -0.6);
                        next.belt.orientation = Quaternion::rotation_x(movement * 0.1);

                        next.shorts.orientation = Quaternion::rotation_x(movement * 0.1);

                        next.head.orientation = Quaternion::rotation_y(movement * 0.1)
                            * Quaternion::rotation_z(movement * 0.4);
                    },
                    StageSection::Swing => {
                        //println!("{:.3} swing", anim_time);
                        next.control.position = Vec3::new(
                            7.0 + movement * -8.0,
                            11.0 + test * 3.0,
                            2.0 + test * 3.5 + movement * 3.0,
                        );
                        next.control.orientation =
                            Quaternion::rotation_x(-1.57 + movement * -0.6 + test * -0.25)
                                * Quaternion::rotation_y(2.8 + movement * -2.0)
                                * Quaternion::rotation_z(1.0 + movement * 1.0);
                        next.head.orientation = Quaternion::rotation_z(-test * 0.8);
                        next.chest.orientation = Quaternion::rotation_x(test * 0.15)
                            * Quaternion::rotation_y(movement * 0.3)
                            * Quaternion::rotation_z(movement * 1.5);
                        next.belt.orientation = Quaternion::rotation_z(test2 * 0.5);
                        next.shorts.orientation = Quaternion::rotation_z(test2 * 1.5);
                        next.torso.orientation = Quaternion::rotation_z(test2 * 7.2);
                    },
                    StageSection::Recover => {
                        //println!("{:.3} recover", anim_time);
                        next.control.position = Vec3::new(
                            -8.0,
                            11.0 - recover * 0.8 + movement * -10.0,
                            6.0 - recover * 0.4 + movement * -4.0,
                        );
                        next.control.orientation = Quaternion::rotation_x(-1.57)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(1.0);
                        next.chest.orientation = Quaternion::rotation_y(movement * -0.1)
                            * Quaternion::rotation_z(movement * 0.4);
                        next.head.orientation = Quaternion::rotation_y(movement * 0.1)
                            * Quaternion::rotation_z(movement * -0.1);
                    },
                    StageSection::Combo => {
                        //println!("{:.3} combo", anim_time);
                    },
                }
            }
        }
        //        println!("{:?}", stage_progress),

        if let Some(ToolKind::Axe(_) | ToolKind::Hammer(_) | ToolKind::Dagger(_)) = active_tool_kind
        {
            //INTENTION: SWORD
            next.l_hand.position = Vec3::new(-0.75, -1.0, -2.5);
            next.l_hand.orientation = Quaternion::rotation_x(1.27);
            next.l_hand.scale = Vec3::one() * 1.04;
            next.r_hand.position = Vec3::new(0.75, -1.5, -5.5);
            next.r_hand.orientation = Quaternion::rotation_x(1.27);
            next.r_hand.scale = Vec3::one() * 1.05;
            next.main.position = Vec3::new(0.0, 6.0, -1.0);
            next.main.orientation = Quaternion::rotation_x(-0.3)
                * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(0.0);
            next.main.scale = Vec3::one();

            next.control.position = Vec3::new(-4.5 + spinhalf * 4.0, 11.0, 8.0);
            next.control.orientation = Quaternion::rotation_x(-1.7)
                * Quaternion::rotation_y(0.2 + spin * -2.0)
                * Quaternion::rotation_z(1.4 + spin * 0.1);
            next.control.scale = Vec3::one();
            next.head.position = Vec3::new(
                0.0,
                -2.0 + skeleton_attr.head.0 + spin * -0.8,
                skeleton_attr.head.1,
            );
            next.head.orientation = Quaternion::rotation_z(spin * -0.25)
                * Quaternion::rotation_x(0.0 + spin * -0.1)
                * Quaternion::rotation_y(spin * -0.2);
            next.chest.position = Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1);
            next.chest.orientation = Quaternion::rotation_z(spin * 0.1)
                * Quaternion::rotation_x(0.0 + spin * 0.1)
                * Quaternion::rotation_y(decel * -0.2);
            next.chest.scale = Vec3::one();

            next.belt.position = Vec3::new(0.0, 0.0, -2.0);
            next.belt.orientation = next.chest.orientation * -0.1;
            next.belt.scale = Vec3::one();

            next.shorts.position = Vec3::new(0.0, 0.0, -5.0);
            next.belt.orientation = next.chest.orientation * -0.08;
            next.shorts.scale = Vec3::one();
            next.torso.position = Vec3::new(0.0, 0.0, 0.1) * skeleton_attr.scaler;
            next.torso.orientation = Quaternion::rotation_z((spin * 7.0).max(0.3))
                * Quaternion::rotation_x(0.0)
                * Quaternion::rotation_y(0.0);
            next.torso.scale = Vec3::one() / 11.0 * skeleton_attr.scaler;

            next.l_foot.position =
                Vec3::new(-skeleton_attr.foot.0, foot * 1.0, skeleton_attr.foot.2);
            next.l_foot.orientation = Quaternion::rotation_x(foot * -1.2);
            next.l_foot.scale = Vec3::one();

            next.r_foot.position =
                Vec3::new(skeleton_attr.foot.0, foot * -1.0, skeleton_attr.foot.2);
            next.r_foot.orientation = Quaternion::rotation_x(foot * 1.2);
            next.r_foot.scale = Vec3::one();

            next.l_shoulder.position = Vec3::new(-5.0, 0.0, 4.7);
            next.l_shoulder.orientation = Quaternion::rotation_x(0.0);
            next.l_shoulder.scale = Vec3::one() * 1.1;

            next.r_shoulder.position = Vec3::new(5.0, 0.0, 4.7);
            next.r_shoulder.orientation = Quaternion::rotation_x(0.0);
            next.r_shoulder.scale = Vec3::one() * 1.1;

            next.glider.position = Vec3::new(0.0, 5.0, 0.0);
            next.glider.orientation = Quaternion::rotation_y(0.0);
            next.glider.scale = Vec3::one() * 0.0;

            next.lantern.position = Vec3::new(
                skeleton_attr.lantern.0,
                skeleton_attr.lantern.1,
                skeleton_attr.lantern.2,
            );
            next.lantern.orientation =
                Quaternion::rotation_x(spin * -0.7 + 0.4) * Quaternion::rotation_y(spin * 0.4);
            next.lantern.scale = Vec3::one() * 0.65;
            next.hold.scale = Vec3::one() * 0.0;

            next.l_control.position = Vec3::new(0.0, 0.0, 0.0);
            next.l_control.orientation = Quaternion::rotation_x(0.0);
            next.l_control.scale = Vec3::one();

            next.r_control.position = Vec3::new(0.0, 0.0, 0.0);
            next.r_control.orientation = Quaternion::rotation_x(0.0);
            next.r_control.scale = Vec3::one();
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
