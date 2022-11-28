use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use core::f32::consts::PI;

pub struct WallrunAnimation;

type WallrunAnimationDependency = (Vec3<f32>, f32, Option<Vec3<f32>>);

impl Animation for WallrunAnimation {
    type Dependency<'a> = WallrunAnimationDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_wallrun\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_wallrun")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (orientation, acc_vel, wall): Self::Dependency<'_>,
        _anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        *rate = 1.0;

        let lab: f32 = 0.8;

        let footrotl = ((1.0 / (0.5 + (0.5) * ((acc_vel * 1.6 * lab + PI * 1.4).sin()).powi(2)))
            .sqrt())
            * ((acc_vel * 1.6 * lab + PI * 1.4).sin());
        let footrotr = ((1.0 / (0.5 + (0.5) * ((acc_vel * 1.6 * lab + PI * 0.4).sin()).powi(2)))
            .sqrt())
            * ((acc_vel * 1.6 * lab + PI * 0.4).sin());

        let foothoril = (acc_vel * 2.2 * lab + PI * 1.45).sin();
        let foothorir = (acc_vel * 2.2 * lab + PI * (0.45)).sin();

        let shortalt = (acc_vel * lab * 2.2 + PI / 1.0).sin();

        let short = ((5.0 / (1.5 + 3.5 * ((acc_vel * lab * 1.6).sin()).powi(2))).sqrt())
            * ((acc_vel * lab * 1.6).sin());

        next.shorts.position = Vec3::new(0.0, s_a.shorts.0 + 2.0, s_a.shorts.1 + 1.0);
        next.belt.position = Vec3::new(0.0, s_a.belt.0 + 1.0, s_a.belt.1);

        next.foot_l.position = Vec3::new(
            -s_a.foot.0,
            s_a.foot.1 + foothorir * 6.0,
            s_a.foot.2 + shortalt * 2.0 + 2.0,
        );
        next.foot_l.orientation = Quaternion::rotation_x(0.6 + shortalt * 0.8);

