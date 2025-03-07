use std::f32::consts::PI;

use super::{
    super::{Animation, vek::*},
    ArthropodSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct MultiAction;

pub struct MultiActionDependency<'a> {
    pub ability_id: Option<&'a str>,
    pub stage_section: Option<StageSection>,
    pub current_action: u32,
    pub max_actions: Option<u32>,
    pub global_time: f32,
    pub timer: f32,
}

impl Animation for MultiAction {
    type Dependency<'a> = MultiActionDependency<'a>;
    type Skeleton = ArthropodSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"arthropod_multi\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "arthropod_multi"))]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        d: Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let multi_action_pullback = 1.0
            - if matches!(d.stage_section, Some(StageSection::Recover)) {
                anim_time
            } else {
                0.0
            };

        for action in 0..=d.current_action {
            let (move1base, move2base, move3base) = if action == d.current_action {
                match d.stage_section {
                    Some(StageSection::Buildup) => (anim_time, 0.0, 0.0),
                    Some(StageSection::Action) => (1.0, anim_time, 0.0),
                    Some(StageSection::Recover) => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                }
            } else {
                (1.0, 1.0, 1.0)
            };
            let _move1 = move1base * multi_action_pullback;
            let _move2 = move2base * multi_action_pullback;

            match d.ability_id {
                Some(
                    "common.abilities.custom.arthropods.tarantula.singlestrike"
                    | "common.abilities.custom.arthropods.blackwidow.singlestrike"
                    | "common.abilities.custom.arthropods.antlion.singlestrike"
                    | "common.abilities.custom.arthropods.hornbeetle.singlestrike"
                    | "common.abilities.custom.arthropods.weevil.singlestrike"
                    | "common.abilities.custom.arthropods.crawler.singlestrike",
                ) => {
                    let pullback = 1.0 - move3base;
                    let subtract = d.global_time - d.timer;
                    let check = subtract - subtract.trunc();
                    let mirror = (check - 0.5).signum();
                    let movement1abs = move1base.powi(2) * pullback;
                    let movement2abs = move2base.powi(4) * pullback;
                    let movement3abs = move3base * pullback;

                    if s_a.snapper {
                        next.chest.scale = Vec3::one() * s_a.scaler;

                        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
                        next.head.orientation = Quaternion::rotation_x(
                            movement1abs * -0.2 + movement2abs * 0.4 + movement3abs * 0.8,
                        ) * Quaternion::rotation_y(
                            movement1abs * -0.1 + movement2abs * 0.2 + movement3abs * 0.2,
                        );

                        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);

                        next.mandible_l.position =
                            Vec3::new(-s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);
                        next.mandible_r.position =
                            Vec3::new(s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);
                        next.mandible_l.orientation = Quaternion::rotation_z(
                            movement1abs * 0.7 + movement2abs * -1.0 + movement3abs * 0.8,
                        );
                        next.mandible_r.orientation = Quaternion::rotation_z(
                            movement1abs * -0.7 + movement2abs * 1.0 + movement3abs * -0.8,
                        );

                        next.wing_fl.position =
                            Vec3::new(-s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);
                        next.wing_fr.position = Vec3::new(s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);
                        next.wing_fl.orientation =
                            Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * 0.5)
                                * Quaternion::rotation_y(movement1abs * 0.2 + movement2abs * -0.2)
                                * Quaternion::rotation_z(movement1abs * -0.2 + movement2abs * 0.2);
                        next.wing_fr.orientation =
                            Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * 0.5)
                                * Quaternion::rotation_y(movement1abs * -0.2 + movement2abs * 0.2)
                                * Quaternion::rotation_z(movement1abs * 0.2 + movement2abs * -0.2);

                        next.wing_bl.position =
                            Vec3::new(-s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);
                        next.wing_br.position = Vec3::new(s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);
                        next.wing_bl.orientation =
                            Quaternion::rotation_x(movement1abs * -0.3 + movement2abs * 0.3)
                                * Quaternion::rotation_y(movement1abs * 0.2 + movement2abs * -0.2);
                        next.wing_br.orientation =
                            Quaternion::rotation_x(movement1abs * -0.3 + movement2abs * 0.3)
                                * Quaternion::rotation_y(movement1abs * -0.2 + movement2abs * 0.2);

                        next.leg_fl.position = Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
                        next.leg_fr.position = Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);

                        next.leg_fcl.position =
                            Vec3::new(-s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);
                        next.leg_fcr.position = Vec3::new(s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);

                        next.leg_bcl.position =
                            Vec3::new(-s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);
                        next.leg_bcr.position = Vec3::new(s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);

                        next.leg_bl.position = Vec3::new(-s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
                        next.leg_br.position = Vec3::new(s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
                    } else {
                        next.chest.scale = Vec3::one() * s_a.scaler;
                        next.chest.orientation = Quaternion::rotation_x(movement2abs * 0.3)
                            * Quaternion::rotation_z((movement1abs * 4.0 * PI).sin() * 0.02);

                        next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
                        next.head.orientation =
                            Quaternion::rotation_x(
                                movement1abs * 0.5 + movement2abs * -1.5 + movement3abs * 0.8,
                            ) * Quaternion::rotation_y(
                                mirror * movement1abs * -0.2 + mirror * movement2abs * 0.2,
                            ) * Quaternion::rotation_z((movement1abs * 4.0 * PI).sin() * 0.02);

                        next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);

                        next.mandible_l.position =
                            Vec3::new(-s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);
                        next.mandible_r.position =
                            Vec3::new(s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);
                        next.mandible_l.orientation = Quaternion::rotation_x(
                            movement1abs * 0.5 + movement2abs * -1.5 + movement3abs * 0.8,
                        ) * Quaternion::rotation_z(
                            movement1abs * 0.5 + movement2abs * -0.6 + movement3abs * 0.8,
                        );
                        next.mandible_r.orientation = Quaternion::rotation_x(
                            movement1abs * 0.5 + movement2abs * -1.5 + movement3abs * 0.8,
                        ) * Quaternion::rotation_z(
                            movement1abs * -0.5 + movement2abs * 0.6 + movement3abs * -0.8,
                        );

                        next.wing_fl.position =
                            Vec3::new(-s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);
                        next.wing_fr.position = Vec3::new(s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);

                        next.wing_bl.position =
                            Vec3::new(-s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);
                        next.wing_br.position = Vec3::new(s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);

                        next.leg_fl.position = Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
                        next.leg_fr.position = Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
                        next.leg_fl.orientation = Quaternion::rotation_z(
                            s_a.leg_ori.0
                                + movement1abs * 0.4
                                + movement2abs * -0.4
                                + movement3abs * 0.8,
                        ) * Quaternion::rotation_x(
                            movement1abs * 1.0 + movement2abs * -1.8 + movement3abs * 0.8,
                        );
                        next.leg_fr.orientation = Quaternion::rotation_z(
                            -s_a.leg_ori.0
                                + movement1abs * -0.4
                                + movement2abs * 0.4
                                + movement3abs * -0.8,
                        ) * Quaternion::rotation_x(
                            movement1abs * 1.0 + movement2abs * -1.8 + movement3abs * 0.8,
                        );

                        next.leg_fcl.position =
                            Vec3::new(-s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);
                        next.leg_fcr.position = Vec3::new(s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);

                        next.leg_bcl.position =
                            Vec3::new(-s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);
                        next.leg_bcr.position = Vec3::new(s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);

                        next.leg_bl.position = Vec3::new(-s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
                        next.leg_br.position = Vec3::new(s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
                    }
                },
                _ => {},
            }
        }

        next
    }
}
