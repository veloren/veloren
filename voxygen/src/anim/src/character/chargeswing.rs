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

        let short = (((5.0)
            / (1.5 + 3.5 * ((anim_time as f32 * lab as f32 * 8.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 8.0).sin());
        // end spin stuff

        let (movement1, movement2, movement3, tension) = match stage_section {
            Some(StageSection::Charge) => (
                (anim_time as f32).min(1.0),
                0.0,
                0.0,
                (anim_time as f32 * 18.0 * lab as f32).sin(),
            ),
            Some(StageSection::Swing) => (1.0, anim_time as f32, 0.0, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, (anim_time as f32).powf(4.0), 0.0),
            _ => (0.0, 0.0, 0.0, 0.0),
        };
        if let Some(ToolKind::Hammer(_)) = active_tool_kind {
            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_x(0.0);
            next.hand_l.position = Vec3::new(
                s_a.hhl.0,
                s_a.hhl.1,
                s_a.hhl.2 + (movement2 * -8.0) * (1.0 - movement3),
            );
            next.hand_l.orientation =
                Quaternion::rotation_x(s_a.hhl.3) * Quaternion::rotation_y(s_a.hhl.4);
            next.hand_r.position = Vec3::new(s_a.hhr.0, s_a.hhr.1, s_a.hhr.2);
            next.hand_r.orientation =
                Quaternion::rotation_x(s_a.hhr.3) * Quaternion::rotation_y(s_a.hhr.4);

            next.control.position = Vec3::new(
                s_a.hc.0 + (movement1 * -2.0 + movement2 * -3.0) * (1.0 - movement3),
                s_a.hc.1 + (movement1 * 2.0 + movement2 * 3.0) * (1.0 - movement3),
                s_a.hc.2 + (movement1 * 2.0 + movement2 * 4.0) * (1.0 - movement3),
            );
            next.control.orientation = Quaternion::rotation_x(s_a.hc.3+(movement2*4.0)*(1.0-movement3))
                    * Quaternion::rotation_y(s_a.hc.4+(tension*0.08+movement1 * 0.7+movement2*-3.5)*(1.0-movement3))//+fire * 0.1
                    * Quaternion::rotation_z(s_a.hc.5+(movement1 * 0.2+movement2*-0.5)*(1.0-movement3));
            next.chest.orientation = Quaternion::rotation_z(
                short * 0.04 + (movement1 * 2.0 + movement2 * -2.5) * (1.0 - movement3),
            );
            next.belt.orientation =
                Quaternion::rotation_z(short * 0.08 + (movement1 * -1.0) * (1.0 - movement3));
            next.shorts.orientation =
                Quaternion::rotation_z(short * 0.15 + (movement1 * -1.0) * (1.0 - movement3));
            next.head.position = Vec3::new(
                0.0 + (movement1 * -1.0 + movement2 * 2.0) * (1.0 - movement3),
                s_a.head.0 + (movement1 * 1.0) * (1.0 - movement3),
                s_a.head.1,
            );
            next.head.orientation =
                Quaternion::rotation_z((movement1 * -1.5 + movement2 * 2.2) * (1.0 - movement3));
            next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);
        }
        next
    }
}
