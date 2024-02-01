use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use std::ops::Mul;

pub struct GlidingAnimation;

type GlidingAnimationDependency = (Vec3<f32>, Quaternion<f32>, Quaternion<f32>, f32, f32);

impl Animation for GlidingAnimation {
    type Dependency<'a> = GlidingAnimationDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_gliding\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_gliding")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (velocity, orientation, glider_orientation, global_time, acc_vel): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        // TODO: remove glider trails completely
        next.glider_trails = false;

        let speednorm = velocity.magnitude().min(50.0) / 50.0;
        let slow = (acc_vel * 0.1).sin();

        let head_look = Vec2::new(
            ((global_time + anim_time) / 4.0).floor().mul(7331.0).sin() * 0.5,
            ((global_time + anim_time) / 4.0).floor().mul(1337.0).sin() * 0.25,
        );

        let speedlog = speednorm.powi(2);
        let chest_ori = Quaternion::rotation_z(slow * 0.01);
        let chest_global_inv = (orientation * chest_ori).inverse();
        let glider_ori = chest_global_inv * glider_orientation;
        let glider_pos = Vec3::new(0.0, -5.0 + speedlog * 2.0, 13.0);

        next.head.orientation = Quaternion::rotation_x(0.5 + head_look.y * speednorm)
            * Quaternion::rotation_z(head_look.x);

        next.glider.position = glider_pos;
        next.glider.orientation = glider_ori;
        next.glider.scale = Vec3::one();

        next.chest.orientation = chest_ori;

        //necessary for overwriting jump anim
        next.belt.orientation = Quaternion::rotation_z(0.0);
        next.belt.position = Vec3::new(0.0, s_a.belt.0, s_a.belt.1);
        next.shorts.position = Vec3::new(0.0, s_a.shorts.0, s_a.shorts.1);
        next.shorts.orientation = Quaternion::rotation_z(slow * 0.15);

        next.shoulder_r.orientation = glider_ori * Quaternion::rotation_x(2.0);
        next.shoulder_l.orientation = next.shoulder_r.orientation;

        next.hand_l.position =
            glider_pos + glider_ori * Vec3::new(-s_a.hand.0 + -2.0, s_a.hand.1 + 8.0, s_a.hand.2);
        next.hand_l.orientation = Quaternion::rotation_x(3.35) * Quaternion::rotation_y(0.2);

        next.hand_r.position =
            glider_pos + glider_ori * Vec3::new(s_a.hand.0 + 2.0, s_a.hand.1 + 8.0, s_a.hand.2);
        next.hand_r.orientation = Quaternion::rotation_x(3.35) * Quaternion::rotation_y(-0.2);

        next.foot_l.position = Vec3::new(
            -s_a.foot.0,
            s_a.foot.1 + speedlog * -1.0 - slow * 2.3,
            s_a.foot.2,
        );
        next.foot_l.orientation = Quaternion::rotation_x(-speedlog + slow * -1.3 * speedlog);

        next.foot_r.position = Vec3::new(
            s_a.foot.0,
            s_a.foot.1 + speedlog * -1.0 + slow * 2.3,
            s_a.foot.2,
        );
        next.foot_r.orientation = Quaternion::rotation_x(-speedlog + slow * 1.3 * speedlog);

        next
    }
}
