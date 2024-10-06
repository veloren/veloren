use std::f32::consts::PI;

use common::{
    comp::tool::ToolKind,
    states::utils::{HandInfo, StageSection},
};

use crate::{
    Animation,
    character::{CharacterSkeleton, SkeletonAttr, twist_back, twist_forward},
    vek::*,
};

pub struct ThrowAnimation;

type ThrowDependency = (Option<StageSection>, ToolKind, HandInfo);

impl Animation for ThrowAnimation {
    type Dependency<'a> = ThrowDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_throw";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_throw")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (stage_section, tool_kind, hand_info): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        _s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (move1base, chargebase, move2base, move3base) = match stage_section {
            Some(StageSection::Buildup) => (anim_time, 0.0, 0.0, 0.0),
            Some(StageSection::Charge) => (1.0, anim_time, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, 1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - move3base;
        let move1 = move1base * pullback;
        let charge = chargebase.min(1.0);
        let tension = 0.5 * (chargebase * 16.0).sin();
        let move2 = move2base * pullback;

        match tool_kind {
            ToolKind::Throwable => {
                next.main.position += Vec3::new(0.0, 0.0, 4.0);
                next.main.orientation.rotate_x(-PI / 2.0);
                next.main.scale = Vec3::one();
                next.second.position += Vec3::new(0.0, 0.0, 4.0);
                next.second.orientation.rotate_x(-PI / 2.0);
                next.second.scale = Vec3::one();

                match hand_info {
                    HandInfo::MainHand => {
                        twist_back(&mut next, move1, 0.5, 0.5, 0.2, 0.4);
                        next.control_l.position += Vec3::new(-6.0, 0.0, 0.0) * move1;
                        next.control_l.orientation.rotate_x(PI / 2.0 * move1);
                        next.control_r.position += Vec3::new(6.0, 0.0, 0.0) * move1;
                        next.foot_l.position += Vec3::new(0.0, -2.0, 0.0) * move1;
                        next.foot_l.orientation.rotate_z(PI / 4.0 * move1);
                        next.foot_r.position += Vec3::new(0.0, 2.0, 0.0) * move1;
                        next.foot_r.orientation.rotate_z(PI / 8.0 * move1);

                        twist_back(&mut next, charge, 1.0, 1.0, 0.2, 0.4);
                        next.control_l.position += Vec3::new(tension, 6.0, 4.0) * charge;
                        next.control_l.orientation.rotate_y(PI / 4.0 * charge);
                        next.control_r.position += Vec3::new(6.0, 0.0, -2.0) * charge;
                        next.control_r.orientation.rotate_y(-PI / 2.0 * charge);

                        twist_forward(&mut next, move2, 3.0, 2.8, 0.2, 0.4);
                        next.control_l.position += Vec3::new(6.0, 14.0, -4.0) * move2;
                        next.foot_l.position += Vec3::new(0.0, 4.0, 0.0) * move2;
                        next.foot_l.orientation.rotate_z(-PI / 2.0 * move2);
                        next.foot_r.position += Vec3::new(0.0, -4.0, 0.0) * move2;
                        next.foot_r.orientation.rotate_z(-PI / 4.0 * move2);
                    },
                    HandInfo::OffHand => {
                        twist_forward(&mut next, move1, 0.5, 0.5, 0.2, 0.4);
                        next.control_l.position += Vec3::new(-6.0, 0.0, 0.0) * move1;
                        next.control_r.position += Vec3::new(6.0, 0.0, 0.0) * move1;
                        next.control_r.orientation.rotate_x(PI / 2.0 * move1);
                        next.foot_l.position += Vec3::new(0.0, 2.0, 0.0) * move1;
                        next.foot_l.orientation.rotate_z(-PI / 8.0 * move1);
                        next.foot_r.position += Vec3::new(0.0, -2.0, 0.0) * move1;
                        next.foot_r.orientation.rotate_z(-PI / 4.0 * move1);

                        twist_forward(&mut next, charge, 1.0, 1.0, 0.2, 0.4);
                        next.control_l.position += Vec3::new(-6.0, 0.0, -2.0) * charge;
                        next.control_l.orientation.rotate_y(PI / 2.0 * charge);
                        next.control_r.position += Vec3::new(tension, 6.0, 4.0) * charge;
                        next.control_r.orientation.rotate_y(-PI / 4.0 * charge);

                        twist_back(&mut next, move2, 3.0, 2.8, 0.2, 0.4);
                        next.control_r.position += Vec3::new(6.0, 14.0, -4.0) * move2;
                        next.foot_l.position += Vec3::new(0.0, -4.0, 0.0) * move2;
                        next.foot_l.orientation.rotate_z(PI / 4.0 * move2);
                        next.foot_r.position += Vec3::new(0.0, 4.0, 0.0) * move2;
                        next.foot_r.orientation.rotate_z(PI / 2.0 * move2);
                    },
                    HandInfo::TwoHanded => {},
                }
            },
            _ => {},
        }

        next
    }
}
