use super::{
    super::{Animation, vek::*},
    ArthropodSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;
use std::f32::consts::PI;

pub struct BasicAction;

pub struct BasicActionDependency<'a> {
    pub ability_id: Option<&'a str>,
    pub stage_section: Option<StageSection>,
    pub global_time: f32,
    pub timer: f32,
}

impl Animation for BasicAction {
    type Dependency<'a> = BasicActionDependency<'a>;
    type Skeleton = ArthropodSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"arthropod_shoot\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "arthropod_shoot"))]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        d: Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (move1base, _chargebase, movementbase, move2base, move3base) = match d.stage_section {
            Some(StageSection::Buildup) => (anim_time, 0.0, 0.0, 0.0, 0.0),
            Some(StageSection::Charge) => (1.0, anim_time, 0.0, 0.0, 0.0),
            Some(StageSection::Movement) => (1.0, 1.0, anim_time, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, 1.0, 1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, 1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - move3base;
        let _move1 = move1base * pullback;
        let _move2 = move2base * pullback;

        match d.ability_id {
            Some(
                "common.abilities.custom.arthropods.blackwidow.poisonball"
                | "common.abilities.custom.arthropods.weevil.threadshot"
                | "common.abilities.custom.arthropods.crawler.threadshot",
            ) => {
                let movement1abs = move1base.powf(0.25) * pullback;
                let twitch = (move1base * 30.0).sin();

                next.chest.scale = Vec3::one() * s_a.scaler;
                next.chest.orientation = Quaternion::rotation_x(0.0) * Quaternion::rotation_z(0.0);

                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
                next.head.orientation =
                    Quaternion::rotation_x(movement1abs * 0.35 + twitch * -0.02)
                        * Quaternion::rotation_y(0.0);

                next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);

                next.mandible_l.position =
                    Vec3::new(-s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);
                next.mandible_r.position =
                    Vec3::new(s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);
                next.mandible_l.orientation =
                    Quaternion::rotation_x(movement1abs * 0.5 + twitch * 0.2)
                        * Quaternion::rotation_y(movement1abs * 0.5)
                        * Quaternion::rotation_z(movement1abs * 0.5);
                next.mandible_r.orientation =
                    Quaternion::rotation_x(movement1abs * 0.5 + twitch * 0.2)
                        * Quaternion::rotation_y(movement1abs * -0.5)
                        * Quaternion::rotation_z(movement1abs * -0.5);

                next.wing_fl.position = Vec3::new(-s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);
                next.wing_fr.position = Vec3::new(s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);

                next.wing_bl.position = Vec3::new(-s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);
                next.wing_br.position = Vec3::new(s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);

                next.leg_fl.position = Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
                next.leg_fr.position = Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
                next.leg_fl.orientation =
                    Quaternion::rotation_z(s_a.leg_ori.0 + movement1abs * 0.4)
                        * Quaternion::rotation_x(movement1abs * 1.0);
                next.leg_fr.orientation =
                    Quaternion::rotation_z(-s_a.leg_ori.0 + movement1abs * -0.4)
                        * Quaternion::rotation_x(movement1abs * 1.0);

                next.leg_fcl.orientation =
                    Quaternion::rotation_z(s_a.leg_ori.1 + movement1abs * 0.2)
                        * Quaternion::rotation_y(movement1abs * 0.5);
                next.leg_fcr.orientation =
                    Quaternion::rotation_z(-s_a.leg_ori.1 + movement1abs * -0.2)
                        * Quaternion::rotation_y(movement1abs * -0.5);

                next.leg_fcl.position = Vec3::new(-s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);
                next.leg_fcr.position = Vec3::new(s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);

                next.leg_bcl.position = Vec3::new(-s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);
                next.leg_bcr.position = Vec3::new(s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);

                next.leg_bl.position = Vec3::new(-s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
                next.leg_br.position = Vec3::new(s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
            },
            Some("common.abilities.custom.arthropods.antlion.charge") => {
                let movement1abs = move1base.powi(4) * pullback;
                let movement2abs = move2base.powi(6) * pullback;
                let chargemovementbase = if matches!(d.stage_section, Some(StageSection::Buildup)) {
                    0.0
                } else {
                    1.0
                };
                let shortalt =
                    (anim_time * 200.0 + PI * 0.25).sin() * chargemovementbase * pullback;

                next.chest.scale = Vec3::one() * s_a.scaler;

                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
                next.head.orientation =
                    Quaternion::rotation_x(movement1abs * -0.4 + movement2abs * 1.4);

                next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1);

                next.mandible_l.position =
                    Vec3::new(-s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);
                next.mandible_r.position =
                    Vec3::new(s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);
                next.mandible_l.orientation =
                    Quaternion::rotation_z(movement1abs * 0.5 + movement2abs * -0.7);
                next.mandible_r.orientation =
                    Quaternion::rotation_z(movement1abs * -0.5 + movement2abs * 0.7);

                next.wing_fl.position = Vec3::new(-s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);
                next.wing_fr.position = Vec3::new(s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);
                next.wing_fl.orientation = Quaternion::rotation_x(
                    movement1abs * -0.4 + shortalt * 0.2 + movement2abs * -0.6,
                ) * Quaternion::rotation_y(
                    movement1abs * -0.5 + movement2abs * -0.1,
                ) * Quaternion::rotation_z(movement1abs * -0.2);
                next.wing_fr.orientation = Quaternion::rotation_x(
                    movement1abs * -0.4 + shortalt * 0.2 + movement2abs * -0.6,
                ) * Quaternion::rotation_y(
                    movement1abs * 0.5 + movement2abs * 0.1,
                ) * Quaternion::rotation_z(movement1abs * 0.2);

                next.wing_bl.position = Vec3::new(-s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);
                next.wing_br.position = Vec3::new(s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);
                next.wing_bl.orientation =
                    Quaternion::rotation_x(
                        movement1abs * -0.2 + shortalt * 0.2 + movement2abs * -0.6,
                    ) * Quaternion::rotation_y(movement1abs * -0.4 + movement2abs * -0.1);
                next.wing_br.orientation =
                    Quaternion::rotation_x(
                        movement1abs * -0.2 + shortalt * 0.2 + movement2abs * -0.6,
                    ) * Quaternion::rotation_y(movement1abs * 0.4 + movement2abs * 0.1);
            },
            Some(
                "common.abilities.custom.arthropods.tarantula.leap"
                | "common.abilities.custom.arthropods.hornbeetle.leap"
                | "common.abilities.custom.arthropods.emberfly.leap",
            ) => {
                let pullback = 1.0 - move3base.powi(4);
                let movement1abs = move1base * pullback;
                let movement2abs = movementbase.powf(0.1) * pullback;
                let movement3abs = move2base.powf(0.1) * pullback;
                let early_pullback = 1.0 - move2base.powf(0.1);
                let shortalt = (d.global_time * 80.0).sin() * movementbase * early_pullback;

                next.chest.scale = Vec3::one() * s_a.scaler;

                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
                next.head.orientation =
                    Quaternion::rotation_x(
                        movement1abs * -0.2 + movement2abs * 0.4 + movement3abs * -1.0,
                    ) * Quaternion::rotation_z((movement1abs * 4.0 * PI).sin() * 0.08);

                next.chest.position =
                    Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + movement1abs * -2.0);
                next.chest.orientation = Quaternion::rotation_x(movement2abs * 0.3)
                    * Quaternion::rotation_z((movement1abs * 4.0 * PI).sin() * 0.08);

                next.mandible_l.position =
                    Vec3::new(-s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);
                next.mandible_r.position =
                    Vec3::new(s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);
                next.mandible_l.orientation = Quaternion::rotation_x(
                    (movement1abs * 4.0 * PI).sin() * 0.08
                        + movement2abs * 0.3
                        + movement3abs * -0.4,
                );
                next.mandible_r.orientation = Quaternion::rotation_x(
                    (movement1abs * 4.0 * PI).sin() * 0.08
                        + movement2abs * 0.3
                        + movement3abs * -0.4,
                );

                next.wing_fl.position = Vec3::new(-s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);
                next.wing_fr.position = Vec3::new(s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);

                next.wing_bl.position = Vec3::new(-s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);
                next.wing_br.position = Vec3::new(s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);

                next.leg_fl.position = Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
                next.leg_fr.position = Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
                next.leg_fl.orientation = Quaternion::rotation_x(
                    movement1abs * 0.2 + movement2abs * 0.8 + movement3abs * -1.5,
                ) * Quaternion::rotation_z(s_a.leg_ori.0);
                next.leg_fr.orientation = Quaternion::rotation_x(
                    movement1abs * 0.2 + movement2abs * 0.8 + movement3abs * -1.5,
                ) * Quaternion::rotation_z(-s_a.leg_ori.0);

                next.leg_fcl.position = Vec3::new(-s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);
                next.leg_fcr.position = Vec3::new(s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);
                next.leg_fcl.orientation = Quaternion::rotation_y(
                    movement1abs * 0.2 + movement2abs * -1.0 + movement3abs * 0.8,
                ) * Quaternion::rotation_z(s_a.leg_ori.1);
                next.leg_fcr.orientation =
                    Quaternion::rotation_y(movement1abs * -0.2 + movement2abs * 1.0)
                        * Quaternion::rotation_z(-s_a.leg_ori.1);

                next.leg_bcl.position = Vec3::new(-s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);
                next.leg_bcr.position = Vec3::new(s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);
                next.leg_bcl.orientation = Quaternion::rotation_y(
                    movement1abs * 0.2 + movement2abs * -1.0 + movement3abs * 0.8,
                ) * Quaternion::rotation_z(s_a.leg_ori.2);
                next.leg_bcr.orientation =
                    Quaternion::rotation_y(movement1abs * -0.2 + movement2abs * 1.0)
                        * Quaternion::rotation_z(-s_a.leg_ori.2);

                next.leg_bl.position = Vec3::new(-s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
                next.leg_br.position = Vec3::new(s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
                next.leg_bl.orientation = Quaternion::rotation_y(
                    movement1abs * 0.2 + movement2abs * -1.0 + movement3abs * 0.8,
                ) * Quaternion::rotation_z(s_a.leg_ori.3);
                next.leg_br.orientation =
                    Quaternion::rotation_y(movement1abs * -0.2 + movement2abs * 1.0)
                        * Quaternion::rotation_z(-s_a.leg_ori.3);

                next.wing_fl.position = Vec3::new(-s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);
                next.wing_fr.position = Vec3::new(s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);
                next.wing_fl.orientation =
                    Quaternion::rotation_x(movement1abs * -0.4 + movement2abs * -0.2)
                        * Quaternion::rotation_y(movement1abs * 0.5 + movement2abs * 0.1)
                        * Quaternion::rotation_z(movement1abs * -0.2);
                next.wing_fr.orientation =
                    Quaternion::rotation_x(movement1abs * -0.4 + movement2abs * -0.2)
                        * Quaternion::rotation_y(movement1abs * -0.5 + movement2abs * -0.1)
                        * Quaternion::rotation_z(movement1abs * 0.2);

                next.wing_bl.position = Vec3::new(-s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);
                next.wing_br.position = Vec3::new(s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);
                next.wing_bl.orientation = Quaternion::rotation_x(
                    (movement1abs * -0.2 + movement2abs * -0.6) * early_pullback,
                ) * Quaternion::rotation_y(
                    movement1abs * 0.4 + shortalt * 2.0 + movement2abs * 0.1,
                ) * Quaternion::rotation_z(movement1abs * -1.4);
                next.wing_br.orientation = Quaternion::rotation_x(
                    (movement1abs * -0.2 + movement2abs * -0.6) * early_pullback,
                ) * Quaternion::rotation_y(
                    movement1abs * -0.4 + shortalt * 2.0 + movement2abs * -0.1,
                ) * Quaternion::rotation_z(movement1abs * 1.4);
            },
            Some("common.abilities.custom.arthropods.dagonite.leapshockwave") => {
                let pullback = 1.0 - move3base.powi(4);
                let movement1abs = move1base * pullback;
                let movement2abs = movementbase.powf(0.1) * pullback;
                let movement3abs = move2base.powf(0.1) * pullback;
                let early_pullback = 1.0 - move2base.powf(0.1);
                let shortalt = (d.global_time * 80.0).sin() * movementbase * early_pullback;

                next.chest.scale = Vec3::one() * s_a.scaler;

                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
                next.head.orientation =
                    Quaternion::rotation_x(
                        movement1abs * -0.2 + movement2abs * 0.4 + movement3abs * -1.0,
                    ) * Quaternion::rotation_z((movement1abs * 4.0 * PI).sin() * 0.08);

                next.chest.position =
                    Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + movement1abs * -0.25);
                next.chest.orientation = Quaternion::rotation_x(movement2abs * 0.15)
                    * Quaternion::rotation_z((movement1abs * 4.0 * PI).sin() * 0.08);

                next.mandible_l.position =
                    Vec3::new(-s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);
                next.mandible_r.position =
                    Vec3::new(s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);
                next.mandible_l.orientation = Quaternion::rotation_x(
                    (movement1abs * 4.0 * PI).sin() * 0.08
                        + movement2abs * 0.3
                        + movement3abs * -0.4,
                );
                next.mandible_r.orientation = Quaternion::rotation_x(
                    (movement1abs * 4.0 * PI).sin() * 0.08
                        + movement2abs * 0.3
                        + movement3abs * -0.4,
                );

                next.wing_fl.position = Vec3::new(-s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);
                next.wing_fr.position = Vec3::new(s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);

                next.wing_bl.position = Vec3::new(-s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);
                next.wing_br.position = Vec3::new(s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);

                next.leg_fl.position = Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
                next.leg_fr.position = Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
                next.leg_fl.orientation = Quaternion::rotation_x(
                    movement1abs * 0.2 + movement2abs * 0.8 + movement3abs * -1.5,
                ) * Quaternion::rotation_z(s_a.leg_ori.0);
                next.leg_fr.orientation = Quaternion::rotation_x(
                    movement1abs * 0.2 + movement2abs * 0.8 + movement3abs * -1.5,
                ) * Quaternion::rotation_z(-s_a.leg_ori.0);

                next.leg_fcl.position = Vec3::new(-s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);
                next.leg_fcr.position = Vec3::new(s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);
                next.leg_fcl.orientation = Quaternion::rotation_y(
                    movement1abs * 0.2 + movement2abs * -1.0 + movement3abs * 0.8,
                ) * Quaternion::rotation_z(s_a.leg_ori.1);
                next.leg_fcr.orientation =
                    Quaternion::rotation_y(movement1abs * -0.2 + movement2abs * 1.0)
                        * Quaternion::rotation_z(-s_a.leg_ori.1);

                next.leg_bcl.position = Vec3::new(-s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);
                next.leg_bcr.position = Vec3::new(s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);
                next.leg_bcl.orientation = Quaternion::rotation_y(
                    movement1abs * 0.2 + movement2abs * -1.0 + movement3abs * 0.8,
                ) * Quaternion::rotation_z(s_a.leg_ori.2);
                next.leg_bcr.orientation =
                    Quaternion::rotation_y(movement1abs * -0.2 + movement2abs * 1.0)
                        * Quaternion::rotation_z(-s_a.leg_ori.2);

                next.leg_bl.position = Vec3::new(-s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
                next.leg_br.position = Vec3::new(s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
                next.leg_bl.orientation = Quaternion::rotation_y(
                    movement1abs * 0.2 + movement2abs * -1.0 + movement3abs * 0.8,
                ) * Quaternion::rotation_z(s_a.leg_ori.3);
                next.leg_br.orientation =
                    Quaternion::rotation_y(movement1abs * -0.2 + movement2abs * 1.0)
                        * Quaternion::rotation_z(-s_a.leg_ori.3);

                next.wing_fl.position = Vec3::new(-s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);
                next.wing_fr.position = Vec3::new(s_a.wing_f.0, s_a.wing_f.1, s_a.wing_f.2);
                next.wing_fl.orientation =
                    Quaternion::rotation_x(movement1abs * -0.4 + movement2abs * -0.2)
                        * Quaternion::rotation_y(movement1abs * 0.5 + movement2abs * 0.1)
                        * Quaternion::rotation_z(movement1abs * -0.2);
                next.wing_fr.orientation =
                    Quaternion::rotation_x(movement1abs * -0.4 + movement2abs * -0.2)
                        * Quaternion::rotation_y(movement1abs * -0.5 + movement2abs * -0.1)
                        * Quaternion::rotation_z(movement1abs * 0.2);

                next.wing_bl.position = Vec3::new(-s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);
                next.wing_br.position = Vec3::new(s_a.wing_b.0, s_a.wing_b.1, s_a.wing_b.2);
                next.wing_bl.orientation = Quaternion::rotation_x(
                    (movement1abs * -0.2 + movement2abs * -0.6) * early_pullback,
                ) * Quaternion::rotation_y(
                    movement1abs * 0.4 + shortalt * 2.0 + movement2abs * 0.1,
                ) * Quaternion::rotation_z(movement1abs * -1.4);
                next.wing_br.orientation = Quaternion::rotation_x(
                    (movement1abs * -0.2 + movement2abs * -0.6) * early_pullback,
                ) * Quaternion::rotation_y(
                    movement1abs * -0.4 + shortalt * 2.0 + movement2abs * -0.1,
                ) * Quaternion::rotation_z(movement1abs * 1.4);
            },
            Some(
                "common.abilities.custom.arthropods.tarantula.ensnaringwebs"
                | "common.abilities.custom.arthropods.blackwidow.ensnaringwebs",
            ) => {
                let subtract = d.global_time - d.timer;
                let check = subtract - subtract.trunc();
                let mirror = (check - 0.5).signum();
                let movement1abs = move1base.powi(2) * pullback;
                let movement2abs = move2base.powi(4) * pullback;
                let movement3abs = move3base * pullback;

                next.chest.scale = Vec3::one() * s_a.scaler;
                next.chest.orientation = Quaternion::rotation_x(movement2abs * 0.3)
                    * Quaternion::rotation_z((movement1abs * 4.0 * PI).sin() * 0.02);

                next.head.position = Vec3::new(
                    0.0,
                    s_a.head.0 + movement1abs * 3.0,
                    s_a.head.1 + movement1abs * -3.0,
                );
                next.head.orientation =
                    Quaternion::rotation_x(
                        movement1abs * 1.5 + movement2abs * -1.5 + movement3abs * 0.8,
                    ) * Quaternion::rotation_y(
                        mirror * movement1abs * -0.2 + mirror * movement2abs * 0.2,
                    ) * Quaternion::rotation_z((movement1abs * 4.0 * PI).sin() * 0.02);

                next.chest.position = Vec3::new(
                    0.0,
                    s_a.chest.0,
                    s_a.chest.1 + movement1abs * 7.0 + movement2abs * -2.0,
                );
                next.chest.orientation =
                    Quaternion::rotation_x(movement1abs * -1.0 + movement2abs * 0.2);
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

                next.leg_fl.position = Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
                next.leg_fr.position = Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
                next.leg_fl.orientation =
                    Quaternion::rotation_x(movement1abs * 1.0 + movement2abs * 0.2)
                        * Quaternion::rotation_z(movement1abs * -0.2 + movement2abs * -0.2);
                next.leg_fr.orientation =
                    Quaternion::rotation_x(movement1abs * 1.0 + movement2abs * 0.2)
                        * Quaternion::rotation_x(movement1abs * 0.2 + movement2abs * 0.2);

                next.leg_fcl.position = Vec3::new(-s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);
                next.leg_fcr.position = Vec3::new(s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);

                next.leg_fcl.orientation =
                    Quaternion::rotation_x(movement1abs * 1.3 + movement2abs * 0.3)
                        * Quaternion::rotation_z(movement1abs * -0.5 + movement2abs * -0.2);
                next.leg_fcr.orientation =
                    Quaternion::rotation_x(movement1abs * 1.3 + movement2abs * 0.3)
                        * Quaternion::rotation_z(movement1abs * 0.5 + movement2abs * -0.2);

                next.leg_bcl.position = Vec3::new(-s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);
                next.leg_bcr.position = Vec3::new(s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);

                next.leg_bcl.orientation =
                    Quaternion::rotation_x(movement1abs * 0.5 + movement2abs * 0.2);
                next.leg_bcr.orientation =
                    Quaternion::rotation_x(movement1abs * 0.5 + movement2abs * 0.2);

                next.leg_bl.position = Vec3::new(-s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
                next.leg_br.position = Vec3::new(s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);

                next.leg_bl.orientation =
                    Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * -0.2)
                        * Quaternion::rotation_z(movement1abs * 0.8);
                next.leg_br.orientation =
                    Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * -0.2)
                        * Quaternion::rotation_z(movement1abs * -0.8);
            },
            _ => {},
        }

        next
    }
}
