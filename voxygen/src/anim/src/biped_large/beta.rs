use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};

pub struct BetaAnimation;

impl Animation for BetaAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        f32,
        f64,
        Option<StageSection>,
    );
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_beta\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_beta")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, _second_tool_kind, _velocity, _global_time, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (movement1, movement2, movement3) = match stage_section {
            Some(StageSection::Buildup) => ((anim_time as f32).powf(0.25), 0.0, 0.0),
            Some(StageSection::Swing) => (1.0, anim_time as f32, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, (anim_time as f32).powf(4.0)),
            _ => (0.0, 0.0, 0.0),
        };

        match active_tool_kind {
            Some(ToolKind::Sword(_)) => {
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position = Vec3::new(s_a.shr.0, s_a.shr.1, s_a.shr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.shr.3) * Quaternion::rotation_y(s_a.shr.4);

                next.control.position = Vec3::new(
                    s_a.sc.0 + (-1.4 + movement1 * -3.0 + movement2 * -2.0) * (1.0 - movement3),
                    s_a.sc.1 + (-1.4 + movement1 * 3.0 + movement2 * 3.0) * (1.0 - movement3),
                    s_a.sc.2 + (-1.9 + movement1 * 2.5 * (1.0 - movement3)),
                );
                next.control.orientation =
                    Quaternion::rotation_x(s_a.sc.3 + (-1.7) * (1.0 - movement3))
                        * Quaternion::rotation_y(
                            s_a.sc.4
                                + (0.4 + movement1 * 1.5 + movement2 * -2.5) * (1.0 - movement3),
                        )
                        * Quaternion::rotation_z(
                            s_a.sc.5 + (1.67 + movement2 * 1.57) * (1.0 - movement3),
                        );
                next.upper_torso.orientation = Quaternion::rotation_x(0.15)
                    * Quaternion::rotation_y((-0.1) * (1.0 - movement3))
                    * Quaternion::rotation_z(
                        (0.4 + movement1 * 1.5 + movement2 * -2.5) * (1.0 - movement3),
                    );
                next.head.orientation = Quaternion::rotation_z((-0.4) * (1.0 - movement3));
            },
            _ => {},
        }

        next
    }
}
