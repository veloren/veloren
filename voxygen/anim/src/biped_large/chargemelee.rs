use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::{
    comp::item::tool::{AbilitySpec, ToolKind},
    states::utils::StageSection,
};
use core::f32::consts::PI;

pub struct ChargeMeleeAnimation;

impl Animation for ChargeMeleeAnimation {
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
    const UPDATE_FN: &'static [u8] = b"biped_large_chargemelee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_chargemelee")]
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
        let speed = Vec2::<f32>::from(velocity).magnitude();

        let lab: f32 = 0.65 * s_a.tempo;
        let speednorm = (speed / 12.0).powf(0.4);
        let foothoril = (acc_vel * lab + PI * 1.45).sin() * speednorm;
        let foothorir = (acc_vel * lab + PI * (0.45)).sin() * speednorm;
        let footrotl = ((1.0 / (0.5 + (0.5) * ((acc_vel * lab + PI * 1.4).sin()).powi(2))).sqrt())
            * ((acc_vel * lab + PI * 1.4).sin());

        let footrotr = ((1.0 / (0.5 + (0.5) * ((acc_vel * lab + PI * 0.4).sin()).powi(2))).sqrt())
            * ((acc_vel * lab + PI * 0.4).sin());
        let (move1base, move2base, movement3, tension) = match stage_section {
            Some(StageSection::Charge) => (
                (anim_time.powf(0.25)).min(1.0),
                0.0,
                0.0,
                (anim_time * 100.0).sin(),
            ),
            Some(StageSection::Action) => (1.0, anim_time.powf(0.25), 0.0, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4), 0.0),
            _ => (0.0, 0.0, 0.0, 0.0),
        };

        let pullback = 1.0 - movement3;
        let move1 = move1base * pullback;
        let move2 = move2base * pullback;
        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation = Quaternion::rotation_x(0.0);
        next.shoulder_l.position = Vec3::new(
            -s_a.shoulder.0,
            s_a.shoulder.1,
            s_a.shoulder.2 - foothorir * 1.0,
        );
        next.shoulder_l.orientation =
            Quaternion::rotation_x(move1 * 0.8 + 0.6 * speednorm + (footrotr * -0.2) * speednorm);

        next.shoulder_r.position = Vec3::new(
            s_a.shoulder.0,
            s_a.shoulder.1,
            s_a.shoulder.2 - foothoril * 1.0,
        );
        next.shoulder_r.orientation =
            Quaternion::rotation_x(move1 * 0.8 + 0.6 * speednorm + (footrotl * -0.2) * speednorm);
        next.torso.orientation = Quaternion::rotation_z(0.0);

        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_x(0.0);

        next.hand_l.position = Vec3::new(0.0, 0.0, s_a.grip.0);
        next.hand_r.position = Vec3::new(0.0, 0.0, s_a.grip.0);

        next.hand_l.orientation = Quaternion::rotation_x(0.0);
        next.hand_r.orientation = Quaternion::rotation_x(0.0);

        #[allow(clippy::single_match)]
        match active_tool_kind {
            Some(ToolKind::Natural) => match ability_id {
                Some("common.abilities.custom.minotaur.cleave") => {
                    next.upper_torso.orientation =
                        Quaternion::rotation_x(move1 * 0.3 + move2 * -0.9);
                    next.lower_torso.orientation =
                        Quaternion::rotation_x(move1 * -0.3 + move2 * 0.9);
                    next.head.orientation = Quaternion::rotation_x(move1 * -0.5 + move2 * 0.5);

                    next.control_l.position = Vec3::new(0.0, 4.0, 5.0);
                    next.control_r.position = Vec3::new(0.0, 4.0, 5.0);
                    next.weapon_l.position = Vec3::new(
                        -12.0 + move2 * 5.0,
                        -6.0 + move1 * 22.0 + move2 * 8.0,
                        -18.0 + move1 * 16.0 + move2 * -19.0,
                    );
                    next.weapon_r.position = Vec3::new(
                        12.0 + move2 * -5.0,
                        -6.0 + move1 * 22.0 + move2 * 8.0,
                        -18.0 + move1 * 14.0 + move2 * -19.0,
                    );
                    next.torso.position = Vec3::new(0.0, move2 * 7.06, 0.0);
                    next.second.scale = Vec3::one() * 1.0;

                    next.weapon_l.orientation =
                        Quaternion::rotation_x(-1.67 + move1 * 2.8 + tension * 0.03 + move2 * -2.3)
                            * Quaternion::rotation_y(move1 * 0.3 + move2 * 0.5);
                    next.weapon_r.orientation = Quaternion::rotation_x(
                        -1.67 + move1 * 1.6 + tension * -0.03 + move2 * -0.7,
                    ) * Quaternion::rotation_y(
                        move1 * -0.3 + move2 * -0.5,
                    ) * Quaternion::rotation_z(0.0);

                    next.control_l.orientation =
                        Quaternion::rotation_x(PI / 2.0 + move1 * 0.2 + move2 * 0.1);
                    next.control_r.orientation =
                        Quaternion::rotation_x(PI / 2.0 + move1 * 0.4 + move2 * -0.4);

                    next.control.orientation =
                        Quaternion::rotation_x(0.0) * Quaternion::rotation_y(0.0);
                    next.shoulder_l.orientation = Quaternion::rotation_x(-0.3 + move1 * 1.0);

                    next.shoulder_r.orientation = Quaternion::rotation_x(-0.3 + move1 * 1.0);
                },
                Some("common.abilities.custom.husk_brute.chargedmelee") => {
                    next.second.scale = Vec3::one() * 0.0;

                    next.head.orientation = Quaternion::rotation_x(move1 * 0.3 + move2 * -0.6);
                    next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
                    next.jaw.orientation = Quaternion::rotation_x(move2 * -0.3);
                    next.control_l.position = Vec3::new(-0.5, 4.0, 1.0);
                    next.control_r.position = Vec3::new(-0.5, 4.0, 1.0);
                    next.control_l.orientation = Quaternion::rotation_x(PI / 2.0);
                    next.control_r.orientation = Quaternion::rotation_x(PI / 2.0);
                    next.weapon_l.position =
                        Vec3::new(-12.0 + (move1 * 10.0).min(6.0), -1.0, -15.0);
                    next.weapon_r.position =
                        Vec3::new(12.0 + (move1 * -10.0).max(-6.0), -1.0, -15.0);

                    next.weapon_l.orientation = Quaternion::rotation_x(-PI / 2.0 - 0.1)
                        * Quaternion::rotation_z(move1 * -0.8);
                    next.weapon_r.orientation = Quaternion::rotation_x(-PI / 2.0 - 0.1)
                        * Quaternion::rotation_z(move1 * 0.8);

                    next.shoulder_l.orientation =
                        Quaternion::rotation_x(-0.3 + move1 * 2.8 + move2 * -2.8);

                    next.shoulder_r.orientation =
                        Quaternion::rotation_x(-0.3 + move1 * 2.8 + move2 * -2.8);

                    next.control.orientation = Quaternion::rotation_x(move1 * 2.5 + move2 * -2.0);

                    next.upper_torso.position =
                        Vec3::new(0.0, s_a.upper_torso.0, s_a.upper_torso.1);
                    next.upper_torso.orientation =
                        Quaternion::rotation_x(move1 * 0.2 + move2 * -0.6);
                    next.lower_torso.orientation =
                        Quaternion::rotation_x(move1 * -0.2 + move2 * 0.6);

                    if speed < 0.1 {
                        next.foot_l.position = Vec3::new(
                            -s_a.foot.0,
                            s_a.foot.1 + move1 * -7.0 + move2 * 7.0,
                            s_a.foot.2,
                        );
                        next.foot_l.orientation =
                            Quaternion::rotation_x(move1 * -0.8 + move2 * 0.8)
                                * Quaternion::rotation_z(move1 * 0.3 + move2 * -0.3);

                        next.foot_r.position = Vec3::new(
                            s_a.foot.0,
                            s_a.foot.1 + move1 * 5.0 + move2 * -5.0,
                            s_a.foot.2,
                        );
                        next.foot_r.orientation =
                            Quaternion::rotation_y(move1 * -0.3 + move2 * 0.3)
                                * Quaternion::rotation_z(move1 * 0.4 + move2 * -0.4);
                    }
                    next.main.orientation = Quaternion::rotation_y(move1 * 0.4 + move2 * -0.6)
                        * Quaternion::rotation_x(move2 * -0.4);
                },
                _ => {},
            },
            _ => {},
        }

        next
    }
}
