use super::{
    super::{vek::*, Animation},
    BirdLargeSkeleton, SkeletonAttr,
};
use std::ops::Mul;

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency<'a> = f32;
    type Skeleton = BirdLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"bird_large_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "bird_large_idle")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        global_time: Self::Dependency<'_>,
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

        next.head.scale = Vec3::one() * 0.99;
        next.neck.scale = Vec3::one() * 1.01;
        next.leg_l.scale = Vec3::one();
        next.leg_r.scale = Vec3::one();
        next.foot_l.scale = Vec3::one() * 1.01;
        next.foot_r.scale = Vec3::one() * 1.01;
        next.chest.scale = Vec3::one() * s_a.scaler * 0.99;
        next.tail_front.scale = Vec3::one() * 1.01;
        next.tail_rear.scale = Vec3::one() * 0.99;

        next.neck.position = Vec3::new(0.0, s_a.neck.0, s_a.neck.1);
        next.neck.orientation = Quaternion::rotation_x(0.0);

        next.beak.position = Vec3::new(0.0, s_a.beak.0, s_a.beak.1);
        next.beak.orientation = Quaternion::rotation_x(wave_slow_cos * -0.02 - 0.02);

        next.leg_l.position = Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2);
        next.leg_l.orientation = Quaternion::rotation_x(0.0);
        next.leg_r.position = Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2);
        next.leg_r.orientation = Quaternion::rotation_x(0.0);

        next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2);
        next.foot_l.orientation = Quaternion::rotation_x(0.0);
        next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2);
        next.foot_r.orientation = Quaternion::rotation_x(0.0);

        if s_a.wyvern {
            next.chest.position =
                Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + wave_slow_cos * 0.06) * s_a.scaler;
            next.chest.orientation = Quaternion::rotation_x(-0.1);

            next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
            next.head.orientation = Quaternion::rotation_z(duck_head_look.x)
                * Quaternion::rotation_x(0.4 - duck_head_look.y.abs() + wave_slow_cos * 0.01);

            next.tail_front.position = Vec3::new(0.0, s_a.tail_front.0, s_a.tail_front.1);
            next.tail_front.orientation = Quaternion::rotation_x(0.38);
            next.tail_rear.position = Vec3::new(0.0, s_a.tail_rear.0, s_a.tail_rear.1);
            next.tail_rear.orientation = Quaternion::rotation_x(-0.1);

            next.wing_in_l.position = Vec3::new(-s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);
            next.wing_in_r.position = Vec3::new(s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);

            next.wing_in_l.orientation =
                Quaternion::rotation_y(-0.8 + wave_slow_cos * 0.06) * Quaternion::rotation_z(0.4);
            next.wing_in_r.orientation =
                Quaternion::rotation_y(0.8 - wave_slow_cos * 0.06) * Quaternion::rotation_z(-0.4);

            next.wing_mid_l.position = Vec3::new(-s_a.wing_mid.0, s_a.wing_mid.1, s_a.wing_mid.2);
            next.wing_mid_r.position = Vec3::new(s_a.wing_mid.0, s_a.wing_mid.1, s_a.wing_mid.2);
            next.wing_mid_l.orientation = Quaternion::rotation_x(0.1)
                * Quaternion::rotation_y(-0.1)
                * Quaternion::rotation_z(0.7);
            next.wing_mid_r.orientation = Quaternion::rotation_x(0.1)
                * Quaternion::rotation_y(0.1)
                * Quaternion::rotation_z(-0.7);

            next.wing_out_l.position = Vec3::new(-s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
            next.wing_out_r.position = Vec3::new(s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
            next.wing_out_l.orientation = Quaternion::rotation_x(0.0)
                * Quaternion::rotation_y(0.1)
                * Quaternion::rotation_z(0.55);
            next.wing_out_r.orientation = Quaternion::rotation_x(0.0)
                * Quaternion::rotation_y(-0.1)
                * Quaternion::rotation_z(-0.55);
        } else {
            next.chest.position =
                Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + wave_slow_cos * 0.06 + 1.5) * s_a.scaler;
            next.chest.orientation = Quaternion::rotation_x(0.0);

            next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
            next.head.orientation = Quaternion::rotation_z(duck_head_look.x)
                * Quaternion::rotation_x(-duck_head_look.y.abs() + wave_slow_cos * 0.01);

            next.tail_front.position = Vec3::new(0.0, s_a.tail_front.0, s_a.tail_front.1);
            next.tail_front.orientation = Quaternion::rotation_x(0.6);
            next.tail_rear.position = Vec3::new(0.0, s_a.tail_rear.0, s_a.tail_rear.1);
            next.tail_rear.orientation = Quaternion::rotation_x(-0.2);

            next.wing_in_l.position = Vec3::new(-s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);
            next.wing_in_r.position = Vec3::new(s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);

            next.wing_in_l.orientation =
                Quaternion::rotation_y(-0.8 + wave_slow_cos * 0.06) * Quaternion::rotation_z(0.2);
            next.wing_in_r.orientation =
                Quaternion::rotation_y(0.8 - wave_slow_cos * 0.06) * Quaternion::rotation_z(-0.2);

            next.wing_mid_l.position = Vec3::new(-s_a.wing_mid.0, s_a.wing_mid.1, s_a.wing_mid.2);
            next.wing_mid_r.position = Vec3::new(s_a.wing_mid.0, s_a.wing_mid.1, s_a.wing_mid.2);
            next.wing_mid_l.orientation =
                Quaternion::rotation_y(-0.1) * Quaternion::rotation_z(0.7);
            next.wing_mid_r.orientation =
                Quaternion::rotation_y(0.1) * Quaternion::rotation_z(-0.7);

            next.wing_out_l.position = Vec3::new(-s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
            next.wing_out_r.position = Vec3::new(s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
            next.wing_out_l.orientation =
                Quaternion::rotation_y(-0.4) * Quaternion::rotation_z(0.2);
            next.wing_out_r.orientation =
                Quaternion::rotation_y(0.4) * Quaternion::rotation_z(-0.2);
        }

        next
    }
}