        next.foot_r.position = Vec3::new(
            s_a.foot.0,
            s_a.foot.1 + foothoril * 6.0,
            s_a.foot.2 + shortalt * -2.0 + 2.0,
        );
        next.foot_r.orientation = Quaternion::rotation_x(0.6 - shortalt * 0.8);
        next.belt.orientation = Quaternion::rotation_x(0.3);
        next.shorts.orientation = Quaternion::rotation_x(0.5);

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + shortalt * 0.0);

        next.shoulder_l.orientation = Quaternion::rotation_x(short * 0.15);

        next.shoulder_r.orientation = Quaternion::rotation_x(short * -0.15);

        if wall.map_or(false, |e| e.y > 0.5) {
            let push = (1.0 - orientation.x.abs()).powi(2);
            let right_sub = -(orientation.x).min(0.0);
            let left_sub = (orientation.x).max(0.0);
            next.hand_l.position = Vec3::new(
                -s_a.hand.0,
                s_a.hand.1,
                s_a.hand.2 + push * 5.0 + 2.0 * left_sub,
            );
            next.hand_r.position = Vec3::new(
                s_a.hand.0,
                s_a.hand.1,
                s_a.hand.2 + push * 5.0 + 2.0 * right_sub,
            );
            next.hand_l.orientation =
                Quaternion::rotation_x(push * 2.0 + footrotr * -0.2 * right_sub)
                    * Quaternion::rotation_y(1.0 * left_sub)
                    * Quaternion::rotation_z(2.5 * left_sub + 1.0 * right_sub);
            next.hand_r.orientation =
                Quaternion::rotation_x(push * 2.0 + footrotl * -0.2 * left_sub)
                    * Quaternion::rotation_y(-1.0 * right_sub)
                    * Quaternion::rotation_z(-2.5 * right_sub - 1.0 * left_sub);
            next.torso.orientation = Quaternion::rotation_y(orientation.x / 1.5);
            next.chest.orientation = Quaternion::rotation_y(orientation.x / -3.0)
                * Quaternion::rotation_z(shortalt * -0.2);
            next.head.orientation = Quaternion::rotation_z(shortalt * 0.25)
                * Quaternion::rotation_z(orientation.x / -2.0)
                * Quaternion::rotation_x(-0.1);
        } else if wall.map_or(false, |e| e.y < -0.5) {
            let push = (1.0 - orientation.x.abs()).powi(2);
            let right_sub = (orientation.x).max(0.0);
            let left_sub = -(orientation.x).min(0.0);
            next.hand_l.position = Vec3::new(
                -s_a.hand.0,
                s_a.hand.1,
                s_a.hand.2 + push * 5.0 + 2.0 * left_sub,
            );
            next.hand_r.position = Vec3::new(
                s_a.hand.0,
                s_a.hand.1,
                s_a.hand.2 + push * 5.0 + 2.0 * right_sub,
            );
            next.hand_l.orientation =
                Quaternion::rotation_x(push * 2.0 + footrotr * -0.2 * right_sub)
                    * Quaternion::rotation_y(1.0 * left_sub)
                    * Quaternion::rotation_z(2.5 * left_sub + 1.0 * right_sub);
            next.hand_r.orientation =
                Quaternion::rotation_x(push * 2.0 + footrotl * -0.2 * left_sub)
                    * Quaternion::rotation_y(-1.0 * right_sub)
                    * Quaternion::rotation_z(-2.5 * right_sub - 1.0 * left_sub);
            next.chest.orientation = Quaternion::rotation_y(orientation.x);
            next.torso.orientation = Quaternion::rotation_y(orientation.x / -1.5);
            next.chest.orientation = Quaternion::rotation_y(orientation.x / 3.0)
                * Quaternion::rotation_z(shortalt * -0.2);
            next.head.orientation = Quaternion::rotation_z(shortalt * 0.25)
                * Quaternion::rotation_z(orientation.x / 2.0)
                * Quaternion::rotation_x(-0.1);
        } else if wall.map_or(false, |e| e.x < -0.5) {
            let push = (1.0 - orientation.y.abs()).powi(2);
            let right_sub = -(orientation.y).min(0.0);
            let left_sub = (orientation.y).max(0.0);
            next.hand_l.position = Vec3::new(
                -s_a.hand.0,
                s_a.hand.1,
                s_a.hand.2 + push * 5.0 + 2.0 * left_sub,
            );
            next.hand_r.position = Vec3::new(
                s_a.hand.0,
                s_a.hand.1,
                s_a.hand.2 + push * 5.0 + 2.0 * right_sub,
            );
            next.hand_l.orientation =
                Quaternion::rotation_x(push * 2.0 + footrotr * -0.2 * right_sub)
                    * Quaternion::rotation_y(1.0 * left_sub)
                    * Quaternion::rotation_z(2.5 * left_sub + 1.0 * right_sub);
            next.hand_r.orientation =
                Quaternion::rotation_x(push * 2.0 + footrotl * -0.2 * left_sub)
                    * Quaternion::rotation_y(-1.0 * right_sub)
                    * Quaternion::rotation_z(-2.5 * right_sub - 1.0 * left_sub);
            next.torso.orientation = Quaternion::rotation_y(orientation.y / 1.5);
            next.chest.orientation = Quaternion::rotation_y(orientation.y / -3.0)
                * Quaternion::rotation_z(shortalt * -0.2);
            next.head.orientation = Quaternion::rotation_z(shortalt * 0.25)
                * Quaternion::rotation_z(orientation.y / -2.0)
                * Quaternion::rotation_x(-0.1);
        } else if wall.map_or(false, |e| e.x > 0.5) {
            let push = (1.0 - orientation.y.abs()).powi(2);
            let right_sub = (orientation.y).max(0.0);
            let left_sub = -(orientation.y).min(0.0);
            next.hand_l.position = Vec3::new(
                -s_a.hand.0,
                s_a.hand.1,
                s_a.hand.2 + push * 5.0 + 2.0 * left_sub,
            );
            next.hand_r.position = Vec3::new(
                s_a.hand.0,
                s_a.hand.1,
                s_a.hand.2 + push * 5.0 + 2.0 * right_sub,
            );
            next.hand_l.orientation =
                Quaternion::rotation_x(push * 2.0 + footrotr * -0.2 * right_sub)
                    * Quaternion::rotation_y(1.0 * left_sub)
                    * Quaternion::rotation_z(2.5 * left_sub + 1.0 * right_sub);
            next.hand_r.orientation =
                Quaternion::rotation_x(push * 2.0 + footrotl * -0.2 * left_sub)
                    * Quaternion::rotation_y(-1.0 * right_sub)
                    * Quaternion::rotation_z(-2.5 * right_sub - 1.0 * left_sub);
            next.torso.orientation = Quaternion::rotation_y(orientation.y / -1.5);
            next.chest.orientation = Quaternion::rotation_y(orientation.y / 3.0)
                * Quaternion::rotation_z(shortalt * -0.2);
            next.head.orientation = Quaternion::rotation_z(shortalt * 0.25)
                * Quaternion::rotation_z(orientation.y / 2.0)
                * Quaternion::rotation_x(-0.1);
        };

        next
    }
}
