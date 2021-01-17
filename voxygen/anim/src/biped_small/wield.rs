use super::{
    super::{vek::*, Animation},
    BipedSmallSkeleton, SkeletonAttr,
};
use std::f32::consts::PI;

pub struct WieldAnimation;

type WieldAnimationDependency = (Vec3<f32>, Vec3<f32>, Vec3<f32>, f64, Vec3<f32>, f32);

impl Animation for WieldAnimation {
    type Dependency = WieldAnimationDependency;
    type Skeleton = BipedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_small_wield\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_small_wield")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, _orientation, _last_ori, _global_time, _avg_vel, acc_vel): Self::Dependency,
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

        next.foot_l.scale = Vec3::one() / 13.0;
        next.foot_r.scale = Vec3::one() / 13.0;

        next.chest.scale = Vec3::one() / 13.0;
        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1 + fast * -0.1 * speednormcancel);
        next.head.orientation = Quaternion::rotation_x(0.45 * speednorm)
            * Quaternion::rotation_y(fast * 0.15 * speednormcancel);
        next.chest.position = Vec3::new(
            0.0,
            s_a.chest.0,
            s_a.chest.1 + fastalt * 0.4 * speednormcancel + speednormcancel * -0.5,
        ) / 13.0;

        next.shorts.position = Vec3::new(0.0, s_a.shorts.0, s_a.shorts.1);
        //next.main.position = Vec3::new(0.0, s_a.hand.2*-1.0, 0.0);

        next.main.position = Vec3::new(0.0, 0.0, 0.0);

        next.hand_l.position = Vec3::new(s_a.grip.0 * 4.0, 0.0, s_a.grip.2);
        next.hand_r.position = Vec3::new(-s_a.grip.0 * 4.0, 0.0, s_a.grip.2);

        next.hand_l.orientation = Quaternion::rotation_x(0.0);
        next.hand_r.orientation = Quaternion::rotation_x(0.0);

        next.control_l.position = Vec3::new(1.0 - s_a.grip.0 * 2.0, 2.0, -2.0);
        next.control_r.position = Vec3::new(-1.0 + s_a.grip.0 * 2.0, 2.0, 2.0);

        next.control.position = Vec3::new(
            -3.0,
            s_a.grip.2,
            -s_a.grip.2 / 2.5 + s_a.grip.0 * -2.0 + fastacc * 1.5 + fastalt * 0.5 * speednormcancel,
        );

        next.control_l.orientation =
            Quaternion::rotation_x(PI / 1.5 + slow * 0.1) * Quaternion::rotation_y(-0.3);
        next.control_r.orientation =
            Quaternion::rotation_x(PI / 1.5 + slow * 0.1 + s_a.grip.0 * 0.2)
                * Quaternion::rotation_y(0.5 + slow * 0.0 + s_a.grip.0 * 0.2);

        next.control.orientation = Quaternion::rotation_x(-1.35 + 0.5 * speednorm);

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.tail.orientation = Quaternion::rotation_x(0.05 * fastalt * speednormcancel)
            * Quaternion::rotation_z(fast * 0.15 * speednormcancel);

        next
    }
}
