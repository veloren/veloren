use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{AbilitySpec, ToolKind},
    states::utils::StageSection,
};
use core::f32::consts::PI;

pub struct ShockwaveAnimation;

type ShockwaveAnimationDependency<'a> = (
    Option<ToolKind>,
    (Option<ToolKind>, Option<&'a AbilitySpec>),
    f32,
    f32,
    Option<StageSection>,
    Option<&'a str>,
);
impl Animation for ShockwaveAnimation {
    type Dependency<'a> = ShockwaveAnimationDependency<'a>;
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_shockwave\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_shockwave")]
    #[allow(clippy::single_match)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            active_tool_kind,
            _second_tool_kind,
            _global_time,
            velocity,
            stage_section,
            ability_id,
        ): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (move1, move1pow, move2, move3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time, anim_time.powf(0.25), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, 1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0, 0.0),
        };

        let pullback = 1.0 - move3;
        let move1pow = move1pow * pullback;
        let move2 = move2 * pullback;

        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_x(0.0);

        next.hand_l.position = Vec3::new(0.0, 0.0, s_a.grip.0);
        next.hand_r.position = Vec3::new(0.0, 0.0, s_a.grip.0);

        next.hand_l.orientation = Quaternion::rotation_x(0.0);
        next.hand_r.orientation = Quaternion::rotation_x(0.0);

        match active_tool_kind {
            Some(ToolKind::Sceptre | ToolKind::Staff) => {
                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

                next.hand_l.position = Vec3::new(s_a.sthl.0, s_a.sthl.1, s_a.sthl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.sthl.3) * Quaternion::rotation_y(s_a.sthl.4);
                next.hand_r.position = Vec3::new(s_a.sthr.0, s_a.sthr.1, s_a.sthr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.sthr.3) * Quaternion::rotation_y(s_a.sthr.4);
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_y(0.0);

                next.control.position = Vec3::new(s_a.stc.0, s_a.stc.1, s_a.stc.2);
                next.control.orientation =
                    Quaternion::rotation_x(s_a.stc.3) * Quaternion::rotation_y(s_a.stc.4);

                let twist = move1 * 0.8;

                next.control.position = Vec3::new(
                    s_a.stc.0 + move1 * 5.0 + move3 * -5.0,
                    s_a.stc.1 + move1 * 13.0 + move3 * -3.0,
                    s_a.stc.2 + move1 * 10.0 + move2 * -2.0 + move3 * -8.0,
                );
                next.control.orientation =
                    Quaternion::rotation_x(s_a.stc.3 + move1 * 0.8 + move2 * 0.3 + move3 * -1.1)
                        * Quaternion::rotation_y(
                            s_a.stc.4 + move1 * -0.15 + move2 * 0.3 + move3 * -0.45,
                        )
                        * Quaternion::rotation_z(move1 * 0.8 + move2 * -0.8);

                next.head.orientation = Quaternion::rotation_x(move1 * 0.4 + move3 * -0.4)
                    * Quaternion::rotation_z(twist * 0.2 + move2 * -0.8 + move3 * 0.6);

                next.upper_torso.position = Vec3::new(
                    0.0,
                    s_a.upper_torso.0,
                    s_a.upper_torso.1 + move1 * 2.0 + move2 * -4.0 + move3 * 2.0,
                );
                next.upper_torso.orientation = Quaternion::rotation_x(move2 * -0.8 + move3 * 0.8)
                    * Quaternion::rotation_z(twist * -0.2 + move2 * -0.1 + move3 * 0.3);

                next.lower_torso.orientation = Quaternion::rotation_x(move2 * 0.3 + move3 * -0.3)
                    * Quaternion::rotation_z(twist + move2 * -0.8);

                if velocity < 0.5 {
                    next.foot_l.position = Vec3::new(
                        -s_a.foot.0,
                        s_a.foot.1 + move1 * -7.0 + move2 * 7.0,
                        s_a.foot.2,
                    );
                    next.foot_l.orientation = Quaternion::rotation_x(move1 * -0.8 + move2 * 0.8)
                        * Quaternion::rotation_z(move1 * 0.3 + move2 * -0.3);

                    next.foot_r.position = Vec3::new(
                        s_a.foot.0,
                        s_a.foot.1 + move1 * 5.0 + move2 * -5.0,
                        s_a.foot.2,
                    );
                    next.foot_r.orientation = Quaternion::rotation_y(move1 * -0.3 + move2 * 0.3)
                        * Quaternion::rotation_z(move1 * 0.4 + move2 * -0.4);
                }
            },
            Some(ToolKind::Natural) => match ability_id {
                Some("common.abilities.custom.yeti.icespikes") => {
                    next.second.scale = Vec3::one() * 0.0;

                    next.head.orientation = Quaternion::rotation_x(move1pow * 0.8 + move2 * -1.2);
                    next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
                    next.jaw.orientation = Quaternion::rotation_x(move2 * -0.3);
                    next.control_l.position = Vec3::new(-0.5, 4.0, 1.0);
                    next.control_r.position = Vec3::new(-0.5, 4.0, 1.0);
                    next.control_l.orientation = Quaternion::rotation_x(PI / 2.0);
                    next.control_r.orientation = Quaternion::rotation_x(PI / 2.0);
                    next.weapon_l.position =
                        Vec3::new(-12.0 + (move1pow * 20.0).min(10.0), -1.0, -15.0);
                    next.weapon_r.position =
                        Vec3::new(12.0 + (move1pow * -20.0).max(-10.0), -1.0, -15.0);

                    next.weapon_l.orientation = Quaternion::rotation_x(-PI / 2.0 - 0.1)
                        * Quaternion::rotation_z(move1pow * -1.0);
                    next.weapon_r.orientation = Quaternion::rotation_x(-PI / 2.0 - 0.1)
                        * Quaternion::rotation_z(move1pow * 1.0);

                    next.shoulder_l.orientation =
                        Quaternion::rotation_x(-0.3 + move1pow * 2.8 + move2 * -2.8);

                    next.shoulder_r.orientation =
                        Quaternion::rotation_x(-0.3 + move1pow * 2.8 + move2 * -2.8);

                    next.control.orientation =
                        Quaternion::rotation_x(move1pow * 2.5 + move2 * -2.0);

                    let twist = move1 * 0.6 + move3 * -0.6;
                    next.upper_torso.position =
                        Vec3::new(0.0, s_a.upper_torso.0, s_a.upper_torso.1);
                    next.upper_torso.orientation =
                        Quaternion::rotation_x(move1pow * 0.8 + move2 * -1.1)
                            * Quaternion::rotation_z(twist * -0.2 + move1 * -0.1 + move2 * 0.3);

                    next.lower_torso.orientation =
                        Quaternion::rotation_x(move1pow * -0.8 + move2 * 1.1)
                            * Quaternion::rotation_z(twist);

                    next.foot_l.position = Vec3::new(
                        -s_a.foot.0,
                        s_a.foot.1 + move1pow * -7.0 + move2 * 7.0,
                        s_a.foot.2,
                    );
                    next.foot_l.orientation = Quaternion::rotation_x(move1pow * -0.8 + move2 * 0.8)
                        * Quaternion::rotation_z(move1pow * 0.3 + move2 * -0.3);

                    next.foot_r.position = Vec3::new(
                        s_a.foot.0,
                        s_a.foot.1 + move1pow * 5.0 + move2 * -5.0,
                        s_a.foot.2,
                    );
                    next.foot_r.orientation = Quaternion::rotation_y(move1pow * -0.3 + move2 * 0.3)
                        * Quaternion::rotation_z(move1pow * 0.4 + move2 * -0.4);

                    next.main.orientation = Quaternion::rotation_y(move1 * 0.4 + move2 * -0.6)
                        * Quaternion::rotation_x(move2 * -0.4);
                },
                _ => {},
            },
            Some(ToolKind::Axe) => match ability_id {
                Some("common.abilities.custom.gigas_frost.flashfreeze") => {
                    next.second.scale = Vec3::one() * 0.0;

                    next.head.orientation = Quaternion::rotation_x(move1pow * 0.8 + move2 * -1.2);
                    next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
                    next.jaw.orientation = Quaternion::rotation_x(move2 * -0.3);
                    next.control_l.position = Vec3::new(-0.5, 4.0, 1.0);
                    next.control_r.position = Vec3::new(-0.5, 4.0, 1.0);
                    next.control_l.orientation = Quaternion::rotation_x(PI / 2.0);
                    next.control_r.orientation = Quaternion::rotation_x(PI / 2.0);
                    next.weapon_l.position =
                        Vec3::new(-12.0 + (move1pow * 20.0).min(10.0), -1.0, -15.0);
                    next.weapon_r.position =
                        Vec3::new(12.0 + (move1pow * -20.0).max(-10.0), -1.0, -15.0);

                    next.weapon_l.orientation = Quaternion::rotation_x(-PI / 2.0 - 0.1)
                        * Quaternion::rotation_z(move1pow * -1.0);
                    next.weapon_r.orientation = Quaternion::rotation_x(-PI / 2.0 - 0.1)
                        * Quaternion::rotation_z(move1pow * 1.0);

                    next.shoulder_l.orientation =
                        Quaternion::rotation_x(-0.3 + move1pow * 2.8 + move2 * -2.8);

                    next.shoulder_r.orientation =
                        Quaternion::rotation_x(-0.3 + move1pow * 2.8 + move2 * -2.8);

                    next.control.orientation = Quaternion::rotation_x(move1pow * 1.5 + move2 * 0.6);

                    let twist = move1 * 0.6 + move3 * -0.6;
                    next.upper_torso.position =
                        Vec3::new(0.0, s_a.upper_torso.0, s_a.upper_torso.1);
                    next.upper_torso.orientation =
                        Quaternion::rotation_x(move1pow * 0.4 + move2 * -1.1)
                            * Quaternion::rotation_z(twist * -0.2 + move1 * -0.1 + move2 * 0.3);

                    next.lower_torso.orientation =
                        Quaternion::rotation_x(move1pow * -0.4 + move2 * 1.1)
                            * Quaternion::rotation_z(twist);

                    next.foot_l.position = Vec3::new(
                        -s_a.foot.0,
                        s_a.foot.1 + move1pow * -7.0 + move2 * 7.0,
                        s_a.foot.2,
                    );
                    next.foot_l.orientation = Quaternion::rotation_x(move1pow * -0.8 + move2 * 0.8)
                        * Quaternion::rotation_z(move1pow * 0.3 + move2 * -0.3);

                    next.foot_r.position = Vec3::new(
                        s_a.foot.0,
                        s_a.foot.1 + move1pow * 5.0 + move2 * -5.0,
                        s_a.foot.2,
                    );
                    next.foot_r.orientation = Quaternion::rotation_y(move1pow * -0.3 + move2 * 0.3)
                        * Quaternion::rotation_z(move1pow * 0.4 + move2 * -0.4);
                    next.main.position = Vec3::new(-5.0 + (move1pow * 20.0).min(10.0), 4.0, 12.0);
                    next.main.orientation = Quaternion::rotation_y(move1 * 0.4 + move2 * -0.1)
                        * Quaternion::rotation_x(PI);
                },
                _ => {},
            },
            _ => {},
        }
        next
    }
}
