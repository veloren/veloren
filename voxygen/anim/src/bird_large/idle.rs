use super::{
    super::{vek::*, Animation},
    BirdLargeSkeleton, SkeletonAttr,
};

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency = f32;
    type Skeleton = BirdLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"bird_large_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "bird_large_idle")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        _global_time: Self::Dependency,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let fast = anim_time * 4.0;

        let freq = 8.0;
        let off2 = -1.7;
        let off3 = -2.0;
        let off4 = -2.4;
        let flap1 = 7.0 / 16.0 * (freq * anim_time).sin()
            + 7.0 / 64.0 * (freq * 2.0 * anim_time).sin()
            + 1.0 / 48.0 * (freq * 3.0 * anim_time).sin();
        let flap2 = 7.0 / 16.0 * (freq * anim_time + off2).sin()
            + 7.0 / 64.0 * (freq * 2.0 * anim_time + off2).sin()
            + 1.0 / 48.0 * (freq * 3.0 * anim_time + off2).sin();
        let flap3 = 7.0 / 16.0 * (freq * anim_time + off3).sin()
            + 7.0 / 64.0 * (freq * 2.0 * anim_time + off3).sin()
            + 1.0 / 48.0 * (freq * 3.0 * anim_time + off3).sin();
        let flap4 = 7.0 / 16.0 * (freq * anim_time + off4).sin()
            + 7.0 / 64.0 * (freq * 2.0 * anim_time + off4).sin()
            + 1.0 / 48.0 * (freq * 3.0 * anim_time + off4).sin();

        next.chest.scale = Vec3::one() * s_a.scaler / 4.0;
        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1) * s_a.scaler / 4.0;
        next.chest.orientation = Quaternion::rotation_x(0.0);

        next.neck.position = Vec3::new(0.0, s_a.neck.0, s_a.neck.1);
        next.neck.orientation = Quaternion::rotation_x(0.0);

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_x(0.0);

        next.beak.position = Vec3::new(0.0, s_a.beak.0, s_a.beak.1);

        next.tail_front.position = Vec3::new(0.0, s_a.tail_front.0, s_a.tail_front.1);
        next.tail_front.orientation = Quaternion::rotation_x(0.0);
        next.tail_rear.position = Vec3::new(0.0, s_a.tail_rear.0, s_a.tail_rear.1);
        next.tail_rear.orientation = Quaternion::rotation_x(0.0);

        next.wing_in_l.position = Vec3::new(-s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);
        next.wing_in_r.position = Vec3::new(s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);

        next.wing_in_l.orientation = Quaternion::rotation_y(0.0);
        next.wing_in_r.orientation = Quaternion::rotation_y(0.0) * Quaternion::rotation_x(0.0);

        next.wing_mid_l.position = Vec3::new(-s_a.wing_mid.0, s_a.wing_mid.1, s_a.wing_mid.2);
        next.wing_mid_r.position = Vec3::new(s_a.wing_mid.0, s_a.wing_mid.1, s_a.wing_mid.2);
        next.wing_mid_l.orientation = Quaternion::rotation_y(0.0);
        next.wing_mid_r.orientation = Quaternion::rotation_y(0.0);

        next.wing_out_l.position = Vec3::new(-s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
        next.wing_out_r.position = Vec3::new(s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
        next.wing_out_l.orientation = Quaternion::rotation_y(0.0);
        next.wing_out_r.orientation = Quaternion::rotation_y(0.0);

        next.leg_l.position = Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2 + 3.0);
        next.leg_l.orientation = Quaternion::rotation_x(0.0);
        next.leg_r.position = Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2 + 3.0);
        next.leg_r.orientation = Quaternion::rotation_x(0.0);

        next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2);
        next.foot_l.orientation = Quaternion::rotation_x(0.0);
        next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2);
        next.foot_r.orientation = Quaternion::rotation_x(0.0);

        next
    }
}
