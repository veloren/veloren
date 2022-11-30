use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::comp::item::ToolKind;
use core::{f32::consts::PI, ops::Mul};

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency<'a> = (Option<ToolKind>, Option<ToolKind>, f32);
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_idle")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, _second_tool_kind, global_time): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let lab: f32 = 1.0;
        let torso = (anim_time * lab + 1.5 * PI).sin() * 1.5;

        let slower = (anim_time * 2.0 + PI).sin() * 1.5;
        let slow = (anim_time * 7.0 + PI).sin() * 1.5;

        let look = Vec2::new(
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

        let breathe = if s_a.beast {
            // Controls for the beast breathing
            let intensity = 0.04;
            let length = 1.5;
            let chop = 0.2;
            let chop_freq = 60.0;
            intensity * (length * anim_time).sin()
                + 0.05 * chop * (anim_time * chop_freq).sin() * (anim_time * length).cos()
        } else {
            0.0
        };

        next.jaw.scale = Vec3::one() * 1.02;
        next.shoulder_l.scale = Vec3::one() * 1.1;
        next.shoulder_r.scale = Vec3::one() * 1.1;
        next.hand_l.scale = Vec3::one() * 1.04;
        next.hand_r.scale = Vec3::one() * 1.04;
        next.lower_torso.scale = Vec3::one() * 1.02;
        next.hold.scale = Vec3::one() * 0.0;
        next.second.scale = Vec3::one() * 0.0;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1 + torso * 0.2);
        next.head.orientation =
            Quaternion::rotation_z(look.x * 0.6) * Quaternion::rotation_x(look.y * 0.6 + breathe);

        next.upper_torso.position =
            Vec3::new(0.0, s_a.upper_torso.0, s_a.upper_torso.1 + torso * -0.5);
        next.upper_torso.orientation = Quaternion::rotation_x(-breathe);

        next.lower_torso.position =
            Vec3::new(0.0, s_a.lower_torso.0, s_a.lower_torso.1 + torso * 0.5);
        next.lower_torso.orientation = Quaternion::rotation_x(breathe);

        if s_a.beast {
            next.jaw.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
        } else {
            next.jaw.position = Vec3::new(0.0, s_a.jaw.0 - slower * 0.12, s_a.jaw.1 + slow * 0.2);
        }
        next.jaw.orientation = Quaternion::rotation_x(-0.1 + breathe * 2.0);

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.tail.orientation =
            Quaternion::rotation_z(0.0 + slow * 0.2) * Quaternion::rotation_x(0.0);

        match active_tool_kind {
            Some(ToolKind::Bow) => {
                next.main.position = Vec3::new(0.0, -6.0, 0.0);
                next.main.orientation =
                    Quaternion::rotation_y(2.5) * Quaternion::rotation_z(PI / 2.0);
            },
            Some(ToolKind::Staff) | Some(ToolKind::Sceptre) => {
                next.main.position = Vec3::new(-6.0, -5.0, -12.0);
                next.main.orientation =
                    Quaternion::rotation_y(0.6) * Quaternion::rotation_z(PI / 2.0);
            },
            Some(ToolKind::Sword) => {
                next.main.position = Vec3::new(-10.0, -8.0, 12.0);
                next.main.orientation =
                    Quaternion::rotation_y(2.5) * Quaternion::rotation_z(PI / 2.0);
            },
            Some(ToolKind::Hammer) | Some(ToolKind::Axe) => {
                next.main.position = Vec3::new(-6.0, -8.0, 8.0);
                next.main.orientation =
                    Quaternion::rotation_y(2.5) * Quaternion::rotation_z(PI / 2.0);
                next.second.position = Vec3::new(6.0, -8.0, 8.0);
                next.second.orientation =
                    Quaternion::rotation_y(-2.5) * Quaternion::rotation_z(PI / 2.0);
            },
            _ => {
                next.main.position = Vec3::new(-2.0, -5.0, -6.0);
                next.main.orientation =
                    Quaternion::rotation_y(0.6) * Quaternion::rotation_z(PI / 2.0);
            },
        }

        next.shoulder_l.position = Vec3::new(-s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_l.orientation = Quaternion::rotation_x(breathe);

        next.shoulder_r.position = Vec3::new(s_a.shoulder.0, s_a.shoulder.1, s_a.shoulder.2);
        next.shoulder_r.orientation = Quaternion::rotation_x(breathe);

        next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2 + torso * -0.1);

        next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2 + torso * -0.1);

        next.leg_l.position = Vec3::new(-s_a.leg.0, s_a.leg.1, s_a.leg.2 + torso * -0.2);

        next.leg_r.position = Vec3::new(s_a.leg.0, s_a.leg.1, s_a.leg.2 + torso * -0.2);

        next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2);

        next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2);

        next.torso.position = Vec3::new(0.0, 0.0, 0.0);

        if s_a.float {
            next.upper_torso.position = Vec3::new(
                0.0,
                s_a.upper_torso.0,
                s_a.upper_torso.1 + slower * 1.0 + 4.0,
            );
            next.foot_l.orientation = Quaternion::rotation_x(-0.5 + slow * 0.1);
            next.foot_r.orientation = Quaternion::rotation_x(-0.5 + slow * 0.1);
        }

        next
    }
}
