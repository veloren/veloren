use super::{
    super::{vek::*, Animation},
    BirdLargeSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct ShockwaveAnimation;

impl Animation for ShockwaveAnimation {
    #[allow(clippy::type_complexity)]
    type Dependency<'a> = (Option<StageSection>, bool);
    type Skeleton = BirdLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"bird_large_shockwave\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "bird_large_shockwave")]
    fn update_skeleton_inner<'a>(
        skeleton: &Self::Skeleton,
        (stage_section, on_ground): Self::Dependency<'a>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (movement1base, movement2base, movement3, _twitch) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.5), 0.0, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time.min(1.0).powf(0.1), 0.0, anim_time),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time, 1.0),
            _ => (0.0, 0.0, 0.0, 0.0),
        };

        let pullback = 1.0 - movement3;

        let movement1abs = movement1base * pullback;
        let movement2abs = movement2base * pullback;

        let wave_slow_cos = (anim_time * 4.5).cos();

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + movement1abs * 1.5);
        next.chest.orientation = Quaternion::rotation_x(movement1abs * 1.0 + movement2abs * -1.0);

        next.neck.position = Vec3::new(0.0, s_a.neck.0, s_a.neck.1);
        next.neck.orientation = Quaternion::rotation_x(-0.2);

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_x(wave_slow_cos * 0.01);

        next.beak.position = Vec3::new(0.0, s_a.beak.0, s_a.beak.1);
        next.beak.orientation = Quaternion::rotation_x(wave_slow_cos * -0.02 - 0.02);

        next.tail_front.position = Vec3::new(0.0, s_a.tail_front.0, s_a.tail_front.1);
        next.tail_front.orientation = Quaternion::rotation_x(0.6);
        next.tail_rear.position = Vec3::new(0.0, s_a.tail_rear.0, s_a.tail_rear.1);
        next.tail_rear.orientation = Quaternion::rotation_x(-0.2);

        if on_ground {
            next.wing_in_l.position = Vec3::new(-s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);
            next.wing_in_r.position = Vec3::new(s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);

            next.wing_in_l.orientation =
                Quaternion::rotation_y(-0.8 + movement1abs * 1.6 + movement2abs * -1.6)
                    * Quaternion::rotation_z(0.2 + movement1abs * -0.8);
            next.wing_in_r.orientation =
                Quaternion::rotation_y(0.8 + movement1abs * -1.6 + movement2abs * 1.6)
                    * Quaternion::rotation_z(-0.2 + movement1abs * 0.8);

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

            next.leg_l.position = Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2);
            next.leg_l.orientation = Quaternion::rotation_x(0.0);
            next.leg_r.position = Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2);
            next.leg_r.orientation = Quaternion::rotation_x(0.0);

            next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2);
            next.foot_l.orientation = Quaternion::rotation_x(0.0);
            next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2);
            next.foot_r.orientation = Quaternion::rotation_x(0.0);
        } else {
        }

        next
    }
}
