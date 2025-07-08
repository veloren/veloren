use super::{
    super::{Animation, vek::*},
    BipedSmallSkeleton, SkeletonAttr,
};
use common::{comp::tool::ToolKind, states::utils::StageSection};
use core::f32::consts::PI;

pub struct BuffAnimation;

impl Animation for BuffAnimation {
    type Dependency<'a> = (Option<ToolKind>, Option<ToolKind>, Option<StageSection>);
    type Skeleton = BipedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_small_selfbuff\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "biped_small_selfbuff"))]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, stage_section): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (move1base, movement3, tensionbase, tension2base) = match stage_section {
            Some(StageSection::Buildup) => (
                (anim_time.powf(0.25)).min(1.0),
                0.0,
                (anim_time * 10.0).sin(),
                0.0,
            ),
            Some(StageSection::Action) => {
                (1.0, 0.0, (anim_time * 30.0).sin(), (anim_time * 12.0).sin())
            },
            Some(StageSection::Recover) => (1.0, anim_time.powi(4), 1.0, 1.0),
            _ => (0.0, 0.0, 0.0, 0.0),
        };

        let pullback = 1.0 - movement3;
        let move1 = move1base * pullback;
        let tension = tensionbase * pullback;
        let tension2 = tension2base * pullback;
        let slow = (anim_time * 3.0).sin();

        match (active_tool_kind, second_tool_kind) {
            (Some(ToolKind::Axe), Some(ToolKind::Axe)) => {
                next.main.position = Vec3::new(-8.0, 2.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);
                next.second.position = Vec3::new(8.0, 2.0, 0.0);
                next.second.orientation = Quaternion::rotation_x(0.0);

                next.hand_l.position = Vec3::new(s_a.grip.0 * 4.0, 0.0, 2.0);
                next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0);
                next.hand_r.position = Vec3::new(-s_a.grip.0 * 4.0, 0.0, 2.0);
                next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0);

                next.head.position += Vec3::new(0.0, -2.0, 0.0) * move1;
                next.head.orientation.rotate_x(PI / 8.0 * move1);
                next.chest.position += Vec3::new(0.0, -2.0, 0.0) * move1;
                next.chest.orientation.rotate_x(PI / 16.0 * move1);
                next.control_l.position += Vec3::new(0.0, 0.0, 4.0) * move1;
                next.control_l.orientation.rotate_y(PI / 5.0 * move1);
                next.main.position += Vec3::new(2.0, 0.0, 9.0) * move1;
                next.main.orientation.rotate_y(PI / 5.0 * move1);
                next.control_r.position += Vec3::new(0.0, 0.0, 4.0) * move1;
                next.control_r.orientation.rotate_y(-PI / 5.0 * move1);

                next.control_l.position += Vec3::new(0.0, 0.0, 4.0 * slow);
                next.main.position += Vec3::new(0.0, 0.0, 4.0 * slow);
                next.control_r.position += Vec3::new(0.0, 0.0, 4.0 * slow);
            },
            _ => {
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);
                next.second.position = Vec3::new(0.0, 0.0, 0.0);
                next.second.orientation = Quaternion::rotation_x(0.0);

                next.hand_l.position = Vec3::new(0.0, 0.0, s_a.grip.0);
                next.hand_r.position = Vec3::new(0.0, 0.0, s_a.grip.0);

                next.hand_l.orientation = Quaternion::rotation_x(0.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.0);

                next.control_l.position = Vec3::new(-1.0, -2.0, 12.0);
                next.control_r.position = Vec3::new(1.0, -2.0, -2.0);

                next.head.orientation =
                    Quaternion::rotation_x(move1 * 0.3) * Quaternion::rotation_z(tension2 * 0.2);
                next.control_l.orientation = Quaternion::rotation_x(PI / 2.0 + move1 * 0.2)
                    * Quaternion::rotation_y(move1 * -1.0);
                next.control_r.orientation = Quaternion::rotation_x(PI / 2.0 + 0.2 + move1 * -0.2)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);

                next.control.orientation = Quaternion::rotation_x(-1.0 + move1 * 1.0)
                    * Quaternion::rotation_y(-1.8 + move1 * 1.2 + tension * 0.09)
                    * Quaternion::rotation_z(move1 * 1.5);
                next.chest.orientation = Quaternion::rotation_z(tension2 * -0.08);
                next.pants.orientation = Quaternion::rotation_z(tension2 * 0.08);

                next.control.orientation.rotate_z(move1 * -2.0);
                next.control.orientation.rotate_x(move1 * 0.5);
                next.control.position += Vec3::new(move1 * -4.0, move1 * 10.0, move1 * 5.0);

                next.chest.orientation.rotate_x(move1 * 0.4);
                next.pants.position += Vec3::new(0.0, -2.0, 0.0) * move1;
                next.pants.orientation.rotate_x(-move1 * 0.4);
            },
        }

        next
    }
}
