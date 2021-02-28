use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};
use std::f32::consts::PI;

pub struct AlphaAnimation;

impl Animation for AlphaAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        f32,
        f32,
        Option<StageSection>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_alpha\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_alpha")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, _second_tool_kind, _velocity, _global_time, stage_section): Self::Dependency,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (move1, move2, move3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
            Some(StageSection::Swing) => (1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
            _ => (0.0, 0.0, 0.0),
        };

        next.torso.position = Vec3::new(0.0, 0.0, 0.1) * s_a.scaler;
        next.torso.orientation = Quaternion::rotation_z(0.0);
        match active_tool_kind {
            Some(ToolKind::Sword) | Some(ToolKind::SwordSimple) => {
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position = Vec3::new(s_a.shr.0, s_a.shr.1, s_a.shr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.shr.3) * Quaternion::rotation_y(s_a.shr.4);

                next.control.position = Vec3::new(
                    s_a.sc.0,
                    s_a.sc.1 + move1 * -4.0 + move2 * 16.0 + move3 * -4.0,
                    s_a.sc.2 + move1 * 1.0,
                );
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3 + move1 * -0.5)
                    * Quaternion::rotation_y(s_a.sc.4 + move1 * -1.0 + move2 * -0.6 + move3 * 1.0)
                    * Quaternion::rotation_z(s_a.sc.5 + move1 * -1.2 + move2 * 1.3);

                next.chest.orientation =
                    Quaternion::rotation_z(move1 * 1.5 + (move2 * 1.75).sin() * -3.0 + move3 * 0.5);

                next.head.position = Vec3::new(0.0 + move2 * 2.0, s_a.head.0, s_a.head.1);
                next.head.orientation = Quaternion::rotation_z(
                    move1 * -0.9 + (move2 * 1.75).sin() * 2.5 + move3 * -0.5,
                );
            },
            Some(ToolKind::Dagger) => {
                next.control_l.position = Vec3::new(-10.0, 6.0, 2.0);
                next.control_l.orientation =
                    Quaternion::rotation_x(-1.4) * Quaternion::rotation_z(1.4);
            },
            Some(ToolKind::Axe) => {
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);
                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.head.position =
                    Vec3::new(0. + move2 * 2.0, s_a.head.0 + move2 * 2.0, s_a.head.1);

                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                    Some(StageSection::Swing) => (1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                    _ => (0.0, 0.0, 0.0),
                };
                next.control.position = Vec3::new(
                    s_a.ac.0 + move1 * -1.0 + move2 * -2.0 + move3 * 0.0,
                    s_a.ac.1 + move1 * -3.0 + move2 * 3.0 + move3 * -3.5,
                    s_a.ac.2 + move1 * 6.0 + move2 * -15.0 + move3 * -2.0,
                );
                next.control.orientation =
                    Quaternion::rotation_x(s_a.ac.3 + move1 * 0.0 + move2 * -3.0 + move3 * 0.4)
                        * Quaternion::rotation_y(
                            s_a.ac.4 + move1 * -0.0 + move2 * -0.6 + move3 * 0.8,
                        )
                        * Quaternion::rotation_z(
                            s_a.ac.5 + move1 * -2.0 + move2 * -1.0 + move3 * 2.5,
                        );
                next.control.scale = Vec3::one();

                next.chest.orientation =
                    Quaternion::rotation_x(0.0 + move1 * 0.6 + move2 * -0.6 + move3 * 0.4)
                        * Quaternion::rotation_y(0.0 + move1 * 0.0 + move2 * 0.0 + move3 * 0.0)
                        * Quaternion::rotation_z(0.0 + move1 * 1.5 + move2 * -2.5 + move3 * 1.5);
                next.head.orientation =
                    Quaternion::rotation_z(0.0 + move1 * -1.5 + move2 * 2.5 + move3 * -1.0);
            },
            Some(ToolKind::Hammer) | Some(ToolKind::HammerSimple) => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                    Some(StageSection::Swing) => (1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                    _ => (0.0, 0.0, 0.0),
                };
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);
                next.hand_l.position = Vec3::new(s_a.hhl.0, s_a.hhl.1, s_a.hhl.2 + move2 * -7.0);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.hhl.3) * Quaternion::rotation_y(s_a.hhl.4);
                next.hand_r.position = Vec3::new(s_a.hhr.0, s_a.hhr.1, s_a.hhr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.hhr.3) * Quaternion::rotation_y(s_a.hhr.4);

                next.control.position = Vec3::new(
                    s_a.hc.0 + (move1 * -13.0) * (1.0 - move3),
                    s_a.hc.1 + (move2 * 5.0) * (1.0 - move3),
                    s_a.hc.2,
                );
                next.control.orientation =
                    Quaternion::rotation_x(s_a.hc.3 + (move1 * 1.5 + move2 * -2.5))
                        * (1.0 - move3)
                        * Quaternion::rotation_y(s_a.hc.4 + (move1 * 1.57))
                        * (1.0 - move3)
                        * Quaternion::rotation_z(s_a.hc.5 + (move2 * -0.5) * (1.0 - move3));
                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
                next.head.orientation =
                    Quaternion::rotation_x((move1 * 0.1 + move2 * 0.3) * (1.0 - move3))
                        * Quaternion::rotation_z((move1 * -0.2 + move2 * 0.2) * (1.0 - move3));
                next.chest.position =
                    Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + move2 * -2.0 * (1.0 - move3));
                next.chest.orientation =
                    Quaternion::rotation_x((move1 * 0.4 + move2 * -0.7) * (1.0 - move3))
                        * Quaternion::rotation_y((move1 * 0.3 + move2 * -0.4) * (1.0 - move3))
                        * Quaternion::rotation_z((move1 * 0.5 + move2 * -0.5) * (1.0 - move3));
            },
            Some(ToolKind::Debug) => {
                next.hand_l.position = Vec3::new(-7.0, 4.0, 3.0);
                next.hand_l.orientation = Quaternion::rotation_x(1.27);
                next.main.position = Vec3::new(-5.0, 5.0, 23.0);
                next.main.orientation = Quaternion::rotation_x(PI);
            },
            _ => {},
        }
        next
    }
}
