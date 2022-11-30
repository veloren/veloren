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

pub struct ShootAnimation;

type ShootAnimationDependency = (
    Option<AbilityInfo>,
    (Option<Hands>, Option<Hands>),
    f32,
    Vec3<f32>,
    Vec3<f32>,
    Dir,
    f32,
    Option<StageSection>,
);
impl Animation for ShootAnimation {
    type Dependency<'a> = ShootAnimationDependency;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_shoot\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_shoot")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            ability_info,
            hands,
            velocity,
            orientation,
            last_ori,
            look_dir,
            _global_time,
            stage_section,
        ): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let _speed = Vec2::<f32>::from(velocity).magnitude();

        let mut next = (*skeleton).clone();

        let lab: f32 = 1.0;
        let ori: Vec2<f32> = Vec2::from(orientation);
        let last_ori = Vec2::from(last_ori);
        let tilt = if vek::Vec2::new(ori, last_ori)
            .map(|o| o.magnitude_squared())
            .map(|m| m > 0.001 && m.is_finite())
            .reduce_and()
            && ori.angle_between(last_ori).is_finite()
        {
            ori.angle_between(last_ori).min(0.2)
                * last_ori.determine_side(Vec2::zero(), ori).signum()
        } else {
            0.0
        } * 1.3;
        let ori_angle = orientation.y.atan2(orientation.x);
        let lookdir_angle = look_dir.y.atan2(look_dir.x);
        let swivel = lookdir_angle - ori_angle;
        match ability_info.and_then(|a| a.tool) {
            Some(ToolKind::Staff) | Some(ToolKind::Sceptre) => {
                let (move1, move2, move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time.powf(0.25), 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                };
                let xmove = (move1 * 6.0 * lab + PI).sin();
                let ymove = (move1 * 6.0 * lab + PI * (0.5)).sin();
                next.hand_l.position = Vec3::new(s_a.sthl.0, s_a.sthl.1, s_a.sthl.2);
                next.hand_l.orientation = Quaternion::rotation_x(s_a.sthl.3);

                next.hand_r.position = Vec3::new(s_a.sthr.0, s_a.sthr.1, s_a.sthr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.sthr.3) * Quaternion::rotation_y(s_a.sthr.4);

                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_y(0.0);

                next.control.position = Vec3::new(
                    s_a.stc.0 + (xmove * 3.0 + move1 * -4.0) * (1.0 - move3),
                    s_a.stc.1 + (2.0 + ymove * 3.0 + move2 * 3.0) * (1.0 - move3),
                    s_a.stc.2 + look_dir.z * 4.0,
                );
                next.control.orientation =
                    Quaternion::rotation_x(look_dir.z + s_a.stc.3 + (move2 * 0.6) * (1.0 - move3))
                        * Quaternion::rotation_y(s_a.stc.4 + (move1 * 0.5 + move2 * -0.5))
                        * Quaternion::rotation_z(
                            s_a.stc.5 - (0.2 + move1 * -0.5 + move2 * 0.8) * (1.0 - move3),
                        );

                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
                next.head.orientation = Quaternion::rotation_x(look_dir.z * 0.7)
                    * Quaternion::rotation_z(
                        tilt * -2.5 + (move1 * -0.2 + move2 * -0.4) * (1.0 - move3),
                    );
                next.chest.orientation = Quaternion::rotation_z(swivel * 0.8);
                next.torso.orientation = Quaternion::rotation_z(swivel * 0.2);
            },
            Some(ToolKind::Bow) => {
                let (_move1, move2, _move3) = match stage_section {
                    Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time.powi(4)),
                    _ => (0.0, 0.0, 0.0),
                };
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);
                next.hand_l.position = Vec3::new(
                    s_a.bhl.0 + move2 * -8.0,
                    s_a.bhl.1 + move2 * -10.0,
                    s_a.bhl.2,
                );
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.bhl.3) * Quaternion::rotation_y(move2 * 0.7);
                next.hand_r.position = Vec3::new(s_a.bhr.0, s_a.bhr.1, s_a.bhr.2);
                next.hand_r.orientation = Quaternion::rotation_x(s_a.bhr.3);

                next.hold.position = Vec3::new(0.0, -1.0 + move2 * 2.0, -5.2 + move2 * 7.0);
                next.hold.orientation = Quaternion::rotation_x(-PI / 2.0);
                next.hold.scale = Vec3::one() * 1.0 * (1.0 - move2);

                next.control.position = Vec3::new(
                    s_a.bc.0 + 11.0 + move2 * 2.0,
                    s_a.bc.1 + 2.0 + (look_dir.z * -5.0).min(-2.0) + move2 * -1.0,
                    s_a.bc.2 + 8.0 + (look_dir.z * 15.0).max(-8.0),
                );
                next.control.orientation = Quaternion::rotation_x(look_dir.z)
                    * Quaternion::rotation_y(-look_dir.z + s_a.bc.4 - 1.25)
                    * Quaternion::rotation_z(s_a.bc.5 - 0.2 + move2 * -0.1);

                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

                next.head.orientation =
                    Quaternion::rotation_x(look_dir.z * 0.7) * Quaternion::rotation_z(tilt * -0.0);
                next.chest.orientation = Quaternion::rotation_z(swivel * 0.8 + 0.8 + move2 * 0.5);
                next.torso.orientation = Quaternion::rotation_z(swivel * 0.2);

                next.shoulder_l.orientation = Quaternion::rotation_x(move2 * 0.5);
            },
            _ => {},
        }

        next.back.orientation = Quaternion::rotation_x(-0.3);

        next.lantern.orientation = Quaternion::rotation_x(0.0) * Quaternion::rotation_y(0.0);

        if let (None, Some(Hands::Two)) = hands {
            next.second = next.main;
        }

        next
    }
}
