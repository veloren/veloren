use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};

pub struct Input {
    pub attack: bool,
}
pub struct ShockwaveAnimation;

impl Animation for ShockwaveAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        f64,
        f32,
        Option<StageSection>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_shockwave\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_shockwave")]
    #[allow(clippy::single_match)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_active_tool_kind, _second_tool_kind, _global_time, velocity, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (movement1, movement2, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time as f32, 0.0, 0.0),
            Some(StageSection::Swing) => (1.0, anim_time as f32, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time as f32),
            _ => (0.0, 0.0, 0.0),
        };

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

        next.hand_l.position = Vec3::new(s_a.sthl.0, s_a.sthl.1, s_a.sthl.2);
        next.hand_l.orientation =
            Quaternion::rotation_x(s_a.sthl.3) * Quaternion::rotation_y(s_a.sthl.4);
        next.hand_r.position = Vec3::new(s_a.sthr.0, s_a.sthr.1, s_a.sthr.2);
        next.hand_r.orientation =
            Quaternion::rotation_x(s_a.sthr.3) * Quaternion::rotation_y(s_a.sthr.4);
        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_x(0.0);

        next.control.position = Vec3::new(s_a.stc.0, s_a.stc.1, s_a.stc.2);
        next.control.orientation =
            Quaternion::rotation_x(s_a.stc.3) * Quaternion::rotation_y(s_a.stc.4);

        let twist = movement1 * 0.8;

        next.control.position = Vec3::new(
            s_a.stc.0 + (movement1 * 5.0) * (1.0 - movement3),
            s_a.stc.1 + (movement1 * 5.0) * (1.0 - movement3),
            s_a.stc.2 + (movement1 * 10.0 + movement2 * -10.0) * (1.0 - movement3),
        );
        next.control.orientation =
            Quaternion::rotation_x(s_a.stc.3 + (movement1 * 0.8) * (1.0 - movement3))
                * Quaternion::rotation_y(
                    s_a.stc.4 + (movement1 * -0.15 + movement2 * -0.15) * (1.0 - movement3),
                )
                * Quaternion::rotation_z((movement1 * 0.8 + movement2 * -0.8) * (1.0 - movement3));

        next.head.orientation = Quaternion::rotation_x((movement1 * 0.4) * (1.0 - movement3))
            * Quaternion::rotation_z((twist * 0.2 + movement2 * -0.8) * (1.0 - movement3));

        next.chest.position = Vec3::new(
            0.0,
            s_a.chest.0,
            s_a.chest.1 + (movement1 * 2.0 + movement2 * -4.0) * (1.0 - movement3),
        );
        next.chest.orientation = Quaternion::rotation_x((movement2 * -0.8) * (1.0 - movement3))
            * Quaternion::rotation_z(twist * -0.2 + movement2 * -0.1 + (1.0 - movement3));

        next.belt.orientation = Quaternion::rotation_x((movement2 * 0.2) * (1.0 - movement3))
            * Quaternion::rotation_z((twist * 0.6 + movement2 * -0.48) * (1.0 - movement3));

        next.shorts.orientation = Quaternion::rotation_x((movement2 * 0.3) * (1.0 - movement3))
            * Quaternion::rotation_z((twist + movement2 * -0.8) * (1.0 - movement3));

        if velocity < 0.5 {
            next.foot_l.position = Vec3::new(
                -s_a.foot.0,
                s_a.foot.1 + movement1 * -7.0 + movement2 * 7.0,
                s_a.foot.2,
            );
            next.foot_l.orientation = Quaternion::rotation_x(movement1 * -0.8 + movement2 * 0.8)
                * Quaternion::rotation_z(movement1 * 0.3 + movement2 * -0.3);

            next.foot_r.position = Vec3::new(
                s_a.foot.0,
                s_a.foot.1 + movement1 * 5.0 + movement2 * -5.0,
                s_a.foot.2,
            );
            next.foot_r.orientation = Quaternion::rotation_y(movement1 * -0.3 + movement2 * 0.3)
                * Quaternion::rotation_z(movement1 * 0.4 + movement2 * -0.4);
        }
        next
    }
}
