use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::comp::item::ToolKind;

pub struct ShootAnimation;

impl Animation for ShootAnimation {
    type Dependency = (Option<ToolKind>, Option<ToolKind>, f32, f64);
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_shoot\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_shoot")]
    /*    fn update_skeleton_inner(
            skeleton: &Self::Skeleton,
            (active_tool_kind, _second_tool_kind, velocity, global_time): Self::Dependency,
            anim_time: f64,
            _rate: &mut f32,
            s_a: &SkeletonAttr,
        ) -> Self::Skeleton {
            let mut next = (*skeleton).clone();

            let lab = 0.55;
            let breathe = (anim_time as f32 + 1.5 * PI).sin();
            let test = (anim_time as f32 + 36.0 * PI).sin();

            let slower = (anim_time as f32 * 1.0 + PI).sin();
            let slow = (anim_time as f32 * 3.5 + PI).sin();

            let exp = ((anim_time as f32).powf(0.3 as f32)).min(1.2);

            let tailmove = Vec2::new(
                ((global_time + anim_time) as f32 / 2.0)
                    .floor()
                    .mul(7331.0)
                    .sin()
                    * 0.25,
                ((global_time + anim_time) as f32 / 2.0)
                    .floor()
                    .mul(1337.0)
                    .sin()
                    * 0.125,
            );

            let look = Vec2::new(
                ((global_time + anim_time) as f32 / 8.0)
                    .floor()
                    .mul(7331.0)
                    .sin()
                    * 0.5,
                ((global_time + anim_time) as f32 / 8.0)
                    .floor()
                    .mul(1337.0)
                    .sin()
                    * 0.25,
            );

            let foothoril = (((1.0)
                / (0.4
                    + (0.6)
                        * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin()).powf(2.0 as f32)))
            .sqrt())
                * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin());
            let foothorir = (((1.0)
                / (0.4
                    + (0.6)
                        * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin()).powf(2.0 as f32)))
            .sqrt())
                * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin());

            let footvertl = (anim_time as f32 * 16.0 * lab as f32).sin();
            let footvertr = (anim_time as f32 * 16.0 * lab as f32 + PI).sin();

            let footrotl = (((5.0)
                / (2.5
                    + (2.5)
                        * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin()).powf(2.0 as f32)))
            .sqrt())
                * ((anim_time as f32 * 16.0 * lab as f32 + PI * 1.4).sin());

            let footrotr = (((5.0)
                / (1.0
                    + (4.0)
                        * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin()).powf(2.0 as f32)))
            .sqrt())
                * ((anim_time as f32 * 16.0 * lab as f32 + PI * 0.4).sin());

            let short = (anim_time as f32 * lab as f32 * 16.0).sin();
            let shortalt = (anim_time as f32 * lab as f32 * 16.0 + PI / 2.0).sin();

            if velocity < 0.5 {
                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1 + breathe * 0.2) * 1.02;
                next.head.orientation =
                    Quaternion::rotation_z(look.x * 0.6) * Quaternion::rotation_x(look.y * 0.6);

                next.upper_torso.position =
                    Vec3::new(0.0, s_a.upper_torso.0, s_a.upper_torso.1 + breathe * 0.5);
                next.upper_torso.orientation =
                    Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);

                next.lower_torso.position =
                    Vec3::new(0.0, s_a.lower_torso.0, s_a.lower_torso.1 + breathe * 0.15);
                next.lower_torso.orientation =
                    Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);

                next.jaw.position = Vec3::new(0.0, s_a.jaw.0 - slower * 0.12, s_a.jaw.1 + slow * 0.2);
                next.jaw.orientation = Quaternion::rotation_x(slow * 0.05);

                next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
                next.tail.orientation =
                    Quaternion::rotation_z(0.0 + slow * 0.2 + tailmove.x) * Quaternion::rotation_x(0.0);

                next.shoulder_l.position = Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
                next.shoulder_l.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);

                next.shoulder_r.position = Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
                next.shoulder_r.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(0.0);

                next.leg_l.position =
                    Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2 + breathe * 0.2) * 1.02;
                next.leg_l.orientation = Quaternion::rotation_z(0.0);

                next.leg_r.position = Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2 + breathe * 0.2) * 1.02;
                next.leg_r.orientation = Quaternion::rotation_z(0.0);

                next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2);
                next.foot_l.orientation = Quaternion::rotation_z(0.0);

                next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2);
                next.foot_r.orientation = Quaternion::rotation_z(0.0);

                next.torso.position = Vec3::new(0.0, 0.0, 0.0) / 8.0;
                next.torso.orientation = Quaternion::rotation_z(test * 0.0);

                next.control.position = Vec3::new(7.0, 9.0, -10.0);
                next.control.orientation = Quaternion::rotation_x(test * 0.02)
                    * Quaternion::rotation_y(test * 0.02)
                    * Quaternion::rotation_z(test * 0.02);
            } else {
                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1) * 1.02;
                next.head.orientation =
                    Quaternion::rotation_z(short * -0.18) * Quaternion::rotation_x(-0.05);

                next.upper_torso.position =
                    Vec3::new(0.0, s_a.upper_torso.0, s_a.upper_torso.1 + shortalt * -1.5);
                next.upper_torso.orientation = Quaternion::rotation_z(short * 0.18);

                next.lower_torso.position = Vec3::new(0.0, s_a.lower_torso.0, s_a.lower_torso.1);
                next.lower_torso.orientation =
                    Quaternion::rotation_z(short * 0.15) * Quaternion::rotation_x(0.14);

                next.jaw.position = Vec3::new(0.0, s_a.jaw.0 - slower * 0.12, s_a.jaw.1 + slow * 0.2);
                next.jaw.orientation = Quaternion::rotation_x(slow * 0.05);

                next.tail.orientation =
                    Quaternion::rotation_z(0.0 + slow * 0.2 + tailmove.x) * Quaternion::rotation_x(0.0);

                next.shoulder_l.position = Vec3::new(
                    -s_a.shoulder.0,
                    s_a.shoulder.1 + foothoril * -1.0,
                    s_a.shoulder.2,
                );
                next.shoulder_l.orientation = Quaternion::rotation_x(0.5 + footrotl * -0.16)
                    * Quaternion::rotation_y(0.1)
                    * Quaternion::rotation_z(footrotl * 0.1);

                next.shoulder_r.position = Vec3::new(
                    s_a.shoulder.0,
                    s_a.shoulder.1 + foothorir * -1.0,
                    s_a.shoulder.2,
                );
                next.shoulder_r.orientation = Quaternion::rotation_x(0.5 + footrotr * -0.16)
                    * Quaternion::rotation_y(-0.1)
                    * Quaternion::rotation_z(footrotr * -0.1);

                next.torso.position = Vec3::new(0.0, 0.0, 0.0) / 8.0;
                next.torso.orientation = Quaternion::rotation_z(0.0) * Quaternion::rotation_x(-0.25);

                next.leg_l.orientation =
                    Quaternion::rotation_z(short * 0.18) * Quaternion::rotation_x(foothoril * 0.3);

                next.leg_r.orientation =
                    Quaternion::rotation_z(short * 0.18) * Quaternion::rotation_x(foothorir * 0.3);

                next.foot_l.position = Vec3::new(
                    -s_a.foot.0,
                    4.0 + s_a.foot.1 + foothoril * 8.5,
                    s_a.foot.2 + ((footvertl * 6.5).max(0.0)),
                );
                next.foot_l.orientation = Quaternion::rotation_x(-0.5 + footrotl * 0.85);

                next.foot_r.position = Vec3::new(
                    s_a.foot.0,
                    4.0 + s_a.foot.1 + foothorir * 8.5,
                    s_a.foot.2 + ((footvertr * 6.5).max(0.0)),
                );
                next.foot_r.orientation = Quaternion::rotation_x(-0.5 + footrotr * 0.85);
            }
            match active_tool_kind {
                Some(ToolKind::Bow(_)) => {
                    next.hand_l.position =
                        Vec3::new(-10.0 - exp * 2.0, -4.0 - exp * 4.0, -1.0 + exp * 6.0);
                    next.hand_l.orientation = Quaternion::rotation_x(1.20)
                        * Quaternion::rotation_y(-0.6 + exp * 0.8)
                        * Quaternion::rotation_z(-0.3 + exp * 0.9);

                    next.hand_r.position = Vec3::new(4.9, 3.0, -4.0);
                    next.hand_r.orientation = Quaternion::rotation_x(1.20)
                        * Quaternion::rotation_y(-0.6)
                        * Quaternion::rotation_z(-0.3);

                    next.shoulder_l.position = Vec3::new(
                        -s_a.shoulder.0,
                        s_a.shoulder.1 + foothoril * -1.0,
                        s_a.shoulder.2,
                    );
                    next.shoulder_l.orientation = Quaternion::rotation_x(1.4 + footrotl * -0.06)
                        * Quaternion::rotation_y(-0.9)
                        * Quaternion::rotation_z(footrotl * -0.05);

                    next.shoulder_r.position = Vec3::new(
                        s_a.shoulder.0,
                        s_a.shoulder.1 + foothorir * -1.0,
                        s_a.shoulder.2,
                    );
                    next.shoulder_r.orientation = Quaternion::rotation_x(1.8 + footrotr * -0.06)
                        * Quaternion::rotation_y(-0.5) //1.9
                        * Quaternion::rotation_z(footrotr * -0.05);

                    next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
                    next.jaw.orientation = Quaternion::rotation_x(-0.2);

                    next.main.position = Vec3::new(7.0, 5.0, -13.0);
                    next.main.orientation = Quaternion::rotation_x(-0.3)
                        * Quaternion::rotation_y(0.3)
                        * Quaternion::rotation_z(-0.6);

                    next.control.position = Vec3::new(6.0, 6.0, 8.0);
                    next.control.orientation = Quaternion::rotation_x(exp * 0.4);
                },
                Some(ToolKind::Staff(_)) => {
                    next.hand_l.position = Vec3::new(11.0, 5.0, -4.0);
                    next.hand_l.orientation =
                        Quaternion::rotation_x(1.27) * Quaternion::rotation_y(0.0);

                    next.hand_r.position = Vec3::new(12.0, 5.5, 2.0);
                    next.hand_r.orientation =
                        Quaternion::rotation_x(1.57) * Quaternion::rotation_y(0.2);

                    next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);

                    next.shoulder_l.position =
                        Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
                    next.shoulder_l.orientation =
                        Quaternion::rotation_z(0.0) * Quaternion::rotation_x(2.0);

                    next.shoulder_r.position =
                        Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
                    next.shoulder_r.orientation =
                        Quaternion::rotation_z(0.4) * Quaternion::rotation_x(2.0);

                    next.jaw.orientation = Quaternion::rotation_x(-0.2);

                    next.main.position = Vec3::new(10.0, 12.5, 13.2);
                    next.main.orientation = Quaternion::rotation_y(PI);

                    next.control.position = Vec3::new(-7.0, 6.0, 6.0 - exp * 5.0);
                    next.control.orientation =
                        Quaternion::rotation_x(exp * 1.3) * Quaternion::rotation_z(exp * 1.5);
                },
                _ => {},
            }

            next
        }
    }
    */
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, _second_tool_kind, velocity, _global_time): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;

        let mut next = (*skeleton).clone();

        let lab = 1.0;
        let foot = (((5.0)
            / (0.2 + 4.8 * ((anim_time as f32 * lab as f32 * 8.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 8.0).sin());
        let foote = (((5.0)
            / (0.5 + 4.5 * ((anim_time as f32 * lab as f32 * 8.0 + 1.57).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 8.0).sin());

        let exp = ((anim_time as f32).powf(0.3 as f32)).min(1.2);

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation = Quaternion::rotation_z(exp * -0.4)
            * Quaternion::rotation_x(0.0)
            * Quaternion::rotation_y(exp * 0.1);

        next.upper_torso.position =
            Vec3::new(0.0, s_a.upper_torso.0 - exp * 1.5, s_a.upper_torso.1);
        next.upper_torso.orientation = Quaternion::rotation_z(0.4 + exp * 1.0)
            * Quaternion::rotation_x(0.0 + exp * 0.2)
            * Quaternion::rotation_y(exp * -0.08);

        next.lower_torso.position =
            Vec3::new(0.0, s_a.lower_torso.0 + exp * 1.0, s_a.lower_torso.1);
        next.lower_torso.orientation = next.upper_torso.orientation * -0.08;

        match active_tool_kind {
            Some(ToolKind::Staff(_)) | Some(ToolKind::Sceptre(_)) => {
                next.hand_l.position = Vec3::new(s_a.sthl.0, s_a.sthl.1, s_a.sthl.2);
                next.hand_l.orientation = Quaternion::rotation_x(s_a.sthl.3);

                next.hand_r.position = Vec3::new(s_a.sthr.0, s_a.sthr.1, s_a.sthr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.sthr.3) * Quaternion::rotation_y(s_a.sthr.4);

                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_y(0.0);

                next.control.position = Vec3::new(
                    s_a.stc.0,
                    s_a.stc.1 + exp * 5.0,
                    10.0 + s_a.stc.2 - exp * 5.0,
                );
                next.control.orientation = Quaternion::rotation_x(s_a.stc.3 + exp * 0.4)
                    * Quaternion::rotation_y(s_a.stc.4)
                    * Quaternion::rotation_z(s_a.stc.5 + exp * 1.5);
            },
            Some(ToolKind::Bow(_)) => {
                next.hand_l.position = Vec3::new(
                    s_a.bhl.0 - exp * 2.0,
                    s_a.bhl.1 - exp * 4.0,
                    s_a.bhl.2 + exp * 6.0,
                );
                next.hand_l.orientation = Quaternion::rotation_x(s_a.bhl.3)
                    * Quaternion::rotation_y(s_a.bhl.4 + exp * 0.8)
                    * Quaternion::rotation_z(s_a.bhl.5 + exp * 0.9);
                next.hand_r.position = Vec3::new(s_a.bhr.0, s_a.bhr.1, s_a.bhr.2);
                next.hand_r.orientation = Quaternion::rotation_x(s_a.bhl.3)
                    * Quaternion::rotation_y(s_a.bhr.4)
                    * Quaternion::rotation_z(s_a.bhr.5);
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);

                next.control.position = Vec3::new(s_a.bc.0, s_a.bc.1, 4.0 + s_a.bc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.bc.3 + exp * 0.4);
            },
            _ => {},
        }
        if velocity > 0.5 {
            next.foot_l.position = Vec3::new(
                -s_a.foot.0 - foot * 1.0 + exp * -1.0,
                foote * 0.8 + exp * 1.5,
                s_a.foot.2,
            );
            next.foot_l.orientation = Quaternion::rotation_x(exp * 0.5)
                * Quaternion::rotation_z(exp * 0.4)
                * Quaternion::rotation_y(0.15);

            next.foot_r.position = Vec3::new(
                s_a.foot.0 + foot * 1.0 + exp * 1.0,
                foote * -0.8 + exp * -1.0,
                s_a.foot.2,
            );
            next.foot_r.orientation = Quaternion::rotation_x(exp * -0.5)
                * Quaternion::rotation_z(exp * 0.4)
                * Quaternion::rotation_y(0.0);
            next.torso.orientation = Quaternion::rotation_x(-0.15);
        } else {
            next.foot_l.position = Vec3::new(-s_a.foot.0, -2.5, s_a.foot.2 + exp * 2.5);
            next.foot_l.orientation =
                Quaternion::rotation_x(exp * -0.2 - 0.2) * Quaternion::rotation_z(exp * 1.0);

            next.foot_r.position = Vec3::new(s_a.foot.0, 3.5 - exp * 2.0, s_a.foot.2);
            next.foot_r.orientation =
                Quaternion::rotation_x(exp * 0.1) * Quaternion::rotation_z(exp * 0.5);
        }

        next
    }
}
