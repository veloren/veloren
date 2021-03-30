use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::{AbilityInfo, StageSection},
};
use std::f32::consts::PI;

pub struct SpinMeleeAnimation;

impl Animation for SpinMeleeAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        (Option<Hands>, Option<Hands>),
        Vec3<f32>,
        f32,
        Option<StageSection>,
        Option<AbilityInfo>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_spinmelee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_spinmelee")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            active_tool_kind,
            _second_tool_kind,
            hands,
            _velocity,
            _global_time,
            stage_section,
            ability_info,
        ): Self::Dependency,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let (move1, move2, move3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
            Some(StageSection::Swing) => (1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time.powf(4.0)),
            _ => (0.0, 0.0, 0.0),
        };
        let mut next = (*skeleton).clone();

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

                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2 + move1 * 2.0);
                next.control.orientation =
                    Quaternion::rotation_x(s_a.sc.3 + move1 * -PI / 2.5 + move3 * PI / 2.0)
                        * Quaternion::rotation_z(s_a.sc.5 + move1 * -PI / 2.0 + move3 * PI / 2.0);
                next.torso.orientation = Quaternion::rotation_z(move2 * PI * 2.0);

                next.chest.position =
                    Vec3::new(0.0, s_a.chest.0 + move1 * -2.0, s_a.chest.1 + move1 * -3.0);
                next.chest.orientation = Quaternion::rotation_x(move1 * -0.3)
                    * Quaternion::rotation_y(move1 * 0.15 + move3 * -0.15);
                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
                next.head.orientation = Quaternion::rotation_x(move1 * 0.2 + move3 * 0.15)
                    * Quaternion::rotation_z(move2 * 0.8 + move3 * -0.6);
                next.belt.orientation = Quaternion::rotation_x(0.1);
                next.shorts.orientation = Quaternion::rotation_x(0.2);
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

                next.control.position =
                    Vec3::new(s_a.ac.0 + move1 * 8.0, s_a.ac.1, s_a.ac.2 + move1 * -4.0);
                next.control.orientation =
                    Quaternion::rotation_x(s_a.ac.3 + move1 * -0.8 * (1.0 - move3))
                        * Quaternion::rotation_y(s_a.ac.4 + move1 * -PI * (1.0 - move3))
                        * Quaternion::rotation_z(s_a.ac.5 + move1 * 1.2 * (1.0 - move3));

                next.head.orientation = Quaternion::rotation_x(move1 * -0.2 * (1.0 - move3))
                    * Quaternion::rotation_z(move1 * 0.4 * (1.0 - move3));
                next.head.position = Vec3::new(0.0, s_a.head.0 + move1 * 2.0, s_a.head.1);

                next.chest.position =
                    Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + move1 * -1.0 * (1.0 - move3));
                next.chest.orientation = Quaternion::rotation_x(move1 * 0.3 * (1.0 - move3))
                    * Quaternion::rotation_y(move1 * 0.3 * (1.0 - move3));

                next.belt.position = Vec3::new(
                    0.0,
                    1.0 + s_a.belt.0,
                    s_a.belt.1 + move1 * 0.5 * (1.0 - move3),
                );
                next.belt.orientation = Quaternion::rotation_x(0.15);
                next.shorts.position = Vec3::new(
                    0.0,
                    1.0 + s_a.shorts.0 + move1 * 1.0 * (1.0 - move3),
                    s_a.shorts.1 + move1 * 1.0 * (1.0 - move3),
                );
                next.shorts.orientation =
                    Quaternion::rotation_x(0.15 + 0.15 * move1 * (1.0 - move3));

                next.torso.orientation =
                    Quaternion::rotation_z(move1 * 1.0 * (1.0 - move3) + move2 * -2.0 * PI);

                next.foot_l.position = Vec3::new(
                    -s_a.foot.0,
                    s_a.foot.1 + move1 * 7.0 * (1.0 - move3),
                    s_a.foot.2,
                );
                next.foot_l.orientation = Quaternion::rotation_x(move1 * 0.8 * (1.0 - move3));

                next.foot_r.position = Vec3::new(
                    s_a.foot.0,
                    s_a.foot.1 + move1 * -3.0 * (1.0 - move3),
                    s_a.foot.2,
                );
                next.foot_r.orientation = Quaternion::rotation_x(move1 * -0.5 * (1.0 - move3));
            },

            _ => {},
        }

        next
    }
}
