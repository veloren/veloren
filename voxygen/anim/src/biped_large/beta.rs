use super::{
    super::{Animation, vek::*},
    BipedLargeSkeleton, SkeletonAttr, biped_large_beta_axe, biped_large_beta_hammer,
    biped_large_beta_sword, init_biped_large_beta,
};
use common::{
    comp::item::tool::{AbilitySpec, ToolKind},
    states::utils::StageSection,
};
use std::f32::consts::PI;

pub struct BetaAnimation;

impl Animation for BetaAnimation {
    type Dependency<'a> = (
        Option<ToolKind>,
        (Option<ToolKind>, Option<&'a AbilitySpec>),
        Vec3<f32>,
        f32,
        Option<StageSection>,
        f32,
        Option<&'a str>,
    );
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_beta\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "biped_large_beta"))]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            active_tool_kind,
            _second_tool,
            velocity,
            _global_time,
            stage_section,
            acc_vel,
            ability_id,
        ): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (move1base, move2base, move3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(6)),
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - move3;
        let move1 = move1base * pullback;
        let move2 = move2base * pullback;

        let speed = Vec2::<f32>::from(velocity).magnitude();

        init_biped_large_beta(&mut next, s_a, speed, acc_vel, move1);

        match active_tool_kind {
            Some(ToolKind::Sword) => {
                biped_large_beta_sword(&mut next, s_a, move1base, move1, move2);
            },
            Some(ToolKind::Hammer) => {
                biped_large_beta_hammer(&mut next, s_a, move1, move2);
            },
            Some(ToolKind::Axe) => {
                biped_large_beta_axe(&mut next, s_a, move1, move2);
            },
            Some(ToolKind::Natural) => match ability_id {
                Some("common.abilities.custom.wendigomagic.singlestrike") => {
                    next.torso.position = Vec3::new(0.0, 0.0, move1 * -2.18);
                    next.upper_torso.orientation =
                        Quaternion::rotation_x(move1 * -0.5 + move2 * -0.4);
                    next.lower_torso.orientation =
                        Quaternion::rotation_x(move1 * 0.5 + move2 * 0.4);

                    next.control_l.position =
                        Vec3::new(-9.0 + move2 * 6.0, 19.0 + move1 * 6.0, -13.0 + move1 * 10.5);
                    next.control_r.position =
                        Vec3::new(9.0 + move2 * -6.0, 19.0 + move1 * 6.0, -13.0 + move1 * 14.5);

                    next.control_l.orientation = Quaternion::rotation_x(PI / 3.0 + move1 * 0.5)
                        * Quaternion::rotation_y(-0.15)
                        * Quaternion::rotation_z(move1 * 0.5 + move2 * -0.6);
                    next.control_r.orientation = Quaternion::rotation_x(PI / 3.0 + move1 * 0.5)
                        * Quaternion::rotation_y(0.15)
                        * Quaternion::rotation_z(move1 * -0.5 + move2 * 0.6);
                    next.head.orientation = Quaternion::rotation_x(move1 * 0.3);
                },
                _ => {},
            },
            _ => {},
        }
        next
    }
}
