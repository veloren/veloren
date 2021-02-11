use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};
use std::f32::consts::PI;

pub struct SpinAnimation;

impl Animation for SpinAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        Vec3<f32>,
        f64,
        Option<StageSection>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_spin\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_spin")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, _second_tool_kind, _velocity, _global_time, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (movement1base, movement2base, movement3) = match stage_section {
            Some(StageSection::Buildup) => ((anim_time as f32).powf(0.25), 0.0, 0.0),
            Some(StageSection::Swing) => (1.0, anim_time as f32, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, (anim_time as f32).powi(4)),
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - movement3;
        let movement1 = movement1base * pullback;
        let movement2 = movement2base * pullback;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

        match active_tool_kind {
            Some(ToolKind::Sword) => {
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position = Vec3::new(s_a.shr.0, s_a.shr.1, s_a.shr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.shr.3) * Quaternion::rotation_y(s_a.shr.4);

                next.control.position = Vec3::new(
                    s_a.sc.0 + movement1 * 2.0 + movement2 * -12.0 + movement3 * -7.0,
                    s_a.sc.1 + 8.0 + movement1 * 0.6 + movement2 * -9.0 + movement3 * -10.0,
                    s_a.sc.2 + 1.0 + movement1 * 0.6 + movement2 * 1.5 + movement3 * -4.0,
                );
                next.control.orientation =
                    Quaternion::rotation_x(-0.5 + s_a.sc.3 + movement1 * -1.2)
                        * Quaternion::rotation_y(
                            s_a.sc.4 - 0.6 + movement1 * 0.0 + movement2 * -1.3,
                        )
                        * Quaternion::rotation_z(s_a.sc.5 + 0.1 + movement1 * -1.57);
                next.head.position = Vec3::new(
                    0.0 + 2.0 + movement2 * -2.0,
                    2.0 + movement2 * -2.0 + s_a.head.0,
                    s_a.head.1,
                );

                next.chest.orientation = Quaternion::rotation_x(movement2 * 0.15)
                    * Quaternion::rotation_y(movement1 * -0.1 + movement2 * 0.3 + movement3 * -0.1)
                    * Quaternion::rotation_z(
                        -1.0 + movement1 * -0.6 + movement2 * 1.5 + movement3 * 0.5,
                    );

                next.belt.orientation = Quaternion::rotation_x(movement1 * 0.1)
                    * Quaternion::rotation_z(movement2.sin() * 0.5);

                next.shorts.orientation = Quaternion::rotation_x(movement1 * 0.1)
                    * Quaternion::rotation_z(movement2.sin() * 1.5);

                next.head.orientation = Quaternion::rotation_y(movement1 * 0.1 + movement2 * -0.1)
                    * Quaternion::rotation_z(1.07 + movement1 * 0.4 + movement2 * -1.1);
            },

            Some(ToolKind::Axe) => {
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);
                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                let (movement1, movement2, movement3) = match stage_section {
                    Some(StageSection::Buildup) => ((anim_time as f32).powf(0.25), 0.0, 0.0),
                    Some(StageSection::Swing) => (1.0, anim_time as f32, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, (anim_time as f32).powi(4)),
                    _ => (0.0, 0.0, 0.0),
                };

                next.control.position = Vec3::new(
                    s_a.ac.0 + (-3.0 + movement1 * 0.0 + movement2 * -2.0),
                    s_a.ac.1 + (-3.5 + movement1 * -4.6 + movement2 * 5.0),
                    s_a.ac.2 + (-11.0 + movement1 * 10.0 + movement2 * -4.0),
                );
                next.control.orientation =
                    Quaternion::rotation_x(s_a.ac.3 + (-2.6 + movement1 * 0.0 + movement2 * -0.6))
                        * Quaternion::rotation_y(
                            s_a.ac.4 + (0.2 + movement1 * -0.5 + movement2 * 0.0),
                        )
                        * Quaternion::rotation_z(
                            s_a.ac.5 + (-0.5 + movement1 * -3.0 + movement2 * 0.5),
                        );
                next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);

                next.chest.orientation = Quaternion::rotation_x(0.4 + movement2 * -0.5)
                    * Quaternion::rotation_y(movement1 * -0.1 + movement2 * 0.0)
                    * Quaternion::rotation_z(0.5 + movement1 * -0.6 + movement2 * 0.6);

                next.belt.orientation = Quaternion::rotation_x(movement1 * -0.2 + movement2 * 0.2);

                next.shorts.orientation =
                    Quaternion::rotation_x(movement1 * -0.2 + movement2 * 0.2);

                next.head.orientation = Quaternion::rotation_y(movement1 * 0.0 + movement3 * -0.0)
                    * Quaternion::rotation_z(1.0 + movement1 * -0.5 + movement2 * 0.0);
                next.torso.position = Vec3::new(
                    0.0,
                    0.0,
                    -1.0 + 1.0 * (movement1 * 0.5 * PI).sin()
                        + 1.0 * (movement2 * 0.5 * PI + 0.5 * PI).sin(),
                );
                next.torso.orientation =
                    Quaternion::rotation_z(movement1.powi(2) * -6.0 + movement2 * -1.7);

                next.foot_l.position = Vec3::new(
                    -s_a.foot.0 + (movement1 * -1.0 + movement2 * -3.0),
                    s_a.foot.1,
                    s_a.foot.2 + (movement2 * 6.0),
                );
                next.foot_l.orientation = Quaternion::rotation_x(movement1 * 0.2 + movement2 * 0.5)
                    * Quaternion::rotation_y(movement2 * 0.5);

                next.foot_r.position = Vec3::new(
                    s_a.foot.0,
                    s_a.foot.1 + (movement1 * -2.0 + movement2 * -3.0),
                    s_a.foot.2,
                );
                next.foot_r.orientation =
                    Quaternion::rotation_x(movement1 * -0.5 + movement2 * -0.5);
            },
            _ => {},
        }
        next
    }
}
