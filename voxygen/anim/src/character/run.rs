use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::comp::item::{Hands, ToolKind};
use core::{f32::consts::PI, ops::Mul};

pub struct RunAnimation;

type RunAnimationDependency = (
    Option<ToolKind>,
    Option<ToolKind>,
    (Option<Hands>, Option<Hands>),
    Vec3<f32>,
    Vec3<f32>,
    Vec3<f32>,
    f32,
    Vec3<f32>,
    f32,
    Option<Vec3<f32>>,
);

impl Animation for RunAnimation {
    type Dependency<'a> = RunAnimationDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_run\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_run")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            active_tool_kind,
            second_tool_kind,
            hands,
            velocity,
            orientation,
            last_ori,
            global_time,
            avg_vel,
            acc_vel,
            wall,
        ): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let speed = Vec2::<f32>::from(velocity).magnitude();
        *rate = 1.0;
        let impact = (avg_vel.z).max(-8.0);
        let speednorm = (speed / 9.4).powf(0.65);

        let lab: f32 = 0.6 / s_a.scaler.powf(0.75);

        let footrotl = ((1.0 / (0.5 + (0.5) * ((acc_vel * 1.6 * lab + PI * 1.4).sin()).powi(2)))
            .sqrt())
            * ((acc_vel * 1.6 * lab + PI * 1.4).sin());

        let footrotr = ((1.0 / (0.5 + (0.5) * ((acc_vel * 1.6 * lab + PI * 0.4).sin()).powi(2)))
            .sqrt())
            * ((acc_vel * 1.6 * lab + PI * 0.4).sin());

        let noisea = (acc_vel * 11.0 + PI / 6.0).sin();
        let noiseb = (acc_vel * 19.0 + PI / 4.0).sin();

        let back_speed = 2.6;

        let dirside = orientation.xy().dot(velocity.xy()).signum();
        let foothoril = if dirside > 0.0 {
            (acc_vel * 1.6 * lab + PI * 1.45).sin() * dirside
        } else {
            (acc_vel * back_speed * lab + PI * 1.45).sin() * dirside
        };
        let foothorir = if dirside > 0.0 {
            (acc_vel * 1.6 * lab + PI * (0.45)).sin() * dirside
        } else {
            (acc_vel * back_speed * lab + PI * (0.45)).sin() * dirside
        };
        let strafeside = orientation
            .xy()
            .dot(velocity.xy().rotated_z(PI * -0.5))
            .signum();
        let footstrafel = (acc_vel * 1.6 * lab + PI * 1.5).sin() * strafeside;
        let footstrafer = (acc_vel * 1.6 * lab + PI).sin() * -strafeside;

        let footvertl = if dirside > 0.0 {
            (acc_vel * 1.6 * lab).sin()
        } else {
            (acc_vel * back_speed * lab).sin()
        };
        let footvertr = if dirside > 0.0 {
            (acc_vel * 1.6 * lab + PI).sin()
        } else {
            (acc_vel * back_speed * lab + PI).sin()
        };
        let footvertsl = (acc_vel * 1.6 * lab).sin();
        let footvertsr = (acc_vel * 1.6 * lab + PI * 0.5).sin();

        let shortalt = (acc_vel * lab * 3.2 + PI / 1.0).sin();
        let shortalt2 = (acc_vel * lab * 3.2).sin();

        let short = ((5.0 / (1.5 + 3.5 * ((acc_vel * lab * 1.6 + PI * 0.5).sin()).powi(2))).sqrt())
            * ((acc_vel * lab * 1.6 + PI * 0.5).sin());

        let side =
            (velocity.x * -0.098 * orientation.y + velocity.y * 0.098 * orientation.x) * -1.0;
        let sideabs = side.abs();
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

        let head_look = Vec2::new(
            (global_time + anim_time / 18.0).floor().mul(7331.0).sin() * 0.2,
            (global_time + anim_time / 18.0).floor().mul(1337.0).sin() * 0.1,
        );

        next.head.position = Vec3::new(0.0, s_a.head.0 * 1.5, s_a.head.1 + short * 0.1);
        next.head.orientation =
            Quaternion::rotation_z(tilt * -2.5 + head_look.x * 0.2 + short * -0.3)
                * Quaternion::rotation_x(head_look.y + 0.45 * speednorm + shortalt2 * -0.05);
        next.head.scale = Vec3::one() * s_a.head_scale;

        next.chest.position = Vec3::new(
            0.0,
            s_a.chest.0,
            s_a.chest.1 + 1.0 * speednorm + shortalt * 1.1,
        );
        next.chest.orientation = Quaternion::rotation_x(impact * 0.07)
            * Quaternion::rotation_z(short * 0.4 + tilt * -0.6)
            * Quaternion::rotation_y(tilt * 2.0 + short * 0.2)
            * Quaternion::rotation_x(shortalt2 * 0.03 + speednorm * -0.5 + tilt.abs());

        next.belt.position = Vec3::new(0.0, 0.25 + s_a.belt.0, 0.25 + s_a.belt.1);
        next.belt.orientation = Quaternion::rotation_x(0.1 * speednorm)
            * Quaternion::rotation_z(short * -0.2 + tilt * -1.1)
            * Quaternion::rotation_y(tilt * 0.5);

        next.back.position = Vec3::new(0.0, s_a.back.0, s_a.back.1);
        next.back.orientation =
            Quaternion::rotation_x(-0.05 + short * 0.02 + noisea * 0.02 + noiseb * 0.02)
                * Quaternion::rotation_y(foothorir * 0.2);

        next.shorts.position = Vec3::new(0.0, 0.65 + s_a.shorts.0, 0.65 * speednorm + s_a.shorts.1);
        next.shorts.orientation = Quaternion::rotation_x(0.2 * speednorm)
            * Quaternion::rotation_z(short * -0.9 + tilt * -1.5)
            * Quaternion::rotation_y(tilt * 0.7 + short * 0.08);

        next.hand_l.position = Vec3::new(
            -s_a.hand.0 * 1.2 - foothorir * 1.3 * speednorm
                + (foothoril.abs().powf(2.0) - 0.5) * speednorm * 4.0,
            s_a.hand.1 * 1.3 + foothorir * -7.0 * speednorm * (1.0 - sideabs),
            s_a.hand.2 - foothorir * 2.75 * speednorm + foothoril.abs().powf(3.0) * speednorm * 8.0,
        );
        next.hand_l.orientation =
            Quaternion::rotation_x(
                0.6 * speednorm + (footrotr * -1.5 + 0.5) * speednorm * (1.0 - sideabs),
            ) * Quaternion::rotation_y(footrotr * 0.4 * speednorm + PI * 0.07);

        next.hand_r.position = Vec3::new(
            s_a.hand.0 * 1.2 + foothoril * 1.3 * speednorm
                - (foothorir.abs().powf(2.0) - 0.5) * speednorm * 4.0,
            s_a.hand.1 * 1.3 + foothoril * -7.0 * speednorm * (1.0 - sideabs),
            s_a.hand.2 - foothoril * 2.75 * speednorm + foothorir.abs().powf(3.0) * speednorm * 8.0,
        );
        next.hand_r.orientation =
            Quaternion::rotation_x(
                0.6 * speednorm + (footrotl * -1.5 + 0.5) * speednorm * (1.0 - sideabs),
            ) * Quaternion::rotation_y(footrotl * -0.4 * speednorm - PI * 0.07);

        next.foot_l.position = Vec3::new(
            -s_a.foot.0 + footstrafel * sideabs * 7.0 + tilt * -10.0,
            s_a.foot.1 + (1.0 - sideabs) * (-1.5 * speednorm + foothoril * -10.0 * speednorm),
            s_a.foot.2
                + (1.0 - sideabs) * (1.25 * speednorm + ((footvertl * -5.0 * speednorm).max(-1.0)))
                + side * ((footvertsl * 1.5).max(-1.0)),
        );
        next.foot_l.orientation = Quaternion::rotation_x(
            (1.0 - sideabs) * (foothoril + 0.3 * (1.0 - sideabs)) * -1.5 * speednorm
                + sideabs * -0.5,
        ) * Quaternion::rotation_y(
            tilt * -0.5 + side * (foothoril * 0.3) + footstrafer * side * 0.5,
        ) * Quaternion::rotation_z(
            side * 1.3 * orientation.xy().dot(velocity.xy() / (speed + 0.01)),
        );

        next.foot_r.position = Vec3::new(
            s_a.foot.0 + footstrafer * sideabs * 7.0 + tilt * -10.0,
            s_a.foot.1 + (1.0 - sideabs) * (-1.5 * speednorm + foothorir * -10.0 * speednorm),
            s_a.foot.2
                + (1.0 - sideabs) * (1.25 * speednorm + ((footvertr * -5.0 * speednorm).max(-1.0)))
                + side * ((footvertsr * -1.5).max(-1.0)),
        );
        next.foot_r.orientation = Quaternion::rotation_x(
            (1.0 - sideabs) * (foothorir + 0.3 * (1.0 - sideabs)) * -1.5 * speednorm
                + sideabs * -0.5,
        ) * Quaternion::rotation_y(
            tilt * -0.5 + side * (foothorir * 0.3) - footstrafer * side * 0.5,
        ) * Quaternion::rotation_z(
            side * 1.3 * orientation.xy().dot(velocity.xy() / (speed + 0.01)),
        );
        //

        next.shoulder_l.position = Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_l.orientation =
            Quaternion::rotation_x(short * 0.15 + (footrotl * 1.2 + 0.5) * speednorm);
        next.shoulder_l.scale = Vec3::one() * 1.1;

        next.shoulder_r.position = Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_r.orientation =
            Quaternion::rotation_x(short * -0.15 + (footrotr * 1.2 + 0.5) * speednorm);
        next.shoulder_r.scale = Vec3::one() * 1.1;

        next.glider.position = Vec3::new(0.0, 0.0, 10.0);
        next.glider.scale = Vec3::one() * 0.0;

        next.do_tools_on_back(hands, active_tool_kind, second_tool_kind);

        next.do_hold_lantern(s_a, anim_time, acc_vel, speednorm, impact, tilt);

        next.torso.position = Vec3::new(0.0, 0.0, 0.0);

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
        };

        next
    }
}
