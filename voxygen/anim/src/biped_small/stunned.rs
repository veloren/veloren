use super::{
    super::{vek::*, Animation},
    BipedSmallSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};
use core::f32::consts::PI;

pub struct StunnedAnimation;

type StunnedAnimationDependency = (
    Option<ToolKind>,
    Vec3<f32>,
    Vec3<f32>,
    Vec3<f32>,
    f32,
    Vec3<f32>,
    f32,
    bool,
    Option<StageSection>,
    f32,
);

impl Animation for StunnedAnimation {
    type Dependency<'a> = StunnedAnimationDependency;
    type Skeleton = BipedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_small_stunned\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_small_stunned")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            active_tool_kind,
            velocity,
            _orientation,
            _last_ori,
            global_time,
            _avg_vel,
            _acc_vel,
            wield_status,
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
        if wield_status {
            let (movement1base, movement2) = match stage_section {
                Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0),
                Some(StageSection::Recover) => (1.0, anim_time.powf(4.0)),
                _ => (0.0, 0.0),
            };
            let pullback = 1.0 - movement2;
            let subtract = global_time - timer;
            let check = subtract - subtract.trunc();
            let mirror = (check - 0.5).signum();
            let movement1 = movement1base * pullback * mirror;
            let movement1abs = movement1base * pullback;
            next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
            next.head.orientation =
                Quaternion::rotation_x(movement1 * 0.2) * Quaternion::rotation_z(movement1 * -0.3);
            next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + movement1abs - 3.0);
            next.chest.orientation = Quaternion::rotation_z(movement1 * 1.2);

            next.pants.position = Vec3::new(0.0, s_a.pants.0, s_a.pants.1);

            next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
            next.tail.orientation = Quaternion::rotation_x(0.0) * Quaternion::rotation_z(0.0);

            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_x(0.0);

            next.hand_l.position = Vec3::new(s_a.grip.0 * 4.0, 0.0, s_a.grip.2);
            next.hand_r.position = Vec3::new(-s_a.grip.0 * 4.0, 0.0, s_a.grip.2);

            next.hand_l.orientation = Quaternion::rotation_x(0.0);
            next.hand_r.orientation = Quaternion::rotation_x(0.0);

            //IMPORTANT: avoid touching any value attached to grip. grip uses the size of
            // the hand bones to correct any irrgularities beween species. Changing
            // coefficients to grip will have different effects across species

            match active_tool_kind {
                Some(ToolKind::Spear) => {
                    next.control_l.position = Vec3::new(1.0 - s_a.grip.0 * 2.0, 2.0, -2.0);
                    next.control_r.position = Vec3::new(-1.0 + s_a.grip.0 * 2.0, 2.0, 2.0);

                    next.control.position = Vec3::new(
                        -3.0,
                        s_a.grip.2,
                        -s_a.grip.2 / 2.5 + s_a.grip.0 * -2.0 + speednorm * 2.0,
                    );

                    next.control_l.orientation =
                        Quaternion::rotation_x(PI / 1.5) * Quaternion::rotation_y(-0.3);
                    next.control_r.orientation =
                        Quaternion::rotation_x(PI / 1.5 + s_a.grip.0 * 0.2)
                            * Quaternion::rotation_y(0.5 + s_a.grip.0 * 0.2);

                    next.control.orientation = Quaternion::rotation_x(-1.35 + 0.5 * speednorm);
                },

                Some(ToolKind::Bow) => {
                    next.control_l.position = Vec3::new(1.0 - s_a.grip.0 * 2.0, 0.0, 0.0);
                    next.control_r.position = Vec3::new(-1.0 + s_a.grip.0 * 2.0, 6.0, -2.0);

                    next.control.position = Vec3::new(
                        -1.0,
                        2.0 + s_a.grip.2,
                        3.0 + -s_a.grip.2 / 2.5 + s_a.grip.0 * -2.0 + speednorm * 2.0,
                    );

                    next.control_l.orientation =
                        Quaternion::rotation_x(PI / 2.0) * Quaternion::rotation_y(-0.3);
                    next.control_r.orientation =
                        Quaternion::rotation_x(PI / 2.0 + s_a.grip.0 * 0.2)
                            * Quaternion::rotation_y(0.5 + s_a.grip.0 * 0.2);

                    next.control.orientation = Quaternion::rotation_x(-0.3 + 0.5 * speednorm)
                        * Quaternion::rotation_y(0.5 * speednorm);
                },
                Some(ToolKind::Staff) => {
                    next.control_l.position = Vec3::new(2.0 - s_a.grip.0 * 2.0, 1.0, 3.0);
                    next.control_r.position =
                        Vec3::new(7.0 + s_a.grip.0 * 2.0, -4.0, 3.0 + speednorm * -3.0);

                    next.control.position = Vec3::new(
                        -5.0,
                        -1.0 + s_a.grip.2,
                        -2.0 + -s_a.grip.2 / 2.5 + s_a.grip.0 * -2.0 + speednorm * 2.0,
                    );

                    next.control_l.orientation = Quaternion::rotation_x(PI / 2.0)
                        * Quaternion::rotation_y(-0.3)
                        * Quaternion::rotation_z(-0.3);
                    next.control_r.orientation =
                        Quaternion::rotation_x(PI / 2.0 + s_a.grip.0 * 0.2)
                            * Quaternion::rotation_y(-0.4 + s_a.grip.0 * 0.2)
                            * Quaternion::rotation_z(-0.0);

                    next.control.orientation = Quaternion::rotation_x(-0.3 + 0.2 * speednorm)
                        * Quaternion::rotation_y(-0.2 * speednorm)
                        * Quaternion::rotation_z(0.5);
                },
                Some(ToolKind::Natural) => {
                    next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2);
                    next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2);

                    next.hand_l.orientation =
                        Quaternion::rotation_x(1.7) * Quaternion::rotation_y(-0.3);
                    next.hand_r.orientation =
                        Quaternion::rotation_x(1.7) * Quaternion::rotation_y(0.3);
                },
                _ => {},
            }
        } else {
            next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

            next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);
            next.pants.position = Vec3::new(0.0, s_a.pants.0, s_a.pants.1);
            next.main.position = Vec3::new(2.0, -3.0, -3.0);
            next.main.orientation = Quaternion::rotation_y(-0.5) * Quaternion::rotation_z(PI / 2.0);

            next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
            next.hand_l.position = Vec3::new(-s_a.hand.0, s_a.hand.1, s_a.hand.2);
            next.hand_r.position = Vec3::new(s_a.hand.0, s_a.hand.1, s_a.hand.2);
            next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2);
            next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2);
        }

        next
    }
}
