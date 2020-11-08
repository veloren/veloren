use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};

pub struct ShootAnimation;

impl Animation for ShootAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        f32,
        Vec3<f32>,
        Vec3<f32>,
        f64,
        Option<StageSection>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_shoot\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_shoot")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            active_tool_kind,
            _second_tool_kind,
            velocity,
            orientation,
            last_ori,
            _global_time,
            stage_section,
        ): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let speed = Vec2::<f32>::from(velocity).magnitude();

        let mut next = (*skeleton).clone();

        let lab = 1.0;
        let foot = (((5.0)
            / (0.2 + 4.8 * ((anim_time as f32 * lab as f32 * 8.0).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 8.0).sin());
        let foote = (((5.0)
            / (0.5 + 4.5 * ((anim_time as f32 * lab as f32 * 8.0 + 1.57).sin()).powf(2.0 as f32)))
        .sqrt())
            * ((anim_time as f32 * lab as f32 * 8.0).sin());

        let exp = ((anim_time as f32).powf(0.3 as f32)).min(1.2);

        match active_tool_kind {
            Some(ToolKind::Staff) | Some(ToolKind::Sceptre) => {
                next.hand_l.position = Vec3::new(s_a.sthl.0, s_a.sthl.1, s_a.sthl.2);
                next.hand_l.orientation = Quaternion::rotation_x(s_a.sthl.3);

                next.hand_r.position = Vec3::new(s_a.sthr.0, s_a.sthr.1, s_a.sthr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.sthr.3) * Quaternion::rotation_y(s_a.sthr.4);

                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_y(0.0);

                next.control.position =
                    Vec3::new(s_a.stc.0, s_a.stc.1 + exp * 5.0, s_a.stc.2 - exp * 5.0);
                next.control.orientation = Quaternion::rotation_x(s_a.stc.3 + exp * 0.4)
                    * Quaternion::rotation_y(s_a.stc.4)
                    * Quaternion::rotation_z(s_a.stc.5 + exp * 1.5);
            },
            Some(ToolKind::Bow(_)) => {
                let (movement1, movement2, movement3) = match stage_section {
                    Some(StageSection::Buildup) => ((anim_time as f32).powf(0.25), 0.0, 0.0),
                    Some(StageSection::Swing) => (1.0, anim_time as f32, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, (anim_time as f32).powf(4.0)),
                    _ => (0.0, 0.0, 0.0),
                };
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);
                next.hand_l.position = Vec3::new(
                    s_a.bhl.0 + movement2 * -5.0,
                    s_a.bhl.1 + movement2 * -10.0,
                    s_a.bhl.2 + movement2 * -7.0,
                );
                next.hand_l.orientation = Quaternion::rotation_x(s_a.bhl.3 + movement2 * 0.5);
                next.hand_r.position = Vec3::new(s_a.bhr.0, s_a.bhr.1, s_a.bhr.2);
                next.hand_r.orientation = Quaternion::rotation_x(s_a.bhr.3);

                next.hold.position = Vec3::new(0.0, -1.0, -5.2 + movement2 * 5.0);
                next.hold.orientation = Quaternion::rotation_x(-1.57);
                next.hold.scale = Vec3::one() * 1.0 * (1.0 - movement2);

                next.control.position = Vec3::new(s_a.bc.0 + 11.0, s_a.bc.1 + 2.0, s_a.bc.2 + 8.0);
                next.control.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(s_a.bc.4 - 1.25)
                    * Quaternion::rotation_z(s_a.bc.5 - 0.1);
                next.chest.orientation = Quaternion::rotation_z(0.8);
                next.head.position = Vec3::new(0.0 + movement1 * -2.0, s_a.head.0, s_a.head.1);

                next.head.orientation = Quaternion::rotation_z(movement1 * -0.8);
                if speed < 0.5 {
                    next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1 + 5.0, s_a.foot.2);
                    next.foot_l.orientation = Quaternion::rotation_x(0.5);

                    next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1 - 3.0, s_a.foot.2);
                    next.foot_r.orientation =
                        Quaternion::rotation_x(-0.5) * Quaternion::rotation_z(-0.3);
                } else {
                };
            },
            _ => {},
        }
        if velocity > 0.5 {
        } else {
        }
        next.back.orientation = Quaternion::rotation_x(-0.3);

        next.lantern.orientation =
            Quaternion::rotation_x(exp * -0.7 + 0.4) * Quaternion::rotation_y(exp * 0.4);

        next
    }
}
