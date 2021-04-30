use super::{
    super::{vek::*, Animation},
    BirdLargeSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct AlphaAnimation;

impl Animation for AlphaAnimation {
    type Dependency<'a> = (Option<StageSection>, Vec3<f32>, Vec3<f32>, bool);
    type Skeleton = BirdLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"bird_large_alpha\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "bird_large_alpha")]
    fn update_skeleton_inner<'a>(
        skeleton: &Self::Skeleton,
        (stage_section, orientation, last_ori, on_ground): Self::Dependency<'a>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (move1base, move2base, move3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
            Some(StageSection::Swing) => (1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powf(4.0)),
            _ => (0.0, 0.0, 0.0),
        };

        let wave_slow_cos = (anim_time * 4.5).cos();

        let pullback = 1.0 - move3;

        let move1 = move1base * pullback;
        let move2 = move2base * pullback;

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

        next.head.scale = Vec3::one() * 0.98;
        next.neck.scale = Vec3::one() * 1.02;
        next.leg_l.scale = Vec3::one() / 8.0 * 0.98;
        next.leg_r.scale = Vec3::one() / 8.0 * 0.98;
        next.foot_l.scale = Vec3::one() * 1.02;
        next.foot_r.scale = Vec3::one() * 1.02;
        next.chest.scale = Vec3::one() * s_a.scaler / 8.0;

        next.chest.position =
            Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + wave_slow_cos * 0.06) * s_a.scaler / 8.0;
        next.chest.orientation = Quaternion::rotation_x(move1 * 0.5 - move2 * 0.8);

        next.neck.position = Vec3::new(0.0, s_a.neck.0, s_a.neck.1);
        next.neck.orientation = Quaternion::rotation_x(move1 * 0.5 - move2 * 0.8)
            * Quaternion::rotation_z(move1 * tilt * 1.5);

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_x(move1 * 0.5 - move2 * 0.8);

        next.beak.position = Vec3::new(0.0, s_a.beak.0, s_a.beak.1);
        next.beak.orientation = Quaternion::rotation_x(wave_slow_cos * -0.02 - 0.02);

        if on_ground {
            next.tail_front.position = Vec3::new(0.0, s_a.tail_front.0, s_a.tail_front.1);
            next.tail_front.orientation = Quaternion::rotation_x(-move1 * 0.2);
            next.tail_rear.position = Vec3::new(0.0, s_a.tail_rear.0, s_a.tail_rear.1);
            next.tail_rear.orientation = Quaternion::rotation_x(0.0);

            next.wing_in_l.position = Vec3::new(-s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);
            next.wing_in_r.position = Vec3::new(s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);

            next.wing_in_l.orientation =
                Quaternion::rotation_y(-1.0 + wave_slow_cos * 0.06) * Quaternion::rotation_z(0.2);
            next.wing_in_r.orientation =
                Quaternion::rotation_y(1.0 - wave_slow_cos * 0.06) * Quaternion::rotation_z(-0.2);

            next.wing_mid_l.position = Vec3::new(-s_a.wing_mid.0, s_a.wing_mid.1, s_a.wing_mid.2);
            next.wing_mid_r.position = Vec3::new(s_a.wing_mid.0, s_a.wing_mid.1, s_a.wing_mid.2);
            next.wing_mid_l.orientation =
                Quaternion::rotation_y(-0.1) * Quaternion::rotation_z(0.7);
            next.wing_mid_r.orientation =
                Quaternion::rotation_y(0.1) * Quaternion::rotation_z(-0.7);

            next.wing_out_l.position = Vec3::new(-s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
            next.wing_out_r.position = Vec3::new(s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
            next.wing_out_l.orientation =
                Quaternion::rotation_y(-0.2) * Quaternion::rotation_z(0.2);
            next.wing_out_r.orientation =
                Quaternion::rotation_y(0.2) * Quaternion::rotation_z(-0.2);
        } else {
        }

        next
    }
}
