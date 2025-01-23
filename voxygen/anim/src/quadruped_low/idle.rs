use common::comp::body::parts::HeadState;

use super::{
    super::{Animation, vek::*},
    QuadrupedLowSkeleton, SkeletonAttr,
};
use std::{f32::consts::PI, ops::Mul};

pub struct IdleAnimation;

impl Animation for IdleAnimation {
    type Dependency<'a> = (f32, [HeadState; 3]);
    type Skeleton = QuadrupedLowSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_low_idle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_low_idle")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (global_time, head_states): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let slower = (anim_time * 1.25).sin();
        let slow = (anim_time * 2.5).sin();
        let slowalt = (anim_time * 2.5 + PI / 2.0).sin();

        let dragon_look = |a: f32, b: f32| {
            Vec2::new(
                (global_time / 2.0 + anim_time / 8.0).floor().mul(a).sin() * 0.2,
                (global_time / 2.0 + anim_time / 8.0).floor().mul(b).sin() * 0.1,
            )
        };

        let dragon_look1 = dragon_look(7331.0, 1337.0);
        let dragon_look2 = dragon_look(1553.0, 7777.0);
        let dragon_look3 = dragon_look(3551.0, 4587.0);

        next.tail_front.scale = Vec3::one() * 0.98;
        next.tail_rear.scale = Vec3::one() * 0.98;

        // Central head
        next.jaw_c.scale = Vec3::one() * 0.98;
        next.head_c_upper.position =
            Vec3::new(0.0, s_a.head_upper.0, s_a.head_upper.1 + slower * 0.2);
        next.head_c_upper.orientation = Quaternion::rotation_z(0.8 * dragon_look1.x)
            * Quaternion::rotation_x(0.8 * dragon_look1.y);

        next.head_c_lower.position =
            Vec3::new(0.0, s_a.head_lower.0, s_a.head_lower.1 + slower * 0.20);
        next.head_c_lower.orientation = Quaternion::rotation_z(0.8 * dragon_look1.x)
            * Quaternion::rotation_x(0.8 * dragon_look1.y);
        next.head_c_lower.scale = Vec3::one() * (head_states[1].is_attached() as i32 as f32);

        next.jaw_c.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
        next.jaw_c.orientation = Quaternion::rotation_x(slow * 0.05 - 0.05);

        // Left head
        next.jaw_l.scale = Vec3::one() * 0.98;
        next.head_l_upper.position = Vec3::new(
            -s_a.side_head_upper.0,
            s_a.side_head_upper.1,
            s_a.side_head_upper.2 + slower * 0.2,
        );
        next.head_l_upper.orientation = Quaternion::rotation_z(0.8 * dragon_look2.x)
            * Quaternion::rotation_x(0.8 * dragon_look2.y);

        next.head_l_lower.position = Vec3::new(
            -s_a.side_head_lower.0,
            s_a.side_head_lower.1,
            s_a.side_head_lower.2 + slower * 0.20,
        );
        next.head_l_lower.orientation = Quaternion::rotation_z(0.8 * dragon_look2.x)
            * Quaternion::rotation_x(0.8 * dragon_look2.y)
            * Quaternion::rotation_y(-dragon_look1.x.max(0.0) + dragon_look2.x.min(0.0));
        next.head_l_lower.scale = Vec3::one() * (head_states[0].is_attached() as i32 as f32);

        next.jaw_l.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
        next.jaw_l.orientation = Quaternion::rotation_x(slow * 0.05 - 0.05);

        // Right head
        next.jaw_r.scale = Vec3::one() * 0.98;
        next.head_r_upper.position = Vec3::new(
            s_a.side_head_upper.0,
            s_a.side_head_upper.1,
            s_a.side_head_upper.2 + slower * 0.2,
        );
        next.head_r_upper.orientation = Quaternion::rotation_z(0.8 * dragon_look3.x)
            * Quaternion::rotation_x(0.8 * dragon_look3.y);

        next.head_r_lower.position = Vec3::new(
            s_a.side_head_lower.0,
            s_a.side_head_lower.1,
            s_a.side_head_lower.2 + slower * 0.20,
        );
        next.head_r_lower.orientation = Quaternion::rotation_z(0.8 * dragon_look3.x)
            * Quaternion::rotation_x(0.8 * dragon_look3.y)
            * Quaternion::rotation_y(-dragon_look1.x.min(0.0) + dragon_look3.x.max(0.0));
        next.head_r_lower.scale = Vec3::one() * (head_states[2].is_attached() as i32 as f32);

        next.jaw_r.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);
        next.jaw_r.orientation = Quaternion::rotation_x(slow * 0.05 - 0.05);

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);
        next.chest.orientation = Quaternion::rotation_y(slow * 0.03);
        if s_a.tongue_for_tail {
            next.tail_front.position = Vec3::new(0.0, s_a.tail_front.0, s_a.tail_front.1);
            next.tail_rear.position = Vec3::new(0.0, s_a.tail_rear.0, s_a.tail_rear.1);
        } else {
            next.tail_front.position = Vec3::new(0.0, s_a.tail_front.0, s_a.tail_front.1);
            next.tail_front.orientation =
                Quaternion::rotation_x(0.15) * Quaternion::rotation_z(slowalt * 0.12);

            next.tail_rear.position = Vec3::new(0.0, s_a.tail_rear.0, s_a.tail_rear.1);
            next.tail_rear.orientation =
                Quaternion::rotation_z(slowalt * 0.12) * Quaternion::rotation_x(-0.12);
        }
        next.foot_fl.position = Vec3::new(-s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2);
        next.foot_fl.orientation = Quaternion::rotation_y(slow * -0.05);

        next.foot_fr.position = Vec3::new(s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2);
        next.foot_fr.orientation = Quaternion::rotation_y(slow * -0.05);

        next.foot_bl.position = Vec3::new(-s_a.feet_b.0, s_a.feet_b.1, s_a.feet_b.2);
        next.foot_bl.orientation = Quaternion::rotation_y(slow * -0.05);

        next.foot_br.position = Vec3::new(s_a.feet_b.0, s_a.feet_b.1, s_a.feet_b.2);
        next.foot_br.orientation = Quaternion::rotation_y(slow * -0.05);

        next
    }
}
