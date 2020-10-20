use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::{
    comp::item::ToolKind,
    states::utils::StageSection,
};

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
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (movement1, movement2, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time as f32, 0.0, 0.0),
            Some(StageSection::Cast) => (1.0, anim_time as f32, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time as f32),
            _ => (0.0, 0.0, 0.0),
        };

        match active_tool_kind {
            Some(ToolKind::Sword(_)) => {
                next.hand_l.position = Vec3::new(-4.75, -1.0, 2.5);
                next.hand_l.orientation = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(-0.2);
                next.hand_l.scale = Vec3::one() * 1.04;
                next.hand_r.position = Vec3::new(0.75, -1.5, -0.5);
                next.hand_r.orientation = Quaternion::rotation_x(1.47) * Quaternion::rotation_y(0.3);
                next.hand_r.scale = Vec3::one() * 1.05;
                next.main.position = Vec3::new(0.0, 5.0, -6.0);
                next.main.orientation = Quaternion::rotation_x(-0.1)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);

                next.head.position = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1);

                next.control.position = Vec3::new(
                    -8.0 + movement1 * -5.0 + (movement2 as f32 * 2.5).sin() * 30.0 + movement3 * -5.0,
                    1.0 - (movement1 as f32 * 8.0).sin() * 0.8 + movement1 * 2.0 + movement3 * 2.0,
                    2.0 - (movement1 as f32 * 8.0).sin() * 0.4,
                );
                next.control.orientation = Quaternion::rotation_x(-1.57)
                    * Quaternion::rotation_y(
                        0.0 + movement1 * 1.5 + (movement2 as f32 * 2.5).sin() * 0.5,
                    )
                    * Quaternion::rotation_z(1.0 + (movement2 as f32 * 2.5).sin() * 1.0);
                next.upper_torso.orientation = Quaternion::rotation_y(-0.1)
                    * Quaternion::rotation_z(
                        0.4 + movement1 * 1.5 + (movement2 as f32 * 2.5).sin() * -0.5 + movement3 * 1.0,
                    );
                next.head.orientation = Quaternion::rotation_y(0.1)
                    * Quaternion::rotation_z(
                        -0.1 + movement1 * -1.1 + (movement2 as f32 * 2.5).sin() * -0.5,
                    );
            }
            _ => {},
        }

        next.hold.scale = Vec3::one() * 0.0;

        next
    }
}