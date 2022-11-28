use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::Hands,
    states::utils::{AbilityInfo, StageSection},
};

pub struct Input {
    pub attack: bool,
}
pub struct ShockwaveAnimation;

impl Animation for ShockwaveAnimation {
    type Dependency<'a> = (
        Option<AbilityInfo>,
        (Option<Hands>, Option<Hands>),
        f32,
        f32,
        Option<StageSection>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_shockwave\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_shockwave")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_ability_info, hands, _global_time, velocity, stage_section): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (move1, move2, move3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0),
        };

        if matches!(
            stage_section,
            Some(StageSection::Action | StageSection::Recover)
        ) {
            next.main_weapon_trail = true;
            next.off_weapon_trail = true;
        }
        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);

        next.hand_l.position = Vec3::new(s_a.sthl.0, s_a.sthl.1, s_a.sthl.2);
        next.hand_l.orientation =
            Quaternion::rotation_x(s_a.sthl.3) * Quaternion::rotation_y(s_a.sthl.4);
        next.hand_r.position = Vec3::new(s_a.sthr.0, s_a.sthr.1, s_a.sthr.2);
        next.hand_r.orientation =
            Quaternion::rotation_x(s_a.sthr.3) * Quaternion::rotation_y(s_a.sthr.4);
        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_x(0.0);

        next.control.position = Vec3::new(s_a.stc.0, s_a.stc.1, s_a.stc.2);
        next.control.orientation =
            Quaternion::rotation_x(s_a.stc.3) * Quaternion::rotation_y(s_a.stc.4);

        let twist = move1 * 0.8;

        next.control.position = Vec3::new(
            s_a.stc.0 + (move1 * 5.0) * (1.0 - move3),
            s_a.stc.1 + (move1 * 5.0) * (1.0 - move3),
            s_a.stc.2 + (move1 * 10.0 + move2 * -10.0) * (1.0 - move3),
        );
        next.control.orientation =
            Quaternion::rotation_x(s_a.stc.3 + (move1 * 0.8) * (1.0 - move3))
                * Quaternion::rotation_y(
                    s_a.stc.4 + (move1 * -0.15 + move2 * -0.15) * (1.0 - move3),
                )
                * Quaternion::rotation_z((move1 * 0.8 + move2 * -0.8) * (1.0 - move3));

        next.head.orientation = Quaternion::rotation_x((move1 * 0.4) * (1.0 - move3))
            * Quaternion::rotation_z((twist * 0.2 + move2 * -0.8) * (1.0 - move3));

        next.chest.position = Vec3::new(
            0.0,
            s_a.chest.0,
            s_a.chest.1 + (move1 * 2.0 + move2 * -4.0) * (1.0 - move3),
        );
        next.chest.orientation = Quaternion::rotation_x((move2 * -0.8) * (1.0 - move3))
            * Quaternion::rotation_z(twist * -0.2 + move2 * -0.1 + (1.0 - move3));

        next.belt.orientation = Quaternion::rotation_x((move2 * 0.2) * (1.0 - move3))
            * Quaternion::rotation_z((twist * 0.6 + move2 * -0.48) * (1.0 - move3));

        next.shorts.orientation = Quaternion::rotation_x((move2 * 0.3) * (1.0 - move3))
            * Quaternion::rotation_z((twist + move2 * -0.8) * (1.0 - move3));

        if velocity < 0.5 {
            next.foot_l.position = Vec3::new(
                -s_a.foot.0,
                s_a.foot.1 + move1 * -7.0 + move2 * 7.0,
                s_a.foot.2,
            );
            next.foot_l.orientation = Quaternion::rotation_x(move1 * -0.8 + move2 * 0.8)
                * Quaternion::rotation_z(move1 * 0.3 + move2 * -0.3);

            next.foot_r.position = Vec3::new(
                s_a.foot.0,
                s_a.foot.1 + move1 * 5.0 + move2 * -5.0,
                s_a.foot.2,
            );
            next.foot_r.orientation = Quaternion::rotation_y(move1 * -0.3 + move2 * 0.3)
                * Quaternion::rotation_z(move1 * 0.4 + move2 * -0.4);
        }

        if let (None, Some(Hands::Two)) = hands {
            next.second = next.main;
        }

        next
    }
}
