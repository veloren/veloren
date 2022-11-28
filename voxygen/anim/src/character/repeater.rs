use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::{AbilityInfo, StageSection},
    util::Dir,
};
use core::f32::consts::PI;

pub struct RepeaterAnimation;

impl Animation for RepeaterAnimation {
    type Dependency<'a> = (
        Option<AbilityInfo>,
        (Option<Hands>, Option<Hands>),
        Vec3<f32>,
        Dir,
        Vec3<f32>,
        f32,
        Option<StageSection>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_repeater\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_repeater")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_info, hands, orientation,look_dir, velocity, _global_time, stage_section): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();
        let speed = Vec2::<f32>::from(velocity).magnitude();
        let ori_angle = orientation.y.atan2(orientation.x);
        let lookdir_angle = look_dir.y.atan2(look_dir.x);
        let swivel = lookdir_angle - ori_angle;
        let (move1base, move2base, move3base, move4) = match stage_section {
            Some(StageSection::Movement) => (anim_time, 0.0, 0.0, 0.0),
            Some(StageSection::Buildup) => (1.0, anim_time, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, 1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.1, 1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - move4;
        let move1 = move1base * pullback;
        let move2 = move2base * pullback;
        let move3 = move3base * pullback;
        // end spin stuff

        if let Some(ToolKind::Bow) = ability_info.and_then(|a| a.tool) {
            next.hand_l.position = Vec3::new(s_a.bhl.0, s_a.bhl.1, s_a.bhl.2);
            next.hand_l.orientation = Quaternion::rotation_x(s_a.bhl.3);
            next.hand_r.position = Vec3::new(s_a.bhr.0, s_a.bhr.1, s_a.bhr.2);
            next.hand_r.orientation = Quaternion::rotation_x(s_a.bhr.3);
            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_y(0.0) * Quaternion::rotation_z(0.0);

            next.hold.position = Vec3::new(0.0, -1.0 + move3 * 2.0, -5.2);
            next.hold.orientation = Quaternion::rotation_x(-PI / 2.0) * Quaternion::rotation_z(0.0);
            next.hold.scale = Vec3::one() * (1.0);

            next.chest.orientation = Quaternion::rotation_z(swivel * 0.8);
            next.torso.orientation = Quaternion::rotation_z(swivel * 0.2);

            if speed < 0.5 {
                next.foot_l.position = Vec3::new(
                    -s_a.foot.0 + move1 * -0.75,
                    s_a.foot.1 + move1 * 4.0,
                    s_a.foot.2,
                );
                next.foot_l.orientation =
                    Quaternion::rotation_x(move1 * 0.2 + move2 * -0.1 + move3 * -0.2)
                        * Quaternion::rotation_z(move3 * 0.1);

                next.foot_r.position = Vec3::new(s_a.foot.0 + move1 * 0.75, s_a.foot.1, s_a.foot.2);
                next.foot_r.orientation =
                    Quaternion::rotation_x(move1 * 0.06 + move2 * -0.2 + move3 * -0.5)
                        * Quaternion::rotation_z(move1 * -0.6 + move3 * 0.8);
                next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);
                next.chest.orientation = Quaternion::rotation_x(0.0);
            } else {
            };
            next.shorts.position = Vec3::new(0.0, s_a.shorts.0 + move1 * 2.0, s_a.shorts.1);
            next.shorts.orientation = Quaternion::rotation_x(move1 * 0.2 + move3 * 0.2);
            next.belt.position = Vec3::new(0.0, s_a.belt.0 + move1 * 1.0, s_a.belt.1);
            next.belt.orientation = Quaternion::rotation_x(move1 * 0.1 + move3 * 0.1);
            next.control.position = Vec3::new(
                s_a.bc.0 + move1 * 5.0,
                s_a.bc.1 + move1 * 3.0,
                s_a.bc.2 + move1 * 5.0,
            );
            next.control.orientation = Quaternion::rotation_x(s_a.bc.3 + move1 * 0.4)
                * Quaternion::rotation_y(s_a.bc.4 + move1 * 0.8)
                * Quaternion::rotation_z(s_a.bc.5);
            next.head.orientation = Quaternion::rotation_x(move1 * 0.15)
                * Quaternion::rotation_y(move1 * 0.15 + move2 * 0.05);
            next.torso.orientation = Quaternion::rotation_x(move1 * 0.25 + move3 * -0.2);

            next.hand_l.position = Vec3::new(0.0, -2.5 + move3 * -6.0, 0.0);
            next.hand_l.orientation = Quaternion::rotation_x(1.5)
                * Quaternion::rotation_y(-0.0)
                * Quaternion::rotation_z(-0.3);
        }

        if let (None, Some(Hands::Two)) = hands {
            next.second = next.main;
        }

        next
    }
}
