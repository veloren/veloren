use super::{
    super::{vek::*, Animation},
    biped_small_wield_bow, biped_small_wield_spear, biped_small_wield_sword, BipedSmallSkeleton,
    SkeletonAttr,
};
use common::comp::item::tool::{AbilitySpec, ToolKind};
use std::f32::consts::PI;

pub struct WieldAnimation;

type WieldAnimationDependency<'a> = (
    (Option<ToolKind>, Option<&'a AbilitySpec>),
    Vec3<f32>,
    Vec3<f32>,
    Vec3<f32>,
    f32,
    Vec3<f32>,
    f32,
);

impl Animation for WieldAnimation {
    type Dependency<'a> = WieldAnimationDependency<'a>;
    type Skeleton = BipedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_small_wield\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_small_wield")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            (active_tool_kind, active_tool_spec),
            velocity,
            _orientation,
            _last_ori,
            _global_time,
            _avg_vel,
            acc_vel,
        ): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();

        let fastacc = (acc_vel * 2.0).sin();
        let fast = (anim_time * 10.0).sin();
        let fastalt = (anim_time * 10.0 + PI / 2.0).sin();
        let slow = (anim_time * 2.0).sin();

        let speednorm = speed / 9.4;
        let speednormcancel = 1.0 - speednorm;

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1 + fast * -0.1 * speednormcancel);
        next.head.orientation = Quaternion::rotation_x(0.45 * speednorm)
            * Quaternion::rotation_y(fast * 0.07 * speednormcancel);
        next.chest.position = Vec3::new(
            0.0,
            s_a.chest.0,
            s_a.chest.1 + fastalt * 0.4 * speednormcancel + speednormcancel * -0.5,
        );

        next.pants.position = Vec3::new(0.0, s_a.pants.0, s_a.pants.1);

        next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
        next.tail.orientation = Quaternion::rotation_x(0.05 * fastalt * speednormcancel)
            * Quaternion::rotation_z(fast * 0.15 * speednormcancel);

        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_z(0.0);

        next.hand_l.position = Vec3::new(s_a.grip.0 * 4.0, 0.0, s_a.grip.2);
        next.hand_r.position = Vec3::new(-s_a.grip.0 * 4.0, 0.0, s_a.grip.2);

        next.hand_l.orientation = Quaternion::rotation_x(0.0);
        next.hand_r.orientation = Quaternion::rotation_x(0.0);

        //IMPORTANT: avoid touching any value attached to grip. grip uses the size of
        // the hand bones to correct any irrgularities beween species. Changing
        // coefficients to grip will have different effects across species

        match active_tool_kind {
            Some(ToolKind::Spear) => {
                biped_small_wield_spear(&mut next, s_a, anim_time, speed, fastacc);
            },
            Some(ToolKind::Blowgun) => {
                next.control_l.position = Vec3::new(1.0 - s_a.grip.0 * 2.0, 0.0, 3.0);
                next.control_r.position = Vec3::new(-1.0 + s_a.grip.0 * 2.0, 0.0, 4.0);

                next.control.position = Vec3::new(
                    0.0,
                    s_a.grip.2,
                    4.0 - s_a.grip.2 / 2.5
                        + s_a.grip.0 * -2.0
                        + fastacc * 0.5
                        + fastalt * 0.1 * speednormcancel
                        + speednorm * 4.0,
                );

                next.control_l.orientation =
                    Quaternion::rotation_x(3.8 + slow * 0.1) * Quaternion::rotation_y(-0.3);
                next.control_r.orientation =
                    Quaternion::rotation_x(3.5 + slow * 0.1 + s_a.grip.0 * 0.2)
                        * Quaternion::rotation_y(0.5 + slow * 0.0 + s_a.grip.0 * 0.2);

                next.control.orientation = Quaternion::rotation_x(-2.2 + 0.5 * speednorm);
            },
            Some(ToolKind::Bow) => {
                biped_small_wield_bow(&mut next, s_a, anim_time, speed, fastacc);
            },
            Some(ToolKind::Staff) => {
                next.control_l.position = Vec3::new(2.0 - s_a.grip.0 * 2.0, 1.0, 3.0);
                next.control_r.position =
                    Vec3::new(7.0 + s_a.grip.0 * 2.0, -4.0, 3.0 + speednorm * -3.0);

                next.control.position = Vec3::new(
                    -5.0,
                    -1.0 + s_a.grip.2,
                    -2.0 + -s_a.grip.2 / 2.5
                        + s_a.grip.0 * -2.0
                        + fastacc * 1.5
                        + fastalt * 0.5 * speednormcancel
                        + speednorm * 2.0,
                );

                next.control_l.orientation = Quaternion::rotation_x(PI / 2.0 + slow * 0.1)
                    * Quaternion::rotation_y(-0.3)
                    * Quaternion::rotation_z(-0.3);
                next.control_r.orientation =
                    Quaternion::rotation_x(PI / 2.0 + slow * 0.1 + s_a.grip.0 * 0.2)
                        * Quaternion::rotation_y(-0.4 + slow * 0.0 + s_a.grip.0 * 0.2)
                        * Quaternion::rotation_z(-0.0);

                next.control.orientation = Quaternion::rotation_x(-0.3 + 0.2 * speednorm)
                    * Quaternion::rotation_y(-0.2 * speednorm)
                    * Quaternion::rotation_z(0.5);
            },
            Some(ToolKind::Axe | ToolKind::Hammer | ToolKind::Pick) => {
                next.control_l.position = Vec3::new(2.0 - s_a.grip.0 * 2.0, 1.0, 3.0);
                next.control_r.position =
                    Vec3::new(9.0 + s_a.grip.0 * 2.0, -1.0, -2.0 + speednorm * -3.0);

                next.control.position = Vec3::new(
                    -5.0,
                    -1.0 + s_a.grip.2,
                    -1.0 + -s_a.grip.2 / 2.5 + s_a.grip.0 * -2.0 + speednorm * 2.0,
                );

                next.control_l.orientation = Quaternion::rotation_x(PI / 2.0 + slow * 0.1)
                    * Quaternion::rotation_y(-0.0)
                    * Quaternion::rotation_z(-0.0);
                next.control_r.orientation =
                    Quaternion::rotation_x(0.5 + slow * 0.1 + s_a.grip.0 * 0.2)
                        * Quaternion::rotation_y(0.2 + slow * 0.0 + s_a.grip.0 * 0.2)
                        * Quaternion::rotation_z(-0.0);

                next.control.orientation = Quaternion::rotation_x(-0.3 + 0.2 * speednorm)
                    * Quaternion::rotation_y(-0.2 * speednorm)
                    * Quaternion::rotation_z(-0.3);
            },
            Some(ToolKind::Dagger | ToolKind::Sword) => {
                biped_small_wield_sword(&mut next, s_a, speednorm, slow);
            },
            Some(ToolKind::Natural) => {
                if let Some(AbilitySpec::Custom(spec)) = active_tool_spec {
                    match spec.as_str() {
                        "ShamanicSpirit" => {
                            next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2);
                            next.hand_l.orientation = Quaternion::rotation_x(1.2);
                            next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2);
                            next.hand_r.orientation = Quaternion::rotation_x(1.2);
                            next.main.position = Vec3::new(0.0, 12.0, 5.0);
                        },
                        _ => {
                            next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2);
                            next.hand_l.orientation = Quaternion::rotation_x(1.2);
                            next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2);
                            next.hand_r.orientation = Quaternion::rotation_x(1.2);
                        },
                    }
                }
            },
            _ => {
                next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2);
                next.hand_l.orientation = Quaternion::rotation_x(1.2);
                next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2);
                next.hand_r.orientation = Quaternion::rotation_x(1.2);
            },
        }

        next
    }
}
