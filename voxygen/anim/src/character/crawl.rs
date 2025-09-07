use super::{
    super::{Animation, vek::*},
    CharacterSkeleton, SkeletonAttr,
};
use std::{f32::consts::PI, ops::Mul};

pub struct CrawlAnimation;

impl Animation for CrawlAnimation {
    type Dependency<'a> = (Vec3<f32>, Vec3<f32>, Vec3<f32>, f32);
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_crawl\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "character_crawl"))]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, orientation, last_ori, global_time): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();
        *rate = 1.0;
        let slow = (anim_time * 3.0).sin();
        let breathe = ((anim_time * 0.5).sin()).abs();
        let walkintensity = if speed > 5.0 { 1.0 } else { 0.45 };
        let lab: f32 = 1.0;

        let short = (anim_time * lab * 7.0).sin();
        let noisea = (anim_time * 11.0 + PI / 6.0).sin();
        let noiseb = (anim_time * 19.0 + PI / 4.0).sin();

        let shorte = ((5.0 / (4.0 + 1.0 * ((anim_time * lab * 7.0).sin()).powi(2))).sqrt())
            * ((anim_time * lab * 7.0).sin());

        let head_look = Vec2::new(
            (global_time / 3.0 + anim_time / 40.0)
                .floor()
                .mul(7331.0)
                .sin()
                * 0.2,
            (global_time / 3.0 + anim_time / 37.0)
                .floor()
                .mul(1337.0)
                .sin()
                * 0.1,
        );

        let orientation: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if vek::Vec2::new(orientation, last_ori)
            .map(|o| o.magnitude_squared())
            .map(|m| m > 0.001 && m.is_finite())
            .reduce_and()
            && orientation.angle_between(last_ori).is_finite()
        {
            orientation.angle_between(last_ori).min(0.2)
                * last_ori.determine_side(Vec2::zero(), orientation).signum()
        } else {
            0.0
        } * 1.3;

        next.hold.scale = Vec3::one() * 0.0;

        if speed > 0.5 {
            next.hand_l.position = Vec3::new(-s_a.hand.0, 5.0 + s_a.hand.1, 11.0 - s_a.hand.2);
            next.hand_l.orientation = Quaternion::rotation_x(3.0);

            next.hand_r.position = Vec3::new(s_a.hand.0, 5.0 + s_a.hand.1, 11.0 - s_a.hand.2);
            next.hand_r.orientation = Quaternion::rotation_x(3.0);

            next.head.position = Vec3::new(0.0, 1.0 + s_a.head.0, -1.0 + s_a.head.1 + short * 0.06);
            next.head.orientation =
                Quaternion::rotation_z(tilt * -2.5 + head_look.x * 0.2 - short * 0.06)
                    * Quaternion::rotation_x(head_look.y + 0.65);

            next.chest.position = Vec3::new(0.0, s_a.chest.0, -5.0 + s_a.chest.1 + slow * 0.03);
            next.chest.orientation = Quaternion::rotation_z(0.03 + short * 0.08 + tilt * -0.2)
                * Quaternion::rotation_x(-1.25);

            next.belt.position = Vec3::new(0.0, s_a.belt.0, s_a.belt.1);
            next.belt.orientation = Quaternion::rotation_z(0.3 + head_look.x * -0.1);

            next.shorts.position = Vec3::new(0.0, s_a.shorts.0, s_a.shorts.1);
            next.shorts.orientation = Quaternion::rotation_z(0.0 + head_look.x * -0.2);

            next.back.orientation =
                Quaternion::rotation_x(-0.05 + short * 0.02 + noisea * 0.01 + noiseb * 0.01);

            next.foot_l.position = Vec3::new(-s_a.foot.0, -10.0 + s_a.foot.1, 1.0 + s_a.foot.2);
            next.foot_l.orientation = Quaternion::rotation_x(-1.6 + walkintensity * 0.5 * short);

            next.foot_r.position = Vec3::new(s_a.foot.0, -10.0 + s_a.foot.1, 1.0 + s_a.foot.2);
            next.foot_r.orientation = Quaternion::rotation_x(-1.6 - walkintensity * 0.5 * short);

            next.shoulder_l.orientation = Quaternion::rotation_x(2.2);

            next.shoulder_r.orientation = Quaternion::rotation_x(2.2);

            next.lantern.orientation =
                Quaternion::rotation_x(shorte * 0.2 + 0.4) * Quaternion::rotation_y(shorte * 0.1);
        } else {
            next.head.position = Vec3::new(
                0.0,
                1.0 + s_a.head.0,
                -2.0 + s_a.head.1 + slow * 0.1 + breathe * -0.05,
            );
            next.head.orientation = Quaternion::rotation_z(head_look.x)
                * Quaternion::rotation_x(0.8 + head_look.y.abs());

            next.chest.position = Vec3::new(0.0, s_a.chest.0, -5.0 + s_a.chest.1 + slow * 0.1);
            next.chest.orientation = Quaternion::rotation_x(-1.25);

            next.belt.position = Vec3::new(0.0, s_a.belt.0, s_a.belt.1);
            next.belt.orientation = Quaternion::rotation_z(0.3 + head_look.x * -0.1);

            next.shorts.position = Vec3::new(0.0, s_a.shorts.0, s_a.shorts.1);
            next.shorts.orientation = Quaternion::rotation_z(0.0 + head_look.x * -0.2);

            next.hand_l.position = Vec3::new(-s_a.hand.0, 5.0 + s_a.hand.1, 11.0 - s_a.hand.2);
            next.hand_l.orientation = Quaternion::rotation_x(3.0);

            next.hand_r.position = Vec3::new(s_a.hand.0, 5.0 + s_a.hand.1, 11.0 - s_a.hand.2);
            next.hand_r.orientation = Quaternion::rotation_x(3.0);

            next.foot_l.position = Vec3::new(-s_a.foot.0, -10.0 + s_a.foot.1, 1.0 + s_a.foot.2);
            next.foot_l.orientation = Quaternion::rotation_x(-1.6);

            next.foot_r.position = Vec3::new(s_a.foot.0, -10.0 + s_a.foot.1, 1.0 + s_a.foot.2);
            next.foot_r.orientation = Quaternion::rotation_x(-1.6);

            next.shoulder_l.orientation = Quaternion::rotation_x(2.2);

            next.shoulder_r.orientation = Quaternion::rotation_x(2.2);
        }

        next.do_hold_lantern(s_a, anim_time, anim_time, speed / 10.0, 0.0, tilt);

        next
    }
}
