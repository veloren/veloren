use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};
use core::f32::consts::{PI, TAU};

pub struct SpinMeleeAnimation;

impl Animation for SpinMeleeAnimation {
    type Dependency<'a> = (
        Option<ToolKind>,
        Option<ToolKind>,
        Vec3<f32>,
        f32,
        Option<StageSection>,
        f32,
    );
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_spinmelee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_spinmelee")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, _second_tool_kind, velocity, _global_time, stage_section, acc_vel): Self::Dependency<'_>,
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

        let (move1base, move2, move3) = match stage_section {
            Some(StageSection::Buildup) => ((anim_time.powf(0.25)), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, (anim_time * 0.05).sin() - 0.05, 0.0),
            Some(StageSection::Recover) => (1.0, 0.0, anim_time),
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - move3;
        let move1 = move1base * pullback;

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

        next.hand_l.position = Vec3::new(s_a.grip.1, 0.0, s_a.grip.0);
        next.hand_r.position = Vec3::new(-s_a.grip.1, 0.0, s_a.grip.0);

        next.hand_l.orientation = Quaternion::rotation_x(0.0);
        next.hand_r.orientation = Quaternion::rotation_x(0.0);

        #[allow(clippy::single_match)]
        match active_tool_kind {
            Some(ToolKind::Staff) => {
                next.head.orientation = Quaternion::rotation_x(move1 * -0.3 + move2 * 0.5);
                next.control_l.position = Vec3::new(
                    -1.0 + move1 * -10.0 + move2 * -10.0,
                    3.0,
                    12.0 + move1 * 7.0,
                );
                next.control_r.position = Vec3::new(
                    1.0 + move1 * 10.0 + move2 * -10.0,
                    2.0 + move1 * -0.0,
                    2.0 + move1 * 15.0,
                );

                next.control.position = Vec3::new(
                    -3.0 + move1 * 3.0,
                    3.0 + s_a.grip.0 / 1.2 + move1 * 18.0,
                    -11.0 + -s_a.grip.0 / 2.0 + move1 * 8.0,
                );

                next.control_l.orientation = Quaternion::rotation_x(PI / 2.0 - move1 * 0.2)
                    * Quaternion::rotation_y(-0.5 + move1 * 0.3)
                    * Quaternion::rotation_z(move1 * 0.0);
                next.control_r.orientation = Quaternion::rotation_x(PI / 2.5 + move1 * 0.0)
                    * Quaternion::rotation_y(0.5 + move1 * -0.3)
                    * Quaternion::rotation_z(move2 * 1.0);

                next.control.orientation = Quaternion::rotation_x(-0.2 + move1 * 0.8)
                    * Quaternion::rotation_y(-0.1 + move1 * 0.1 + move2 * 1.0);

                next.upper_torso.orientation = Quaternion::rotation_x(move1 * -0.5);

                next.lower_torso.orientation = Quaternion::rotation_x(move1 * 0.8);
                next.torso.position = Vec3::new(0.0, 0.0, move1 * 6.4);
            },
            Some(ToolKind::Axe) => {
                let (move1base, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => ((anim_time.powf(0.25)), 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, (anim_time), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1base * pullback;
                let move2ret = move2 * pullback;

                next.shoulder_r.orientation =
                    Quaternion::rotation_y(move1 * 0.5) * Quaternion::rotation_x(move1 * 0.5);
                next.head.orientation = Quaternion::rotation_x(move2ret * 0.7)
                    * Quaternion::rotation_y(move1 * 0.3)
                    * Quaternion::rotation_z(move1 * -0.2 - move2ret * 0.5);

                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);

                next.hand_l.position = Vec3::new(s_a.grip.1, 0.0, s_a.grip.0);
                next.hand_r.position = Vec3::new(-s_a.grip.1, 0.0, s_a.grip.0);

                next.hand_l.orientation = Quaternion::rotation_x(0.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.0);

                next.control_l.position = Vec3::new(-1.0, 2.0, 12.0);
                next.control_r.position = Vec3::new(1.0, 2.0, -2.0);

                next.control.position = Vec3::new(
                    4.0 + move1 * -25.0,
                    0.0 + s_a.grip.0 / 1.0 + move1 * -TAU,
                    -s_a.grip.0 / 0.8 + move1 * 10.0,
                );

                next.control_l.orientation =
                    Quaternion::rotation_x(PI / 2.0 + move1 * 0.3) * Quaternion::rotation_y(-0.0);
                next.control_r.orientation = Quaternion::rotation_x(PI / 2.0 + 0.2)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);
                next.control.orientation = Quaternion::rotation_x(-1.0 + move1 * -1.2)
                    * Quaternion::rotation_y(-1.8 + move1 * -1.0)
                    * Quaternion::rotation_z(0.0 + move1 * -1.5);
                next.upper_torso.orientation = Quaternion::rotation_y(move1 * -0.4);

                next.lower_torso.orientation = Quaternion::rotation_y(move1 * 0.5);
                next.torso.position = Vec3::new(move1 * -2.0, 0.0, 0.0);

                next.leg_l.orientation = Quaternion::rotation_x(0.0);
                next.leg_r.orientation = Quaternion::rotation_x(move1 * -0.5);
                next.foot_r.orientation =
                    Quaternion::rotation_x(move1 * -0.5) * Quaternion::rotation_y(move1 * -0.2);
                next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1 + move1 * -3.0, s_a.foot.2);
                next.torso.orientation = Quaternion::rotation_z(move2 * -TAU);
            },
            _ => {},
        }

        next
    }
}
