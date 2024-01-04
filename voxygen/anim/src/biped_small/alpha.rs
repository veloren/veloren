use super::{
    super::{vek::*, Animation},
    biped_small_alpha_axe, biped_small_alpha_dagger, biped_small_alpha_spear,
    init_biped_small_alpha, BipedSmallSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};
use std::f32::consts::PI;

pub struct AlphaAnimation;

type AlphaAnimationDependency<'a> = (
    Option<&'a str>,
    Option<ToolKind>,
    Vec3<f32>,
    Vec3<f32>,
    Vec3<f32>,
    f32,
    Vec3<f32>,
    f32,
    Option<StageSection>,
    f32,
);

impl Animation for AlphaAnimation {
    type Dependency<'a> = AlphaAnimationDependency<'a>;
    type Skeleton = BipedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_small_alpha\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_small_alpha")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            ability_id,
            active_tool_kind,
            velocity,
            _orientation,
            _last_ori,
            global_time,
            _avg_vel,
            _acc_vel,
            stage_section,
            timer,
        ): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();
        let speednorm = speed / 9.4;
        let speednormcancel = 1.0 - speednorm;

        let fast = (anim_time * 10.0).sin();
        let fastalt = (anim_time * 10.0 + PI / 2.0).sin();

        let anim_time = anim_time.min(1.0);
        let (move1base, move2base, move3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.sqrt(), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time.powi(4), 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - move3;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let move1 = move1base * pullback * mirror;
        let move2 = move2base * pullback * mirror;
        let move1abs = move1base * pullback;
        let move2abs = move2base * pullback;

        init_biped_small_alpha(&mut next, s_a);

        match active_tool_kind {
            Some(ToolKind::Spear) => {
                biped_small_alpha_spear(
                    &mut next,
                    s_a,
                    move1abs,
                    move2abs,
                    fast,
                    fastalt,
                    speednormcancel,
                );
            },
            Some(ToolKind::Axe | ToolKind::Hammer | ToolKind::Pick | ToolKind::Shovel) => {
                biped_small_alpha_axe(&mut next, s_a, move1abs, move2abs);
            },
            Some(ToolKind::Dagger) => {
                biped_small_alpha_dagger(&mut next, s_a, move1abs, move2abs);
            },
            Some(ToolKind::Staff) => match ability_id {
                Some("common.abilities.custom.dwarves.flamekeeper.flamecrush") => {
                    next.control_l.position = Vec3::new(2.0 - s_a.grip.0 * 2.0, 1.0, 3.0);
                    next.control_r.position = Vec3::new(12.0 + s_a.grip.0 * 2.0, -4.0, 3.0);

                    next.control.position = Vec3::new(
                        -9.0 + move1 * 5.0 + move2 * -5.0,
                        2.0 + s_a.grip.2,
                        -2.0 + -s_a.grip.2 / 2.5
                            + s_a.grip.0 * -2.0
                            + move1abs * 6.0
                            + move2abs * -3.0,
                    );

                    next.control_l.orientation = Quaternion::rotation_x(PI / 2.0)
                        * Quaternion::rotation_y(-0.3)
                        * Quaternion::rotation_z(-0.3);
                    next.control_r.orientation =
                        Quaternion::rotation_x(PI / 2.0 + s_a.grip.0 * 0.2)
                            * Quaternion::rotation_y(-0.4 + s_a.grip.0 * 0.2)
                            * Quaternion::rotation_z(-0.0);

                    next.control.orientation =
                        Quaternion::rotation_x(-0.3 + move1abs * 1.0 + move2abs * -2.0)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(0.5);
                    next.chest.orientation =
                        Quaternion::rotation_x(move1abs * 0.5 + move2abs * -1.0)
                            * Quaternion::rotation_z(move1 * 1.2 + move2 * -1.8);
                    next.head.orientation = Quaternion::rotation_z(move1 * -0.8 + move2 * 0.8);
                },
                _ => {
                    next.control_l.position = Vec3::new(2.0 - s_a.grip.0 * 2.0, 1.0, 3.0);
                    next.control_r.position = Vec3::new(7.0 + s_a.grip.0 * 2.0, -4.0, 3.0);

                    next.control.position = Vec3::new(
                        -5.0 + move1 * 5.0 + move2 * -5.0,
                        -1.0 + s_a.grip.2,
                        -2.0 + -s_a.grip.2 / 2.5
                            + s_a.grip.0 * -2.0
                            + move1abs * 6.0
                            + move2abs * -3.0,
                    );

                    next.control_l.orientation = Quaternion::rotation_x(PI / 2.0)
                        * Quaternion::rotation_y(-0.3)
                        * Quaternion::rotation_z(-0.3);
                    next.control_r.orientation =
                        Quaternion::rotation_x(PI / 2.0 + s_a.grip.0 * 0.2)
                            * Quaternion::rotation_y(-0.4 + s_a.grip.0 * 0.2)
                            * Quaternion::rotation_z(-0.0);

                    next.control.orientation =
                        Quaternion::rotation_x(-0.3 + move1abs * 1.0 + move2abs * -2.0)
                            * Quaternion::rotation_y(0.0)
                            * Quaternion::rotation_z(0.5);
                    next.chest.orientation =
                        Quaternion::rotation_x(move1abs * 0.5 + move2abs * -1.0)
                            * Quaternion::rotation_z(move1 * 1.2 + move2 * -1.8);
                    next.head.orientation = Quaternion::rotation_z(move1 * -0.8 + move2 * 0.8);
                },
            },
            Some(ToolKind::Natural) => {
                let tension = match stage_section {
                    Some(StageSection::Buildup) => (anim_time * 10.0).sin(),
                    Some(StageSection::Action) => 1.0,
                    Some(StageSection::Recover) => 1.0,
                    _ => 0.0,
                };
                next.chest.orientation =
                    Quaternion::rotation_x(move1abs * 1.0 + move2abs * -1.5 + tension * 0.2);
                next.pants.orientation = Quaternion::rotation_x(move1abs * -0.5 + move2abs * 0.5);
                next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(1.2 + move1abs * 1.5 + move2abs * -1.0)
                        * Quaternion::rotation_y(tension * 0.5);
                next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(1.2 + move1abs * 1.5 + move2abs * -1.0)
                        * Quaternion::rotation_y(tension * -0.5);
            },
            _ => {
                next.chest.orientation = Quaternion::rotation_x(move2abs * -1.0)
                    * Quaternion::rotation_z(move1 * 1.2 + move2 * -1.8);
                next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2);
                next.hand_l.orientation = Quaternion::rotation_x(1.2);
                next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2);
                next.hand_r.orientation = Quaternion::rotation_x(1.2);
            },
        }
        next
    }
}
