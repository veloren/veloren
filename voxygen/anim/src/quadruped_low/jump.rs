use common::comp::body::parts::HeadState;

use super::{
    super::{Animation, vek::*},
    QuadrupedLowSkeleton, SkeletonAttr,
};

pub struct JumpAnimation;

impl Animation for JumpAnimation {
    type Dependency<'a> = (f32, f32, &'a [HeadState]);
    type Skeleton = QuadrupedLowSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"quadruped_low_jump\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "quadruped_low_jump"))]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_global_time, _, head_states): Self::Dependency<'_>,
        _anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        next.tail_front.scale = Vec3::one() * 0.98;
        next.tail_rear.scale = Vec3::one() * 0.98;

        // Central head
        next.head_c_upper.position = Vec3::new(0.0, s_a.head_upper.0, s_a.head_upper.1);

        next.head_c_lower.position = Vec3::new(0.0, s_a.head_lower.0, s_a.head_lower.1);
        next.head_c_lower.scale = Vec3::one() * (head_states[1].is_attached() as i32 as f32);

        next.jaw_c.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);

        // Left head
        next.jaw_l.scale = Vec3::one() * 0.98;
        next.head_l_upper.position = Vec3::new(
            -s_a.side_head_upper.0,
            s_a.side_head_upper.1,
            s_a.side_head_upper.2,
        );

        next.head_l_lower.position = Vec3::new(
            -s_a.side_head_lower.0,
            s_a.side_head_lower.1,
            s_a.side_head_lower.2,
        );
        next.head_l_lower.scale = Vec3::one() * (head_states[0].is_attached() as i32 as f32);

        next.jaw_l.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);

        // Right head
        next.jaw_r.scale = Vec3::one() * 0.98;
        next.head_r_upper.position = Vec3::new(
            s_a.side_head_upper.0,
            s_a.side_head_upper.1,
            s_a.side_head_upper.2,
        );

        next.head_r_lower.position = Vec3::new(
            s_a.side_head_lower.0,
            s_a.side_head_lower.1,
            s_a.side_head_lower.2,
        );
        next.head_r_lower.scale = Vec3::one() * (head_states[2].is_attached() as i32 as f32);

        next.jaw_r.position = Vec3::new(0.0, s_a.jaw.0, s_a.jaw.1);

        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);
        if s_a.tongue_for_tail {
            next.tail_front.scale = Vec3::one() * 0.1;
            next.tail_rear.scale = Vec3::one() * 0.1;
        } else {
            next.tail_front.position = Vec3::new(0.0, s_a.tail_front.0, s_a.tail_front.1);

            next.tail_rear.position = Vec3::new(0.0, s_a.tail_rear.0, s_a.tail_rear.1);
        }
        next.foot_fl.position = Vec3::new(-s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2);

        next.foot_fr.position = Vec3::new(s_a.feet_f.0, s_a.feet_f.1, s_a.feet_f.2);

        next.foot_bl.position = Vec3::new(-s_a.feet_b.0, s_a.feet_b.1, s_a.feet_b.2);

        next.foot_br.position = Vec3::new(s_a.feet_b.0, s_a.feet_b.1, s_a.feet_b.2);

        next
    }
}
