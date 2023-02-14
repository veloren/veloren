use super::{
    super::{vek::*, Animation},
    BirdMediumSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
use std::f32::consts::PI;

pub struct DashAnimation;
type DashAnimationDependency<'a> = (
    Vec3<f32>,
    Vec3<f32>,
    Vec3<f32>,
    f32,
    Option<StageSection>,
    f32,
    f32,
);

impl Animation for DashAnimation {
    type Dependency<'a> = DashAnimationDependency<'a>;
    type Skeleton = BirdMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"bird_medium_dash\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "bird_medium_dash")]
    fn update_skeleton_inner<'a>(
        skeleton: &Self::Skeleton,
        (velocity, orientation, last_ori, acc_vel, stage_section, global_time, timer): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = (Vec2::<f32>::from(velocity).magnitude()).min(22.0);
        *rate = 1.0;

        let (movement1base, chargemovementbase, movement2base, movement3, legtell) =
            match stage_section {
                Some(StageSection::Buildup) => (anim_time.sqrt(), 0.0, 0.0, 0.0, anim_time),
                Some(StageSection::Charge) => (1.0, 1.0, 0.0, 0.0, 0.0),
                Some(StageSection::Action) => (1.0, 0.0, anim_time.powi(4), 0.0, 1.0),
                Some(StageSection::Recover) => (1.0, 0.0, 1.0, anim_time, 1.0),
                _ => (0.0, 0.0, 0.0, 0.0, 0.0),
            };
        let pullback = 1.0 - movement3;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let movement1abs = movement1base * pullback;
        let movement2abs = movement2base * pullback;
        let legtwitch = (legtell * 6.0).sin() * pullback;
        let legswing = legtell * pullback;
        let chargeanim = (chargemovementbase * anim_time * 15.0).sin();

        //let speednorm = speed / 13.0;
        let speednorm = (speed / 13.0).powf(0.25);

        let speedmult = 0.8;
        let lab: f32 = 0.6; //6

        // acc_vel and anim_time mix to make sure phase length isn't starting at
        // +infinite
        let mixed_vel = acc_vel + anim_time * 5.0; //sets run frequency using speed, with anim_time setting a floor

        let short = ((1.0
            / (0.72
                + 0.28 * ((mixed_vel * 1.0 * lab * speedmult + PI * -0.15 - 0.5).sin()).powi(2)))
        .sqrt())
            * ((mixed_vel * 1.0 * lab * speedmult + PI * -0.15 - 0.5).sin())
            * speednorm;

        //
        let shortalt = (mixed_vel * 1.0 * lab * speedmult + PI * 3.0 / 8.0 - 0.5).sin() * speednorm;

        let ori: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if vek::Vec2::new(ori, last_ori)
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

        next.head.scale = Vec3::one() * 0.98;
        next.leg_l.scale = Vec3::one() * 0.98;
        next.leg_r.scale = Vec3::one() * 0.98;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_x(
            -0.1 * speednorm + short * -0.05 + movement1abs * -0.8 + movement2abs * 0.2,
        ) * Quaternion::rotation_y(tilt * 0.2)
            * Quaternion::rotation_z(shortalt * -0.05 - tilt * 1.5);

        next.chest.position = Vec3::new(
            0.0,
            s_a.chest.0,
            s_a.chest.1 + short * 0.5 + 0.5 * speednorm,
        );
        next.chest.orientation =
            Quaternion::rotation_x(short * 0.07 + movement1abs * 0.8 + movement2abs * -1.2)
                * Quaternion::rotation_y(tilt * 0.8)
                * Quaternion::rotation_z(shortalt * 0.10);

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.tail.orientation =
            Quaternion::rotation_x(0.6 + short * -0.02 + movement1abs * -0.8 + movement2abs * 0.8);

        next.wing_in_l.position = Vec3::new(-s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);
        next.wing_in_r.position = Vec3::new(s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);

        next.wing_in_l.orientation =
            Quaternion::rotation_y(
                -0.8 + movement1abs * 1.0 + chargeanim * 0.2 - movement2abs * 0.6,
            ) * Quaternion::rotation_z(0.2 - movement1abs * 0.6 - movement2abs * 0.6);
        next.wing_in_r.orientation =
            Quaternion::rotation_y(
                0.8 - movement1abs * 1.0 - chargeanim * 0.2 + movement2abs * 0.6,
            ) * Quaternion::rotation_z(-0.2 + movement1abs * 0.6 + movement2abs * 0.6);

        next.wing_out_l.position = Vec3::new(-s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
        next.wing_out_r.position = Vec3::new(s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
        next.wing_out_l.orientation =
            Quaternion::rotation_y(-0.2 + short * 0.05) * Quaternion::rotation_z(0.2);
        next.wing_out_r.orientation =
            Quaternion::rotation_y(0.2 + short * -0.05) * Quaternion::rotation_z(-0.2);

        if legtell > 0.0 {
            if mirror.is_sign_positive() {
                next.leg_l.orientation = Quaternion::rotation_x(legswing * 1.1 + legtwitch * 0.5);

                next.leg_r.orientation = Quaternion::rotation_x(0.0);
            } else {
                next.leg_l.orientation = Quaternion::rotation_x(0.0);

                next.leg_r.orientation = Quaternion::rotation_x(legswing * 1.1 + legtwitch * 0.5);
            }
        }

        next
    }
}
