use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};
use std::f32::consts::PI;

pub struct ShootAnimation;

type ShootAnimationDependency = (
    Option<ToolKind>,
    Option<ToolKind>,
    f32,
    Vec3<f32>,
    Vec3<f32>,
    f64,
    Option<StageSection>,
);
impl Animation for ShootAnimation {
    type Dependency = ShootAnimationDependency;
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_shoot\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_shoot")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            active_tool_kind,
            _second_tool_kind,
            velocity,
            orientation,
            last_ori,
            _global_time,
            stage_section,
        ): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let speed = Vec2::<f32>::from(velocity).magnitude();

        let mut next = (*skeleton).clone();

        let lab = 1.0;
        let ori: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if ::vek::Vec2::new(ori, last_ori)
            .map(|o| o.magnitude_squared())
            .map(|m| m > 0.001 && m.is_finite())
            .reduce_and()
            && ori.angle_between(last_ori).is_finite()
        {
            ori.angle_between(last_ori).min(0.2)
                * last_ori.determine_side(Vec2::zero(), ori).signum()
        } else {
            0.0
        } * 1.3;

        next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
        next.jaw.orientation = Quaternion::rotation_x(0.0);

        match active_tool_kind {
            Some(ToolKind::Staff) | Some(ToolKind::Sceptre) => {
                let (movement1, movement2, movement3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time as f32, 0.0, 0.0),
                    Some(StageSection::Swing) => (1.0, (anim_time as f32).powf(0.25), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time as f32),
                    _ => (0.0, 0.0, 0.0),
                };
                let xmove = (movement1 as f32 * 6.0 * lab as f32 + PI).sin();
                let ymove = (movement1 as f32 * 6.0 * lab as f32 + PI * (0.5)).sin();
                next.hand_l.position = Vec3::new(s_a.sthl.0, s_a.sthl.1, s_a.sthl.2);
                next.hand_l.orientation = Quaternion::rotation_x(s_a.sthl.3);

                next.hand_r.position = Vec3::new(s_a.sthr.0, s_a.sthr.1, s_a.sthr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.sthr.3) * Quaternion::rotation_y(s_a.sthr.4);
                next.shoulder_l.orientation = Quaternion::rotation_z(0.2)
                    * Quaternion::rotation_x(
                        (movement1 * (movement2 * 8.0 + PI / 2.0).sin() * -0.1) * (1.0 - movement3),
                    );

                next.shoulder_r.orientation = Quaternion::rotation_z(-0.2)
                    * Quaternion::rotation_x(
                        1.0 + movement1 * (movement2 * 8.0 + PI / 2.0).sin() * -0.1,
                    );
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_y(0.0);

                next.control.position = Vec3::new(
                    s_a.stc.0 + (xmove * 3.0 + movement1 * -4.0) * (1.0 - movement3),
                    s_a.stc.1 + (2.0 + ymove * 3.0 + movement2 * 3.0) * (1.0 - movement3),
                    s_a.stc.2,
                );
                next.control.orientation =
                    Quaternion::rotation_x(s_a.stc.3 + (movement2 * 0.6) * (1.0 - movement3))
                        * Quaternion::rotation_y(s_a.stc.4 + (movement1 * 0.5 + movement2 * -0.5))
                        * Quaternion::rotation_z(
                            s_a.stc.5
                                - (0.2 + movement1 * -0.5 + movement2 * 0.8) * (1.0 - movement3),
                        );
                next.upper_torso.orientation =
                    Quaternion::rotation_z((movement1 * 0.3 + movement2 * 0.2) * (1.0 - movement3));
                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
                next.head.orientation = Quaternion::rotation_z(
                    tilt * -2.5 + (movement1 * -0.2 + movement2 * -0.4) * (1.0 - movement3),
                );

                if speed < 0.5 {
                    next.lower_torso.orientation =
                        Quaternion::rotation_x(0.08) * Quaternion::rotation_z(0.0);

                    next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1 - 5.0, s_a.foot.2);
                    next.foot_l.orientation = Quaternion::rotation_x(-0.5);

                    next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1 + 3.0, s_a.foot.2);
                    next.foot_r.orientation =
                        Quaternion::rotation_x(0.5) * Quaternion::rotation_z(0.3);
                } else {
                };
            },
            Some(ToolKind::Bow) => {
                let (movement1, movement2, movement3) = match stage_section {
                    Some(StageSection::Buildup) => ((anim_time as f32).powf(0.25), 0.0, 0.0),
                    Some(StageSection::Swing) => (1.0, anim_time as f32, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, (anim_time as f32).powf(4.0)),
                    _ => (0.0, 0.0, 0.0),
                };
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);
                next.hand_l.position = Vec3::new(
                    s_a.bhl.0 + movement2 * -2.0,
                    s_a.bhl.1 + movement2 * -12.0,
                    s_a.bhl.2 + movement2 * -3.0,
                );
                next.hand_l.orientation = Quaternion::rotation_x(s_a.bhl.3);
                next.hand_r.position = Vec3::new(s_a.bhr.0, s_a.bhr.1, s_a.bhr.2);
                next.hand_r.orientation = Quaternion::rotation_x(s_a.bhr.3);
                next.shoulder_l.orientation = Quaternion::rotation_z(0.2)
                    * Quaternion::rotation_x(
                        (movement1 * (movement2 * 8.0 + PI / 2.0).sin() * 1.1) * (1.0 - movement3),
                    );

                next.shoulder_r.orientation = Quaternion::rotation_z(-0.2)
                    * Quaternion::rotation_x(
                        1.5 + movement1 * (movement2 * 8.0 + PI / 2.0).sin() * -0.1,
                    );
                next.hold.position =
                    Vec3::new(0.0, -1.0 + movement2 * 2.0, -5.2 + movement2 * -3.0);
                next.hold.orientation = Quaternion::rotation_x(-1.57);
                next.hold.scale = Vec3::one() * 1.0 * (1.0 - movement2);

                next.control.position = Vec3::new(s_a.bc.0 + 11.0, s_a.bc.1 + 2.0, s_a.bc.2 + 8.0);
                next.control.orientation =
                    Quaternion::rotation_x(0.0 + (movement2 as f32 * 0.1).sin())
                        * Quaternion::rotation_y(s_a.bc.4 - 1.25)
                        * Quaternion::rotation_z(s_a.bc.5 - 0.2 + (movement2 as f32 * -0.2).sin());
                next.upper_torso.orientation = Quaternion::rotation_z(0.8);
                next.head.position = Vec3::new(0.0 - 2.0, s_a.head.0, s_a.head.1);

                next.head.orientation =
                    Quaternion::rotation_z(tilt * -2.5 - 0.5 + (movement2 as f32 * 0.2).sin());
                if speed < 0.5 {
                    next.upper_torso.orientation =
                        Quaternion::rotation_z(0.8 + (movement2 as f32 * 0.1).sin());

                    next.lower_torso.orientation = Quaternion::rotation_x(0.08)
                        * Quaternion::rotation_z((movement2 as f32 * -0.15).sin());

                    next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1 - 5.0, s_a.foot.2);
                    next.foot_l.orientation = Quaternion::rotation_x(-0.5);

                    next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1 + 3.0, s_a.foot.2);
                    next.foot_r.orientation =
                        Quaternion::rotation_x(0.5) * Quaternion::rotation_z(0.3);
                } else {
                };
            },
            _ => {},
        }

        next
    }
}
