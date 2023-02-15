use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};
use core::f32::consts::PI;

pub struct LeapShockAnimation;

impl Animation for LeapShockAnimation {
    type Dependency<'a> = (
        Option<ToolKind>,
        Option<ToolKind>,
        Vec3<f32>,
        f32,
        Option<StageSection>,
    );
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_leapshockwave\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_leapshockwave")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, _second_tool_kind, _velocity, _global_time, stage_section): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (movement1, movement2, movement3, movement4) = match stage_section {
            Some(StageSection::Buildup) => (anim_time, 0.0, 0.0, 0.0),
            Some(StageSection::Movement) => (1.0, anim_time.powf(0.25), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, 1.0, anim_time.powf(0.25), 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0, 0.0),
        };

        if true {
            next.hand_l.position = Vec3::new(s_a.hhl.0, s_a.hhl.1, s_a.hhl.2);
            next.hand_l.orientation = Quaternion::rotation_x(s_a.hhl.3);
            next.hand_r.position = Vec3::new(s_a.hhr.0, s_a.hhr.1, s_a.hhr.2);
            next.hand_r.orientation = Quaternion::rotation_x(s_a.hhr.3);
            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_y(0.0) * Quaternion::rotation_z(0.0);

            next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

            next.control.position = Vec3::new(
                s_a.hc.0 + movement2 * -10.0 + movement3 * 6.0,
                s_a.hc.1 + movement2 * 5.0 + movement3 * 7.0,
                s_a.hc.2 + movement2 * 5.0 + movement3 * -10.0,
            );
            next.control.orientation =
                Quaternion::rotation_x(s_a.hc.3 + movement2 * PI / 2.0 + movement3 * -2.3)
                    * Quaternion::rotation_y(s_a.hc.4 + movement2 * 1.3)
                    * Quaternion::rotation_z(s_a.hc.5 + movement2 * -1.0 + movement3 * 0.5);
            next.upper_torso.orientation =
                Quaternion::rotation_x(
                    movement1 * 0.3 + movement2 * 0.3 + movement3 * -0.9 + movement4 * 0.3,
                ) * Quaternion::rotation_z(movement1 * 0.5 + movement2 * 0.2 + movement3 * -0.7);

            next.head.orientation = Quaternion::rotation_x(movement3 * 0.2)
                * Quaternion::rotation_y(0.0 + movement2 * -0.1)
                * Quaternion::rotation_z(movement1 * -0.4 + movement2 * -0.2 + movement3 * 0.6);

            //next.hand_l.position = Vec3::new(-12.0 + movement3 * 10.0, 0.0, 0.0);

            next.foot_l.position = Vec3::new(
                -s_a.foot.0,
                s_a.foot.1 + movement3 * 13.0,
                s_a.foot.2 + movement3 * -2.0,
            );
            next.foot_l.orientation = Quaternion::rotation_x(-0.8 + movement3 * 1.7);

            next.foot_r.position = Vec3::new(
                s_a.foot.0,
                s_a.foot.1 + 8.0 + movement3 * -13.0,
                s_a.foot.2 + 5.0 + movement3 * -5.0,
            );
            next.foot_r.orientation = Quaternion::rotation_x(0.9 + movement3 * -1.7);
        }

        next
    }
}
