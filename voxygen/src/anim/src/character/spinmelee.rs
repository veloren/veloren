use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::StageSection,
};
use std::f32::consts::PI;

pub struct SpinMeleeAnimation;

impl Animation for SpinMeleeAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        Vec3<f32>,
        f64,
        Option<StageSection>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_spinmelee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_spinmelee")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, velocity, _global_time, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let lab = 1.0;
        let (movement1, movement2, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time as f32, 0.0, 0.0),
            Some(StageSection::Swing) => (1.0, anim_time as f32, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time as f32),
            _ => (0.0, 0.0, 0.0),
        };
        let speed = Vec2::<f32>::from(velocity).magnitude();
        let mut next = (*skeleton).clone();
        //torso movement
        let xshift = if velocity.z.abs() < 0.1 {
            ((anim_time as f32 - 1.1) * lab as f32 * 3.0).sin()
        } else {
            0.0
        };
        let yshift = if velocity.z.abs() < 0.1 {
            ((anim_time as f32 - 1.1) * lab as f32 * 3.0 + PI / 2.0).sin()
        } else {
            0.0
        };

        let spin = if anim_time < 1.1 && velocity.z.abs() < 0.1 {
            0.5 * ((anim_time as f32).powf(2.0))
        } else {
            lab as f32 * anim_time as f32 * 0.9
        };

        //feet
        let slowersmooth = (anim_time as f32 * lab as f32 * 4.0).sin();
        let quick = (anim_time as f32 * lab as f32 * 8.0).sin();

        match active_tool_kind {
            Some(ToolKind::Sword(_)) => {
                next.hand_l.position = Vec3::new(-0.75, -1.0, 2.5);
                next.hand_l.orientation =
                    Quaternion::rotation_x(1.47) * Quaternion::rotation_y(-0.2);
                next.hand_r.position = Vec3::new(0.75, -1.5, -0.5);
                next.hand_r.orientation =
                    Quaternion::rotation_x(1.47) * Quaternion::rotation_y(0.3);
                next.main.position = Vec3::new(0.0, 0.0, 2.0);
                next.main.orientation = Quaternion::rotation_x(-0.1)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);

                next.control.position = Vec3::new(-7.0, 7.0, 2.0);
                next.control.orientation = Quaternion::rotation_x(-PI / 2.0 + movement3 * PI / 2.0)
                    * Quaternion::rotation_z(-PI / 2.0 + movement3 * PI / 2.0);
                next.torso.orientation = Quaternion::rotation_z(movement2 * PI * 2.0);

                next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);
                next.chest.orientation = Quaternion::rotation_y(0.3 + movement3 * -0.3);
                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
                next.head.orientation = Quaternion::rotation_x(-0.15 + movement3 * 0.15);
                next.belt.orientation = Quaternion::rotation_x(0.1);
                next.shorts.orientation = Quaternion::rotation_x(0.2);
            },
            Some(ToolKind::Axe(_)) => {
                next.hand_l.position = Vec3::new(-0.5, 0.0, 4.0);
                next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
                    * Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_y(PI);
                next.hand_r.position = Vec3::new(0.5, 0.0, -2.5);
                next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
                    * Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_y(0.0);
                next.main.position = Vec3::new(-0.0, -2.0, -1.0);
                next.main.orientation = Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(0.0);

                next.control.position = Vec3::new(0.0, 16.0, 3.0);
                next.control.orientation = Quaternion::rotation_x(-1.4)
                    * Quaternion::rotation_y(0.0)
                    * Quaternion::rotation_z(1.4);

                next.head.orientation = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(-0.15)
                    * Quaternion::rotation_y(0.08);
                next.chest.position = Vec3::new(0.0, s_a.chest.0 - 3.0, s_a.chest.1 - 2.0);
                next.chest.orientation = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(-0.1)
                    * Quaternion::rotation_y(0.3);

                next.belt.position = Vec3::new(0.0, 1.0, -1.0);
                next.belt.orientation = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(0.4)
                    * Quaternion::rotation_y(0.0);
                next.shorts.position = Vec3::new(0.0, 3.0, -2.5);
                next.shorts.orientation = Quaternion::rotation_z(0.0)
                    * Quaternion::rotation_x(0.7)
                    * Quaternion::rotation_y(0.0);
                next.torso.position = Vec3::new(
                    -xshift * (anim_time as f32).min(0.6),
                    -yshift * (anim_time as f32).min(0.6),
                    0.0,
                ) * s_a.scaler;
                next.torso.orientation = Quaternion::rotation_z(spin * -16.0)
                    * Quaternion::rotation_x(0.0)
                    * Quaternion::rotation_y(0.0);
                if velocity.z.abs() > 0.1 {
                    next.foot_l.position = Vec3::new(-s_a.foot.0, 8.0, s_a.foot.2 + 2.0);
                    next.foot_l.orientation =
                        Quaternion::rotation_x(1.0) * Quaternion::rotation_z(0.0);

                    next.foot_r.position = Vec3::new(s_a.foot.0, 8.0, s_a.foot.2 + 2.0);
                    next.foot_r.orientation = Quaternion::rotation_x(1.0);
                } else if speed < 0.5 {
                    next.foot_l.position = Vec3::new(-s_a.foot.0, 2.0 + quick * -6.0, s_a.foot.2);
                    next.foot_l.orientation = Quaternion::rotation_x(0.5 + slowersmooth * 0.2)
                        * Quaternion::rotation_z(0.0);

                    next.foot_r.position = Vec3::new(s_a.foot.0, 4.0, s_a.foot.2);
                    next.foot_r.orientation = Quaternion::rotation_x(0.5 - slowersmooth * 0.2)
                        * Quaternion::rotation_y(-0.4);
                } else {
                    next.foot_l.position = Vec3::new(-s_a.foot.0, 2.0 + quick * -6.0, s_a.foot.2);
                    next.foot_l.orientation = Quaternion::rotation_x(0.5 + slowersmooth * 0.2);

                    next.foot_r.position = Vec3::new(s_a.foot.0, 2.0 + quick * 6.0, s_a.foot.2);
                    next.foot_r.orientation = Quaternion::rotation_x(0.5 - slowersmooth * 0.2);
                };
            },
            _ => {},
        }

        next.lantern.orientation = Quaternion::rotation_z(0.0)
            * Quaternion::rotation_x(0.7)
            * Quaternion::rotation_y(-0.8);

        next.second.scale = match (
            active_tool_kind.map(|tk| tk.hands()),
            second_tool_kind.map(|tk| tk.hands()),
        ) {
            (Some(Hands::OneHand), Some(Hands::OneHand)) => Vec3::one(),
            (_, _) => Vec3::zero(),
        };

        next
    }
}
