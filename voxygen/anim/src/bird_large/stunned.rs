use super::{
    super::{vek::*, Animation},
    BirdLargeSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
use std::ops::Mul;

pub struct StunnedAnimation;

impl Animation for StunnedAnimation {
    type Dependency = (f32, f32, Option<StageSection>, f32);
    type Skeleton = BirdLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"bird_large_stunned\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "bird_large_stunned")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, global_time, stage_section, timer): Self::Dependency,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let duck_head_look = Vec2::new(
            (global_time / 2.0 + anim_time / 8.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.5,
            (global_time / 2.0 + anim_time / 8.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.25,
        );
        let wave_slow_cos = (anim_time * 4.5).cos();

        let (movement1base, movement2, twitch) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.1), 0.0, anim_time),
            Some(StageSection::Recover) => (1.0, anim_time.powf(4.0), 1.0),
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - movement2;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let twitch2 = mirror * (twitch * 20.0).sin() * pullback;
        let movement1abs = movement1base * pullback;

        next.head.scale = Vec3::one() * 0.98;
        next.neck.scale = Vec3::one() * 1.02;
        next.beak.scale = Vec3::one() * 0.98;
        next.leg_l.scale = Vec3::one() / 8.0 * 0.98;
        next.leg_r.scale = Vec3::one() / 8.0 * 0.98;
        next.foot_l.scale = Vec3::one() * 1.02;
        next.foot_r.scale = Vec3::one() * 1.02;
        next.chest.scale = Vec3::one() * s_a.scaler / 8.0;

        next.chest.position =
            Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + wave_slow_cos * 0.06) * s_a.scaler / 8.0;
        next.chest.orientation = Quaternion::rotation_x(movement1base * 0.5);

        next.neck.position = Vec3::new(0.0, s_a.neck.0, s_a.neck.1);
        next.neck.orientation = Quaternion::rotation_x(0.0);

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_z(twitch2 * 0.8)
            * Quaternion::rotation_x(-duck_head_look.y.abs() + wave_slow_cos * 0.01);

        next.beak.position = Vec3::new(0.0, s_a.beak.0, s_a.beak.1);
        next.beak.orientation = Quaternion::rotation_x(-movement1abs * 0.8);

        next.tail_front.position = Vec3::new(0.0, s_a.tail_front.0, s_a.tail_front.1);
        next.tail_front.orientation = Quaternion::rotation_x(0.0);
        next.tail_rear.position = Vec3::new(0.0, s_a.tail_rear.0, s_a.tail_rear.1);
        next.tail_rear.orientation = Quaternion::rotation_x(0.0);

        next.wing_in_l.position = Vec3::new(-s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);
        next.wing_in_r.position = Vec3::new(s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);

        next.wing_in_l.orientation = Quaternion::rotation_y(wave_slow_cos * 0.06 + twitch2 * 0.8)
            * Quaternion::rotation_z(0.2 - movement1abs);
        next.wing_in_r.orientation = Quaternion::rotation_y(wave_slow_cos * 0.06 - twitch2 * 0.8)
            * Quaternion::rotation_z(-0.2 + movement1abs);

        next.wing_mid_l.position = Vec3::new(-s_a.wing_mid.0, s_a.wing_mid.1, s_a.wing_mid.2);
        next.wing_mid_r.position = Vec3::new(s_a.wing_mid.0, s_a.wing_mid.1, s_a.wing_mid.2);
        next.wing_mid_l.orientation = Quaternion::rotation_y(-0.1) * Quaternion::rotation_z(0.7);
        next.wing_mid_r.orientation = Quaternion::rotation_y(0.1) * Quaternion::rotation_z(-0.7);

        next.wing_out_l.position = Vec3::new(-s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
        next.wing_out_r.position = Vec3::new(s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
        next.wing_out_l.orientation = Quaternion::rotation_y(-0.2) * Quaternion::rotation_z(0.2);
        next.wing_out_r.orientation = Quaternion::rotation_y(0.2) * Quaternion::rotation_z(-0.2);

        next.leg_l.position = Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2) / 8.0;
        next.leg_l.orientation = Quaternion::rotation_x(movement1abs * 0.8);
        next.leg_r.position = Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2) / 8.0;
        next.leg_r.orientation = Quaternion::rotation_x(0.0);

        next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2);
        next.foot_l.orientation = Quaternion::rotation_x(0.0);
        next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2);
        next.foot_r.orientation = Quaternion::rotation_x(0.0);

        next
    }
}
