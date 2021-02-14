use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};
pub struct ChargeswingAnimation;

impl Animation for ChargeswingAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        Vec3<f32>,
        f64,
        Option<StageSection>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_chargeswing\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_chargeswing")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, _second_tool_kind, _velocity, _global_time, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let lab = 1.0;

        let short = (((5.0) / (1.5 + 3.5 * ((anim_time as f32 * lab as f32 * 8.0).sin()).powi(2)))
            .sqrt())
            * ((anim_time as f32 * lab as f32 * 8.0).sin());
        // end spin stuff

        let (move1base, move2base, move3, tension, test) = match stage_section {
            Some(StageSection::Charge) => (
                (anim_time as f32).min(1.0),
                0.0,
                0.0,
                (anim_time as f32 * 18.0 * lab as f32).sin(),
                0.0,
            ),
            Some(StageSection::Swing) => (
                1.0,
                (anim_time as f32).powf(0.25),
                0.0,
                0.0,
                (anim_time as f32).powi(4),
            ),
            Some(StageSection::Recover) => (1.0, 1.0, (anim_time as f32).powi(4), 0.0, 1.0),
            _ => (0.0, 0.0, 0.0, 0.0, 0.0),
        };
        let move1 = move1base * (1.0 - move3);
        let slowrise = test * (1.0 - move3);

        let move2 = move2base * (1.0 - move3);
        if let Some(ToolKind::Hammer) = active_tool_kind {
            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_x(0.0);
            next.hand_l.position = Vec3::new(s_a.hhl.0, s_a.hhl.1, s_a.hhl.2 + (move2 * -8.0));
            next.hand_l.orientation =
                Quaternion::rotation_x(s_a.hhl.3) * Quaternion::rotation_y(s_a.hhl.4);
            next.hand_r.position = Vec3::new(s_a.hhr.0, s_a.hhr.1, s_a.hhr.2);
            next.hand_r.orientation =
                Quaternion::rotation_x(s_a.hhr.3) * Quaternion::rotation_y(s_a.hhr.4);

            next.control.position = Vec3::new(
                s_a.hc.0 + (move1 * -2.0 + move2 * -8.0),
                s_a.hc.1 + (move1 * 2.0 + move2 * 6.0),
                s_a.hc.2 + (move1 * -2.0 + slowrise * 8.0),
            );
            next.control.orientation = Quaternion::rotation_x(s_a.hc.3 + (move2 * 0.0))
                * Quaternion::rotation_y(
                    s_a.hc.4 + (tension * 0.08 + move1 * 0.7 + move2 * -1.0 + slowrise * 2.0),
                )
                * Quaternion::rotation_z(s_a.hc.5 + (move1 * 0.2 + move2 * -1.0));
            next.chest.orientation =
                Quaternion::rotation_z(short * 0.04 + (move1 * 2.0 + move2 * -3.5));
            next.belt.orientation = Quaternion::rotation_z(short * 0.08 + (move1 * -1.0));
            next.shorts.orientation = Quaternion::rotation_z(short * 0.15 + (move1 * -1.0));
            next.head.position = Vec3::new(
                0.0 + (move1 * -1.0 + move2 * 2.0),
                s_a.head.0 + (move1 * 1.0),
                s_a.head.1,
            );
            next.head.orientation = Quaternion::rotation_z(move1 * -1.5 + move2 * 3.2);
            next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);
        }
        next
    }
}
