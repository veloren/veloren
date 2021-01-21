use super::{
    super::{vek::*, Animation},
    BipedSmallSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
use std::f32::consts::PI;

pub struct AlphaAnimation;

type AlphaAnimationDependency = (
    Vec3<f32>,
    Vec3<f32>,
    Vec3<f32>,
    f64,
    Vec3<f32>,
    f32,
    Option<StageSection>,
    f64,
);

impl Animation for AlphaAnimation {
    type Dependency = AlphaAnimationDependency;
    type Skeleton = BipedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_small_alpha\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_small_alpha")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, _orientation, _last_ori, global_time, _avg_vel, acc_vel, stage_section, timer): Self::Dependency,
        anim_time: f64,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();

        let fastacc = (acc_vel * 2.0).sin();
        let fast = (anim_time as f32 * 10.0).sin();
        let fastalt = (anim_time as f32 * 10.0 + PI / 2.0).sin();
        let slow = (anim_time as f32 * 2.0).sin();

        let speednorm = speed / 9.4;
        let speednormcancel = 1.0 - speednorm;

        let (movement1base, movement2base, movement3) = match stage_section {
            Some(StageSection::Buildup) => ((anim_time as f32).sqrt(), 0.0, 0.0),
            Some(StageSection::Swing) => (1.0, (anim_time as f32).powi(4), 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time as f32),
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - movement3;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum() as f32;
        let movement1 = mirror * movement1base * pullback;
        let movement2 = mirror * movement2base * pullback;
        let movement1abs = movement1base * pullback;
        let movement2abs = movement2base * pullback;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_x(movement1abs * -0.1 + movement2abs * 0.5)
            * Quaternion::rotation_z(movement1abs * -0.2 + movement2abs * 0.6)
            * Quaternion::rotation_y(movement1abs * 0.3 + movement2abs * -0.5);
        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1) / 13.0;
        next.chest.orientation = Quaternion::rotation_z(movement1abs * 0.5 + movement2abs * -0.6);

        next.shorts.position = Vec3::new(0.0, s_a.shorts.0, s_a.shorts.1);
        next.shorts.orientation = Quaternion::rotation_z(movement1abs * -0.2 + movement2abs * 0.2);

        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_x(0.0);

        next.hand_l.position = Vec3::new(s_a.grip.0 * 4.0, 0.0, s_a.grip.2);
        next.hand_r.position = Vec3::new(-s_a.grip.0 * 4.0, 0.0, s_a.grip.2);

        next.hand_l.orientation = Quaternion::rotation_x(0.0);
        next.hand_r.orientation = Quaternion::rotation_x(0.0);

        next.control_l.position = Vec3::new(1.0 - s_a.grip.0 * 2.0, 2.0, -2.0);
        next.control_r.position = Vec3::new(-1.0 + s_a.grip.0 * 2.0, 2.0, 2.0);

        next.control.position = Vec3::new(
            -3.0 + movement1abs * -3.0 + movement2abs * 5.0,
            s_a.grip.2 + movement1abs * -12.0 + movement2abs * 17.0,
            -s_a.grip.2 / 2.5 + s_a.grip.0 * -2.0 + movement2abs * 10.0,
        );

        next.control_l.orientation =
            Quaternion::rotation_x(PI / 1.5 + movement1abs * -1.0 + movement2abs * 3.0)
                * Quaternion::rotation_y(-0.3);
        next.control_r.orientation = Quaternion::rotation_x(
            PI / 1.5 + s_a.grip.0 * 0.2 + movement1abs * -1.0 + movement2abs * 3.0,
        ) * Quaternion::rotation_y(0.5 + s_a.grip.0 * 0.2);

        next.control.orientation =
            Quaternion::rotation_x(-1.35 + movement1abs * -0.3 + movement2abs * 1.0)
                * Quaternion::rotation_z(movement1abs * 1.0 + movement2abs * -1.8)
                * Quaternion::rotation_y(movement2abs * 0.5);

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.tail.orientation = Quaternion::rotation_x(0.05 * fastalt * speednormcancel)
            * Quaternion::rotation_z(fast * 0.15 * speednormcancel);

        next
    }
}
