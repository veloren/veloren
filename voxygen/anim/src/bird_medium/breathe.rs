use super::{
    super::{Animation, vek::*},
    BirdMediumSkeleton, SkeletonAttr,
};
use common::{states::utils::StageSection, util::Dir};

pub struct BreatheAnimation;

type BreatheAnimationDependency = (
    Vec3<f32>,
    f32,
    Vec3<f32>,
    Vec3<f32>,
    Option<StageSection>,
    f32,
    Dir,
    bool,
);

impl Animation for BreatheAnimation {
    type Dependency<'a> = BreatheAnimationDependency;
    type Skeleton = BirdMediumSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"bird_medium_breathe\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "bird_medium_breathe"))]
    fn update_skeleton_inner<'a>(
        skeleton: &Self::Skeleton,
        (velocity,global_time, _orientation, _last_ori, stage_section, timer, look_dir, on_ground): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (movement1base, movement2base, movement3, twitch) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powf(0.25), 0.0, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time.min(1.0).powf(0.1), 0.0, anim_time),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time, 1.0),
            _ => (0.0, 0.0, 0.0, 0.0),
        };

        let pullback = 1.0 - movement3;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let twitch2 = mirror * (twitch * 20.0).sin() * pullback;

        let movement1abs = movement1base * pullback;
        let movement2abs = movement2base * pullback;

        let wave_slow_cos = (anim_time * 4.5).cos();

        next.chest.position = Vec3::new(
            0.0,
            s_a.chest.0,
            s_a.chest.1 + wave_slow_cos * 0.06 + twitch2 * 0.1,
        );

        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
        next.head.orientation =
            Quaternion::rotation_x(movement1abs * 0.5 - movement2abs * 0.5 + look_dir.z * 0.4);

        if on_ground {
            next.chest.orientation =
                Quaternion::rotation_x(movement1abs * 0.2 - movement2abs * 0.5 + twitch2 * 0.03);
            next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + movement2abs * -3.0);
            next.wing_in_l.position = Vec3::new(-s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);
            next.wing_in_r.position = Vec3::new(s_a.wing_in.0, s_a.wing_in.1, s_a.wing_in.2);

            next.wing_in_l.orientation =
                Quaternion::rotation_y(
                    -1.0 + movement1abs * 0.8 - movement2abs * 0.4 + twitch2 * 0.03,
                ) * Quaternion::rotation_z(0.2 - movement1abs * 0.8 + movement2abs * 0.4);
            next.wing_in_r.orientation =
                Quaternion::rotation_y(
                    1.0 - movement1abs * 0.8 + movement2abs * 0.4 + twitch2 * -0.03,
                ) * Quaternion::rotation_z(-0.2 + movement1abs * 0.8 - movement2abs * 0.4);

            next.wing_out_l.position = Vec3::new(-s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
            next.wing_out_r.position = Vec3::new(s_a.wing_out.0, s_a.wing_out.1, s_a.wing_out.2);
            next.wing_out_l.orientation =
                Quaternion::rotation_y(-0.2) * Quaternion::rotation_z(0.2);
            next.wing_out_r.orientation =
                Quaternion::rotation_y(0.2) * Quaternion::rotation_z(-0.2);

            next.tail.position = Vec3::new(0.0, s_a.tail.0, s_a.tail.1);
            next.tail.orientation =
                Quaternion::rotation_x(-movement1abs * 0.1 + movement2abs * 0.1 + twitch2 * 0.1);
        } else {
            next.head.orientation = Quaternion::rotation_x(
                movement1abs * 0.5
                    + movement2abs * (-0.5 + velocity.xy().magnitude() * 0.2).min(0.0)
                    + look_dir.z * 0.4,
            );
        }

        next
    }
}
