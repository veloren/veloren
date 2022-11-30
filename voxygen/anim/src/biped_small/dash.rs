use super::{
    super::{vek::*, Animation},
    BipedSmallSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
use std::f32::consts::PI;

pub struct DashAnimation;

type DashAnimationDependency = (
    Vec3<f32>,
    Vec3<f32>,
    Vec3<f32>,
    f32,
    Vec3<f32>,
    f32,
    Option<StageSection>,
    f32,
);

impl Animation for DashAnimation {
    type Dependency<'a> = DashAnimationDependency;
    type Skeleton = BipedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_small_dash\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_small_dash")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            velocity,
            _orientation,
            _last_ori,
            _global_time,
            _avg_vel,
            _acc_vel,
            stage_section,
            _timer,
        ): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();

        let fast = (anim_time * 10.0).sin();
        let fastalt = (anim_time * 10.0 + PI / 2.0).sin();

        let speednorm = speed / 9.4;
        let speednormcancel = 1.0 - speednorm;

        let (move1base, move2base, move3, move4) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.sqrt(), 0.0, 0.0, 0.0),
            Some(StageSection::Charge) => (1.0, anim_time.powi(4), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, 1.0, anim_time.powi(4), 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - move4;
        let move1abs = move1base * pullback;
        let move2abs = move2base * pullback;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_x(move1abs * 0.6)
            * Quaternion::rotation_z(move1abs * -0.0)
            * Quaternion::rotation_y(move1abs * 0.3);
        next.chest.orientation = Quaternion::rotation_x(move1abs * -0.8);

        next.pants.orientation = Quaternion::rotation_x(move1abs * -0.2);

        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_x(0.0);

        next.hand_l.position = Vec3::new(s_a.grip.0 * 4.0, 0.0, s_a.grip.2);
        next.hand_r.position = Vec3::new(-s_a.grip.0 * 4.0, 0.0, s_a.grip.2);

        next.hand_l.orientation = Quaternion::rotation_x(0.0);
        next.hand_r.orientation = Quaternion::rotation_x(0.0);

        next.control_l.position = Vec3::new(1.0 - s_a.grip.0 * 2.0, 2.0, -2.0);
        next.control_r.position = Vec3::new(-1.0 + s_a.grip.0 * 2.0, 2.0, 2.0);

        next.control.position = Vec3::new(
            -3.0,
            s_a.grip.2 + move1abs * -5.0,
            -s_a.grip.2 / 2.5 + s_a.grip.0 * -2.0 + move1abs * 4.0,
        );

        next.control_l.orientation =
            Quaternion::rotation_x(PI / 1.5 + move1abs * -0.7 + move3 * 0.7)
                * Quaternion::rotation_y(-0.3);
        next.control_r.orientation =
            Quaternion::rotation_x(PI / 1.5 + s_a.grip.0 * 0.2 + move1abs * -0.7 + move3 * 0.7)
                * Quaternion::rotation_y(0.5 + s_a.grip.0 * 0.2);

        next.control.orientation = Quaternion::rotation_x(-1.35 + move1abs * 0.6)
            * Quaternion::rotation_z(move1abs * 0.2)
            * Quaternion::rotation_y(move2abs * 0.0);

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.tail.orientation = Quaternion::rotation_x(0.05 * fastalt * speednormcancel)
            * Quaternion::rotation_z(fast * 0.15 * speednormcancel);

        next
    }
}
