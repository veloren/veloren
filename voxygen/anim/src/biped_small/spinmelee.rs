use super::{
    super::{vek::*, Animation},
    BipedSmallSkeleton, SkeletonAttr,
};
use common::{comp::item::ToolKind, states::utils::StageSection};

pub struct SpinMeleeAnimation;

type SpinMeleeAnimationDependency = (
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

impl Animation for SpinMeleeAnimation {
    type Dependency<'a> = SpinMeleeAnimationDependency;
    type Skeleton = BipedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_small_spinmelee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_small_spinmelee")]

    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (
            active_tool_kind,
            _velocity,
            _orientation,
            _last_ori,
            _global_time,
            _avg_vel,
            _acc_vel,
            stage_section,
            _timer,
        ): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();
        let anim_time = anim_time.min(1.0);
        let (move1base, tension, move2base, move3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.sqrt(), (anim_time * 10.0).sin(), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, 1.0, anim_time.powf(0.25), 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time.powi(4)),
            _ => (0.0, 0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - move3;
        let tension = tension * pullback;
        let move1abs = move1base * pullback;
        let move2abs = move2base * pullback;
        next.hand_l.position = Vec3::new(s_a.grip.0 * 4.0, 0.0, s_a.grip.2);
        next.hand_r.position = Vec3::new(-s_a.grip.0 * 4.0, 0.0, s_a.grip.2);
        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_x(0.0);
        next.hand_l.orientation = Quaternion::rotation_x(0.0);
        next.hand_r.orientation = Quaternion::rotation_x(0.0);
        match active_tool_kind {
            Some(ToolKind::Spear) => {
                next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);
                next.chest.orientation = Quaternion::rotation_x(move1abs * -0.2 + move2abs * 0.3)
                    * Quaternion::rotation_z(move1abs * 0.5 + move2abs * -0.6);
            },
            _ => {
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
        }
        next
    }
}
