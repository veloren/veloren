use super::{
    super::{Animation, vek::*},
    CharacterSkeleton, SkeletonAttr, bow_draw, bow_start, dual_wield_start, hammer_start,
    twist_back, twist_forward,
};
use common::{
    comp::item::Hands,
    states::utils::{AbilityInfo, HandInfo, StageSection},
    util::Dir,
};
use core::f32::consts::{PI, TAU};

pub struct BasicAction;

pub struct BasicActionDependency<'a> {
    pub ability_id: Option<&'a str>,
    pub hands: (Option<Hands>, Option<Hands>),
    pub stage_section: Option<StageSection>,
    pub ability_info: Option<AbilityInfo>,
    pub velocity: Vec3<f32>,
    pub last_ori: Vec3<f32>,
    pub orientation: Vec3<f32>,
    pub look_dir: Dir,
    pub is_riding: bool,
}

impl Animation for BasicAction {
    type Dependency<'a> = BasicActionDependency<'a>;
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_basic\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "character_basic"))]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        d: Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        // Don't use this for future animations
        let mut legacy_initialize = || {
            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_z(0.0);
            next.second.position = Vec3::new(0.0, 0.0, 0.0);
            next.second.orientation = Quaternion::rotation_z(0.0);
        };

        if matches!(d.stage_section, Some(StageSection::Action)) {
            next.main_weapon_trail = true;
            next.off_weapon_trail = true;
        }
        let (move1base, chargebase, movementbase, move2base, move3base) = match d.stage_section {
            Some(StageSection::Buildup) => (anim_time, 0.0, 0.0, 0.0, 0.0),
            Some(StageSection::Charge) => (1.0, anim_time, 0.0, 0.0, 0.0),
            Some(StageSection::Movement) => (1.0, 1.0, anim_time, 0.0, 0.0),
            Some(StageSection::Action) => (1.0, 1.0, 1.0, anim_time, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, 1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - move3base;
        let move1 = move1base * pullback;
        let move2 = move2base * pullback;

        match d.ability_id {
            // ==================================
            //               SWORD
            // ==================================
            Some(
                "common.abilities.sword.basic_guard" | "common.abilities.sword.defensive_guard",
            ) => {
                legacy_initialize();
                let pullback = 1.0 - move3base.powi(4);
                let move1 = move1base.powf(0.25) * pullback;
                let move2 = (move2base * 10.0).sin();

                if d.velocity.xy().magnitude_squared() < 0.5_f32.powi(2) {
                    next.chest.position =
                        Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + move1 * -1.0 + move2 * 0.2);
                    next.chest.orientation = Quaternion::rotation_x(move1 * -0.15);
                    next.head.orientation = Quaternion::rotation_x(move1 * 0.25);

                    next.belt.position =
                        Vec3::new(0.0, s_a.belt.0 + move1 * 0.5, s_a.belt.1 + move1 * 0.5);
                    next.shorts.position =
                        Vec3::new(0.0, s_a.shorts.0 + move1 * 1.3, s_a.shorts.1 + move1 * 1.0);

                    next.belt.orientation = Quaternion::rotation_x(move1 * 0.15);
                    next.shorts.orientation = Quaternion::rotation_x(move1 * 0.25);

                    if !d.is_riding {
                        next.foot_l.position =
                            Vec3::new(-s_a.foot.0, s_a.foot.1 + move1 * 2.0, s_a.foot.2);
                        next.foot_l.orientation = Quaternion::rotation_z(move1 * -0.5);

                        next.foot_r.position =
                            Vec3::new(s_a.foot.0, s_a.foot.1 + move1 * -2.0, s_a.foot.2);
                        next.foot_r.orientation = Quaternion::rotation_x(move1 * -0.5);
                    }
                }

                match d.hands {
                    (Some(Hands::Two), _) => {
                        next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                        next.hand_l.orientation =
                            Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                        next.hand_r.position = Vec3::new(
                            s_a.shr.0 + move1 * -2.0,
                            s_a.shr.1,
                            s_a.shr.2 + move1 * 20.0,
                        );
                        next.hand_r.orientation = Quaternion::rotation_x(s_a.shr.3)
                            * Quaternion::rotation_y(s_a.shr.4)
                            * Quaternion::rotation_z(move1 * 1.5);

                        next.control.position =
                            Vec3::new(s_a.sc.0 + move1 * -3.0, s_a.sc.1, s_a.sc.2 + move1 * 4.0);
                        next.control.orientation = Quaternion::rotation_x(s_a.sc.3)
                            * Quaternion::rotation_y(move1 * 1.1)
                            * Quaternion::rotation_z(move1 * 1.7);
                    },
                    (Some(Hands::One), offhand) => {
                        next.control_l.position =
                            Vec3::new(-7.0, 8.0 + move1 * 3.0, 2.0 + move1 * 3.0);
                        next.control_l.orientation =
                            Quaternion::rotation_x(-0.3) * Quaternion::rotation_y(move1 * 1.0);
                        next.hand_l.position = Vec3::new(0.0, -0.5, 0.0);
                        next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0);
                        if offhand.is_some() {
                            next.control_r.position =
                                Vec3::new(7.0, 8.0 + move1 * 3.0, 2.0 + move1 * 3.0);
                            next.control_r.orientation =
                                Quaternion::rotation_x(-0.3) * Quaternion::rotation_y(move1 * -1.0);
                            next.hand_r.position = Vec3::new(0.0, -0.5, 0.0);
                            next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
                        } else {
                            next.hand_r.position = Vec3::new(4.5, 8.0, 5.0);
                            next.hand_r.orientation =
                                Quaternion::rotation_x(1.9) * Quaternion::rotation_y(0.5)
                        }
                    },
                    (_, _) => {},
                }
            },
            Some("common.abilities.sword.defensive_deflect") => {
                legacy_initialize();
                let move1 = move1base.powi(2);
                let move2 = (move2base * 20.0).sin();
                let move3 = move3base.powf(0.5);

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation =
                    Quaternion::rotation_x(s_a.sc.3) * Quaternion::rotation_z(move1 * -0.9);

                next.chest.orientation = Quaternion::rotation_z(move1 * -0.6);
                next.head.orientation = Quaternion::rotation_z(move1 * 0.2);
                next.belt.orientation = Quaternion::rotation_z(move1 * -0.1);
                next.shorts.orientation = Quaternion::rotation_z(move1 * 0.1);
                next.control.orientation.rotate_y(move1 * -1.7);
                next.control.orientation.rotate_z(move1 * 0.6);
                next.control.position += Vec3::new(move1 * 11.0, move1 * 2.0, move1 * 5.0);

                next.control.orientation.rotate_y(move2 / 50.0);

                next.chest.orientation.rotate_z(move3 * -0.6);
                next.head.orientation.rotate_z(move3 * 0.4);
                next.belt.orientation.rotate_z(move3 * 0.2);
                next.shorts.orientation.rotate_z(move3 * 0.6);
                next.control.position += Vec3::new(move3 * 6.0, 0.0, move3 * 9.0);
                next.control.orientation.rotate_z(move3 * -0.5);
                next.control.orientation.rotate_y(move3 * 0.6);
            },
            Some(
                "common.abilities.sword.basic_thrust"
                | "common.abilities.sword.defensive_vital_jab",
            ) => {
                legacy_initialize();
                let pullback = 1.0 - move3base.powi(4);
                let move1 = chargebase.powf(0.25).min(1.0) * pullback;
                let move2 = move2base.powi(2) * pullback;
                let tension = (chargebase * 20.0).sin();

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -13.2, -4.0 + move1 * 3.3, 1.0);
                next.hand_r.orientation = Quaternion::rotation_x(PI * 0.5);
                next.chest.position += Vec3::new(0.0, move1 * -0.55, 0.0);
                next.control.position =
                    Vec3::new(s_a.sc.0, s_a.sc.1 + move2 * 3.3, s_a.sc.2 + move2 * 5.5);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3 + move1 * -0.99)
                    * Quaternion::rotation_y(move1 * 1.1 + move2 * -1.1)
                    * Quaternion::rotation_z(move1 * 1.43 + move2 * -1.43);

                next.chest.position += Vec3::new(0.0, move2 * 2.2, 0.0);
                next.chest.orientation =
                    Quaternion::rotation_z(move1 * 1.1 + tension * 0.02 + move2 * -1.32);
                next.head.position += Vec3::new(0.0, move2 * 1.1, 0.0);
                next.head.orientation = Quaternion::rotation_x(move1 * 0.055 + move2 * -0.055)
                    * Quaternion::rotation_y(move1 * 0.055 + move2 * -0.055)
                    * Quaternion::rotation_z(move1 * -0.44 + move2 * 0.33);
                next.belt.orientation = Quaternion::rotation_z(move1 * -0.275 + move2 * 0.22);
                next.shorts.orientation = Quaternion::rotation_z(move1 * -0.66 + move2 * 0.33);
            },
            Some("common.abilities.sword.heavy_slam") => {
                legacy_initialize();
                let pullback = 1.0 - move3base.powi(4);
                let move1 = chargebase.powf(0.25).min(1.0) * pullback;
                let move2 = move2base.powi(2) * pullback;
                let tension = (chargebase * 20.0).sin();

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 7.0 + move1 * -13.2, -5.0 + move1 * 3.3, -1.0);
                next.chest.position += Vec3::new(0.0, move1 * -0.55, 0.0);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3)
                    * Quaternion::rotation_z(move1 * 0.33 + move2 * -0.77);

                next.control
                    .orientation
                    .rotate_x(move1 * 1.54 + tension / 50.0);
                next.control.position +=
                    Vec3::new(move1 * -2.2, 0.0, move1 * 9.9) + Vec3::one() * tension / 4.0;
                next.chest.position += Vec3::new(0.0, move2 * 1.65, 0.0);
                next.chest.orientation = Quaternion::rotation_z(move1 * 0.44 + tension / 50.0);
                next.head.position += Vec3::new(0.0, move2 * 1.1, 0.0);
                next.head.orientation = Quaternion::rotation_x(move2 * -0.22)
                    * Quaternion::rotation_y(move1 * 0.055 + move2 * 0.055)
                    * Quaternion::rotation_z(move2 * -0.165);
                next.belt.orientation = Quaternion::rotation_z(move1 * -0.055 + move2 * 0.165);
                next.shorts.orientation = Quaternion::rotation_z(move1 * -0.11 + move2 * 0.22);

                if move2 < f32::EPSILON {
                    next.main_weapon_trail = false;
                    next.off_weapon_trail = false;
                }
                next.control.orientation.rotate_x(move2 * -3.3);
                next.control.orientation.rotate_z(move2 * -0.44);
                next.control.position += Vec3::new(move2 * 11.0, move2 * 5.5, move2 * -11.0);
                next.chest.orientation.rotate_z(move2 * -0.66);
            },
            Some("common.abilities.sword.crippling_deep_rend") => {
                legacy_initialize();
                let pullback = 1.0 - move3base;
                let move1pre = move1base.min(0.5) * 2.0 * pullback;
                let move1post = (move1base.max(0.5) * 2.0 - 1.0) * pullback;
                let move2 = chargebase.min(1.0) * pullback;
                let move3 = move2base * pullback;
                let tension = (chargebase * 20.0).sin();

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 3.0 + move1 * -9.0, -4.0 + move1 * 3.0, 0.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1pre * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation =
                    Quaternion::rotation_x(s_a.sc.3) * Quaternion::rotation_z(move1pre * PI / 2.0);

                next.foot_r.position += Vec3::new(0.0, move1pre * -2.5, 0.0);
                next.foot_r.orientation.rotate_z(move1pre * -1.2);
                next.chest.position += Vec3::new(0.0, move1pre * -2.0, 0.0);
                next.chest.orientation = Quaternion::rotation_z(move1pre * -1.3);
                next.head.orientation = Quaternion::rotation_x(move1pre * -0.05)
                    * Quaternion::rotation_y(move1pre * -0.05)
                    * Quaternion::rotation_z(move1pre * 0.7);
                next.belt.orientation = Quaternion::rotation_z(move1pre * 0.4);
                next.shorts.orientation = Quaternion::rotation_z(move1pre * 0.8);
                next.control.orientation.rotate_y(move1pre * -1.5);
                next.control.orientation.rotate_z(move1pre * 0.0);
                next.control.position += Vec3::new(move1pre * 9.0, move1pre * -1.5, 0.0);

                next.chest.position += Vec3::new(0.0, move1post * 2.0, 0.0);
                next.chest.orientation.rotate_z(move1post * 1.2);
                next.head.position += Vec3::new(0.0, move1post * 1.0, 0.0);
                next.head.orientation.rotate_z(move1post * -0.7);
                next.belt.orientation.rotate_z(move1post * -0.3);
                next.shorts.orientation.rotate_z(move1post * -0.8);
                next.foot_r.orientation.rotate_z(move1post * 1.2);
                next.foot_r.orientation.rotate_x(move1post * -0.4);
                next.control.orientation.rotate_z(move1post * -1.2);
                next.control.position += Vec3::new(0.0, move1post * 8.0, move1post * 3.0);

                next.control
                    .orientation
                    .rotate_y(move2 * -2.0 + tension / 10.0);
                next.chest.orientation.rotate_z(move2 * -0.4 + move3 * -1.4);
                next.control
                    .orientation
                    .rotate_z(move2 * 0.3 + move3 * -1.3);
                next.head.orientation.rotate_z(move2 * 0.2 + move3 * 0.7);
                next.belt.orientation.rotate_z(move3 * 0.3);
                next.shorts.orientation.rotate_z(move2 * 0.2 + move3 * 0.6);
                next.chest
                    .orientation
                    .rotate_y(move2 * -0.3 - tension / 100.0);
                next.foot_r.orientation.rotate_z(move3 * -1.2);
            },
            Some(
                "common.abilities.sword.cleaving_spiral_slash"
                | "common.abilities.sword.cleaving_dual_spiral_slash",
            ) => {
                legacy_initialize();
                let move1 = chargebase.powf(0.25).min(1.0) * pullback;
                let move2_pre = move2base.min(0.3) * 10.0 / 3.0 * pullback;
                let tension = (chargebase * 15.0).sin();

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -1.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3);

                next.chest.orientation = Quaternion::rotation_z(move1 * 1.2 + tension / 70.0);
                next.head.orientation =
                    Quaternion::rotation_y(move1 * 0.05) * Quaternion::rotation_z(move1 * -0.5);
                next.belt.orientation = Quaternion::rotation_z(move1 * -0.3);
                next.shorts.orientation = Quaternion::rotation_z(move1 * -0.6);
                next.control.position += Vec3::new(0.0, move1 * 1.0, move1 * 1.0);
                next.control.orientation.rotate_x(move1 * 0.2);
                next.control.orientation.rotate_y(move1 * -1.0);

                next.control.orientation.rotate_y(move2_pre * -1.6);
                next.control.position += Vec3::new(0.0, 0.0, move2_pre * 4.0);
                next.torso.orientation.rotate_z(move2base * -TAU);
                next.chest.orientation.rotate_z(move2 * -2.0);
                next.head.orientation.rotate_z(move2 * 1.3);
                next.belt.orientation.rotate_z(move2 * 0.6);
                next.shorts.orientation.rotate_z(move2 * 1.5);
                next.control.orientation.rotate_y(move2 * 1.6);
                next.control.orientation.rotate_z(move2 * -1.8);
                next.control.position += Vec3::new(move2 * 14.0, 0.0, 0.0);
            },
            Some("common.abilities.sword.cleaving_earth_splitter") => {
                legacy_initialize();

                let pullback = 1.0 - move3base.powi(4);
                let move1 = movementbase.min(1.0).powi(2) * pullback;
                let move1alt = movementbase.min(1.0).powf(0.5);
                let move2 = move2base;
                let move3 = move2base.powf(0.25) * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3);
                next.torso.orientation.rotate_x(move1alt * -TAU);

                next.torso.orientation.rotate_x(move1 * -0.8);
                next.control.orientation.rotate_x(move1 * 1.5);
                next.control.position += Vec3::new(move1 * 7.0, move1.powi(4) * -6.0, move1 * 20.0);

                next.torso.orientation.rotate_x(move2 * 0.8);
                next.chest.orientation = Quaternion::rotation_x(move2 * -0.4);
                next.control.orientation.rotate_x(move2 * -1.2);
                next.control.position += Vec3::new(0.0, move2 * 12.0, move2 * -8.0);

                next.control.orientation.rotate_x(move3 * -1.2);
                next.control.position += Vec3::new(0.0, move3 * 4.0, move3 * -8.0);
            },
            Some("common.abilities.sword.heavy_pillar_thrust") => {
                legacy_initialize();
                let pullback = 1.0 - move3base.powi(4);
                let move1 = move1base.powf(0.5) * pullback;
                let move1alt1 = move1base.powf(0.5).min(0.5) * 2.0 * pullback;
                let move1alt2 = (move1base.powf(0.5).max(0.5) - 0.5) * 2.0 * pullback;
                let move2 = move2base.powi(2) * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation = Quaternion::rotation_x(s_a.shl.3 + move1alt2 * PI)
                    * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position = Vec3::new(
                    -s_a.sc.0 + 6.0 + move1alt1 * -12.0,
                    -4.0 + move1alt1 * 3.0,
                    -2.0,
                );
                next.hand_r.orientation =
                    Quaternion::rotation_x(0.9 + move1 * 0.5 + move1alt1 * PI);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3);

                next.control.position += Vec3::new(
                    move1 * 6.0,
                    (1.0 - (move1 - 0.5).abs() * 2.0) * 3.0,
                    move1 * 22.0,
                );
                next.control.orientation.rotate_x(move1 * -1.5);

                next.chest.orientation = Quaternion::rotation_x(move2 * -0.4);
                next.head.orientation = Quaternion::rotation_x(move2 * 0.2);
                next.belt.orientation = Quaternion::rotation_x(move2 * 0.4);
                next.shorts.orientation = Quaternion::rotation_x(move2 * 0.8);
                next.control.orientation.rotate_x(move2 * -0.4);
                next.control.position += Vec3::new(0.0, 0.0, move2 * -10.0);
                next.belt.position += Vec3::new(0.0, move2 * 2.0, move2 * 0.0);
                next.shorts.position += Vec3::new(0.0, move2 * 4.0, move2 * 1.0);
                next.chest.position += Vec3::new(0.0, move2 * -2.5, 0.0);
            },
            Some("common.abilities.sword.basic_mighty_strike") => {
                legacy_initialize();
                let pullback = 1.0 - move3base.powi(4);
                let move1 = move1base.powf(0.25) * pullback;
                let move2 = move2base.powf(0.1) * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(
                    s_a.sc.0 + move1 * -2.0 + move2 * 14.0,
                    s_a.sc.1 + move2 * 4.0,
                    s_a.sc.2 + move1 * 10.0 - move2 * 12.0,
                );
                next.control.orientation =
                    Quaternion::rotation_x(s_a.sc.3 + move1 * 1.6 + move2 * -2.6)
                        * Quaternion::rotation_y(move1 * -0.4 + move2 * 0.6)
                        * Quaternion::rotation_z(move1 * -0.2 + move2 * -0.2);

                next.chest.position += Vec3::new(0.0, move1 * -1.0 + move2 * 2.0, 0.0);
                next.chest.orientation = Quaternion::rotation_z(move1 * 1.0 + move2 * -1.2);
                next.head.position += Vec3::new(0.0, move2 * 1.0, 0.0);
                next.head.orientation = Quaternion::rotation_x(move1 * 0.05 + move2 * -0.25)
                    * Quaternion::rotation_y(move1 * -0.05 + move2 * 0.05)
                    * Quaternion::rotation_z(move1 * -0.5 + move2 * 0.4);
                next.belt.orientation = Quaternion::rotation_z(move1 * -0.25 + move2 * 0.2);
                next.shorts.orientation = Quaternion::rotation_z(move1 * -0.5 + move2 * 0.4);
            },
            Some("common.abilities.sword.heavy_guillotine") => {
                legacy_initialize();
                let pullback = 1.0 - move3base.powi(4);
                let move1 = move1base.powf(0.25) * pullback;
                let move2 = move2base.powi(2) * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -1.0);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3)
                    * Quaternion::rotation_z(move1 * 0.3 + move2 * -0.7);

                next.control.orientation.rotate_x(move1 * 1.4);
                next.control.position += Vec3::new(move1 * -2.0, move1 * -2.0, move1 * 10.0);
                next.chest.position += Vec3::new(0.0, move1 * -1.0 + move2 * 2.0, 0.0);
                next.chest.orientation = Quaternion::rotation_z(move1 * 0.4 + move2 * -0.6);
                next.head.position += Vec3::new(0.0, move2 * 1.0, 0.0);
                next.head.orientation = Quaternion::rotation_x(move1 * 0.05 + move2 * -0.25)
                    * Quaternion::rotation_y(move1 * 0.1 + move2 * -0.05)
                    * Quaternion::rotation_z(move1 * 0.1 + move2 * -0.25);

                next.control.orientation.rotate_x(move2 * -3.0);
                next.control.orientation.rotate_z(move2 * -0.4);
                next.control.position += Vec3::new(move2 * 12.0, move2 * 6.0, move2 * -11.0);
            },
            Some("common.abilities.sword.defensive_counter") => {
                legacy_initialize();
                let pullback = 1.0 - move3base.powi(4);
                let move1 = move1base.powf(0.5) * pullback;
                let move2 = (move2base.min(2.0 / 3.0) * 1.5).powi(2) * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation =
                    Quaternion::rotation_x(s_a.sc.3) * Quaternion::rotation_z(move2 * -PI / 4.0);

                if !d.is_riding {
                    next.foot_l.position = Vec3::new(-s_a.foot.0, s_a.foot.1, s_a.foot.2);
                    next.foot_r.position = Vec3::new(s_a.foot.0, s_a.foot.1, s_a.foot.2);
                    next.foot_l.orientation = Quaternion::identity();
                    next.foot_r.orientation = Quaternion::identity();
                }

                next.foot_l.position += Vec3::new(0.0, move1 * 4.0, 1.0 - (move1 - 0.5) * 2.0);
                next.torso.position += Vec3::new(0.0, move1 * -2.0, 0.0);
                next.chest.position += Vec3::new(0.0, move1 * 2.0, move1 * -3.0);
                next.head.position += Vec3::new(0.0, move1 * 1.0, 0.0);
                next.shorts.orientation = Quaternion::rotation_x(move1 * 0.3);
                next.shorts.position += Vec3::new(0.0, move1 * 1.5, 0.0);
                next.control.orientation.rotate_y(move1 * -1.5);
                next.control.orientation.rotate_z(move1 * 0.8);

                next.chest.orientation = Quaternion::rotation_z(move2 * -0.8);
                next.head.orientation =
                    Quaternion::rotation_x(move2 * 0.05) * Quaternion::rotation_z(move2 * 0.35);
                next.shorts.orientation.rotate_z(move2 * 0.5);
                next.belt.orientation = Quaternion::rotation_z(move2 * 0.3);
                next.control.orientation.rotate_z(move2 * -1.8);
                next.control.orientation.rotate_x(move2 * 0.3);
                next.control.position += Vec3::new(move2 * 7.0, move2 * 7.0, move2 * 6.0);
            },
            Some("common.abilities.sword.defensive_riposte") => {
                legacy_initialize();
                let pullback = 1.0 - move3base.powi(4);
                let move1 = move1base.powf(0.25) * pullback;
                let move2_slow = move2base.powi(8) * pullback;
                let move2 = move2base.powi(2) * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3)
                    * Quaternion::rotation_z(move1 * 1.3 + move2 * -0.7);

                next.chest.position += Vec3::new(0.0, move1 * -1.0, 0.0);
                next.chest.orientation = Quaternion::rotation_z(move1 * 0.8);
                next.head.orientation = Quaternion::rotation_x(move1 * 0.05)
                    * Quaternion::rotation_y(move1 * 0.05)
                    * Quaternion::rotation_z(move1 * -0.4);
                next.belt.orientation = Quaternion::rotation_z(move1 * -0.2);
                next.shorts.orientation = Quaternion::rotation_z(move1 * -0.5);
                next.control.orientation.rotate_x(move1 * 0.5);
                next.control.orientation.rotate_y(move1 * 2.1);
                next.control.orientation.rotate_z(move1 * -0.5);
                next.control.position += Vec3::new(0.0, move1 * 5.0, move1 * 8.0);

                next.chest.position += Vec3::new(0.0, move2 * 2.0, 0.0);
                next.chest.orientation.rotate_z(move2 * -1.4);
                next.head.position += Vec3::new(0.0, move2 * 1.0, 0.0);
                next.head.orientation.rotate_x(move2 * -0.1);
                next.head.orientation.rotate_y(move2 * -0.1);
                next.head.orientation.rotate_z(move2 * 0.8);
                next.belt.orientation.rotate_z(move2 * -0.3);
                next.shorts.orientation.rotate_z(move2 * 0.6);
                next.control.orientation.rotate_y(move2 * -4.0);
                next.control
                    .orientation
                    .rotate_z(move2_slow * -3.0 + move2 * 1.0);
                next.control.position +=
                    Vec3::new(move2_slow * 14.0, move2_slow * -2.0, move2_slow * -7.0);
            },
            Some("common.abilities.sword.heavy_fortitude") => {
                legacy_initialize();
                let pullback = 1.0 - move3base.powi(4);
                let move1 = move1base.powf(0.25) * pullback;
                let move2 = move2base.powi(2) * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3);

                next.foot_l.position += Vec3::new(move1 * 1.0, move1 * 2.0, 0.0);
                next.chest.orientation = Quaternion::rotation_z(move1 * -0.4);
                next.head.orientation = Quaternion::rotation_z(move1 * 0.2);
                next.shorts.orientation = Quaternion::rotation_z(move1 * 0.3);
                next.belt.orientation = Quaternion::rotation_z(move1 * 0.1);
                next.control.orientation.rotate_x(move1 * 0.4 + move2 * 0.6);
                next.control.orientation.rotate_z(move1 * 0.4);

                next.foot_r.position += Vec3::new(move2 * -1.0, move2 * -2.0, 0.0);
                next.control.position += Vec3::new(move2 * 5.0, move2 * 7.0, move2 * 5.0);
                next.chest.position += Vec3::new(0.0, 0.0, move2 * -1.0);
                next.shorts.orientation.rotate_x(move2 * 0.2);
                next.shorts.position += Vec3::new(0.0, move2 * 1.0, 0.0);
            },
            Some("common.abilities.sword.defensive_stalwart_sword") => {
                legacy_initialize();
                let pullback = 1.0 - move3base.powi(4);
                let move1 = move1base.powf(0.25) * pullback;
                let move2 = move2base.powi(2) * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation =
                    Quaternion::rotation_x(s_a.sc.3) * Quaternion::rotation_z(move2 * -0.5);

                next.foot_r.position += Vec3::new(move1 * 1.0, move1 * -2.0, 0.0);
                next.foot_r.orientation.rotate_z(move1 * -0.9);
                next.chest.orientation = Quaternion::rotation_z(move1 * -0.5);
                next.head.orientation = Quaternion::rotation_z(move1 * 0.3);
                next.shorts.orientation = Quaternion::rotation_z(move1 * 0.1);
                next.control.orientation.rotate_x(move1 * 0.4);
                next.control.orientation.rotate_z(move1 * 0.5);
                next.control.position += Vec3::new(0.0, 0.0, move1 * 4.0);

                next.control.position += Vec3::new(move2 * 8.0, 0.0, move2 * -1.0);
                next.control.orientation.rotate_x(move2 * -0.6);
                next.chest.position += Vec3::new(0.0, 0.0, move2 * -2.0);
                next.belt.position += Vec3::new(0.0, 0.0, move2 * 1.0);
                next.shorts.position += Vec3::new(0.0, 0.0, move2 * 1.0);
                next.shorts.orientation.rotate_x(move2 * 0.2);
                next.control.orientation.rotate_z(move2 * 0.4);
            },
            Some("common.abilities.sword.agile_dancing_edge") => {
                legacy_initialize();
                let pullback = 1.0 - move3base.powi(4);
                let move1 = move1base.powf(0.25) * pullback;
                let move2 = move2base.powi(2) * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3);

                next.head.orientation = Quaternion::rotation_x(move1 * 0.3);
                next.head.position += Vec3::new(0.0, 0.0, move1 * -1.0);
                next.control.position += Vec3::new(move1 * 8.0, move1 * 5.0, 0.0);

                next.head.orientation.rotate_x(move2 * 0.2);
                next.head.position += Vec3::new(0.0, 0.0, move2 * -1.0);
                next.control.position += Vec3::new(0.0, move2 * -2.0, move2 * 12.0);
                next.control.orientation.rotate_x(move2 * 1.1);
            },
            Some("common.abilities.sword.cleaving_blade_fever") => {
                legacy_initialize();
                let pullback = 1.0 - move3base.powi(4);
                let move1 = move1base.powf(0.25) * pullback;
                let move2 = move2base.powi(2) * pullback;

                next.hand_l.position = Vec3::new(s_a.shl.0, s_a.shl.1, s_a.shl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.shl.3) * Quaternion::rotation_y(s_a.shl.4);
                next.hand_r.position =
                    Vec3::new(-s_a.sc.0 + 6.0 + move1 * -12.0, -4.0 + move1 * 3.0, -2.0);
                next.hand_r.orientation = Quaternion::rotation_x(0.9 + move1 * 0.5);
                next.control.position = Vec3::new(s_a.sc.0, s_a.sc.1, s_a.sc.2);
                next.control.orientation = Quaternion::rotation_x(s_a.sc.3);

                next.foot_l.position += Vec3::new(move1 * 1.0, move1 * 2.0, 0.0);
                next.chest.orientation = Quaternion::rotation_z(move1 * -0.4);
                next.head.orientation = Quaternion::rotation_z(move1 * 0.2);
                next.shorts.orientation = Quaternion::rotation_z(move1 * 0.3);
                next.belt.orientation = Quaternion::rotation_z(move1 * 0.1);
                next.control.orientation.rotate_x(move1 * 0.4 + move2 * 0.6);
                next.control.orientation.rotate_z(move1 * 0.4);

                next.foot_r.position += Vec3::new(move2 * -1.0, move2 * -2.0, 0.0);
                next.control.position += Vec3::new(move2 * 5.0, move2 * 7.0, move2 * 5.0);
                next.chest.position += Vec3::new(0.0, 0.0, move2 * -1.0);
                next.shorts.orientation.rotate_x(move2 * 0.2);
                next.shorts.position += Vec3::new(0.0, move2 * 1.0, 0.0);
            },
            // ==================================
            //                AXE
            // ==================================
            Some("common.abilities.axe.basic_guard") => {
                let pullback = 1.0 - move3base.powi(4);
                let move1 = move1base.powf(0.25) * pullback;
                let move2 = (move2base * 10.0).sin();

                if d.velocity.xy().magnitude_squared() < 0.5_f32.powi(2) {
                    next.chest.position =
                        Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + move1 * -1.0 + move2 * 0.2);
                    next.chest.orientation = Quaternion::rotation_x(move1 * -0.15);
                    next.head.orientation = Quaternion::rotation_x(move1 * 0.25);

                    next.belt.position =
                        Vec3::new(0.0, s_a.belt.0 + move1 * 0.5, s_a.belt.1 + move1 * 0.5);
                    next.shorts.position =
                        Vec3::new(0.0, s_a.shorts.0 + move1 * 1.3, s_a.shorts.1 + move1 * 1.0);

                    next.belt.orientation = Quaternion::rotation_x(move1 * 0.15);
                    next.shorts.orientation = Quaternion::rotation_x(move1 * 0.25);

                    if !d.is_riding {
                        next.foot_l.position =
                            Vec3::new(-s_a.foot.0, s_a.foot.1 + move1 * 2.0, s_a.foot.2);
                        next.foot_l.orientation = Quaternion::rotation_z(move1 * -0.5);

                        next.foot_r.position =
                            Vec3::new(s_a.foot.0, s_a.foot.1 + move1 * -2.0, s_a.foot.2);
                        next.foot_r.orientation = Quaternion::rotation_x(move1 * -0.5);
                    }
                }

                match d.hands {
                    (Some(Hands::Two), _) => {
                        next.main.position = Vec3::new(0.0, 0.0, 0.0);
                        next.main.orientation = Quaternion::rotation_x(0.0);

                        next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                        next.hand_l.orientation =
                            Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                        next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                        next.hand_r.orientation =
                            Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                        next.control.position = Vec3::new(
                            s_a.ac.0 + 0.5 + move1 * 13.0,
                            s_a.ac.1 + 9.0 + move1 * -3.0,
                            s_a.ac.2 + 2.5 + move1 * 8.0,
                        );
                        next.control.orientation =
                            Quaternion::rotation_x(s_a.ac.3 - 2.25 + move1 * -2.0)
                                * Quaternion::rotation_y(s_a.ac.4 - PI + move1 * -1.8)
                                * Quaternion::rotation_z(s_a.ac.5 - 0.2 + move1 * 4.0);
                    },
                    (Some(Hands::One), offhand) => {
                        next.main.position = Vec3::new(0.0, 0.0, 0.0);
                        next.main.orientation = Quaternion::rotation_x(0.0);

                        next.control_l.position =
                            Vec3::new(-7.0, 8.0 + move1 * 3.0, 2.0 + move1 * 3.0);
                        next.control_l.orientation =
                            Quaternion::rotation_x(-0.3) * Quaternion::rotation_y(move1 * 1.0);
                        next.hand_l.position = Vec3::new(0.0, -0.5, 0.0);
                        next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0);
                        if offhand.is_some() {
                            next.second.position = Vec3::new(0.0, 0.0, 0.0);
                            next.second.orientation = Quaternion::rotation_x(0.0);
                            next.control_r.position =
                                Vec3::new(7.0, 8.0 + move1 * 3.0, 2.0 + move1 * 3.0);
                            next.control_r.orientation =
                                Quaternion::rotation_x(-0.3) * Quaternion::rotation_y(move1 * -1.0);
                            next.hand_r.position = Vec3::new(0.0, -0.5, 0.0);
                            next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
                        } else {
                            next.hand_r.position = Vec3::new(4.5, 8.0, 5.0);
                            next.hand_r.orientation =
                                Quaternion::rotation_x(1.9) * Quaternion::rotation_y(0.5)
                        }
                    },
                    (_, _) => {},
                }
            },
            Some("common.abilities.axe.cleave") => {
                legacy_initialize();
                let move1 = chargebase.min(1.0) * pullback;
                let move2 = move2base.powi(2) * pullback;
                let tension = (chargebase * 20.0).sin();

                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(
                    s_a.ac.0 + 0.5 + move1 * 7.0,
                    s_a.ac.1 + 9.0 + move1 * -4.0,
                    s_a.ac.2 + 2.5 + move1 * 18.0 + tension / 5.0,
                );
                next.control.orientation =
                    Quaternion::rotation_x(s_a.ac.3 - 2.25 + move1 * -1.0 + tension / 30.0)
                        * Quaternion::rotation_y(s_a.ac.4 - PI)
                        * Quaternion::rotation_z(s_a.ac.5 - 0.2 - move1 * PI);

                next.control.orientation.rotate_x(move2 * -3.0);
                next.control.position += Vec3::new(0.0, move2 * 8.0, move2 * -30.0);
            },
            Some("common.abilities.axe.execute") => {
                legacy_initialize();
                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0 + 0.5, s_a.ac.1 + 9.0, s_a.ac.2 + 2.5);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3 - 2.25)
                    * Quaternion::rotation_y(s_a.ac.4 - PI)
                    * Quaternion::rotation_z(s_a.ac.5 - 0.2 - move1 * PI);

                next.control.orientation.rotate_x(move1 * 0.9);
                next.chest.orientation.rotate_z(move1 * 1.2);
                next.head.orientation.rotate_z(move1 * -0.5);
                next.belt.orientation.rotate_z(move1 * -0.3);
                next.shorts.orientation.rotate_z(move1 * -0.7);
                next.control.position += Vec3::new(move1 * 4.0, move1 * -12.0, move1 * 11.0);

                next.chest.orientation.rotate_z(move2 * -2.0);
                next.head.orientation.rotate_z(move2 * 0.9);
                next.belt.orientation.rotate_z(move2 * 0.4);
                next.shorts.orientation.rotate_z(move2 * 1.1);
                next.control.orientation.rotate_x(move2 * -5.0);
                next.control.position += Vec3::new(move2 * -3.0, move2 * 12.0, move2 * -17.0);
                next.control.orientation.rotate_z(move2 * 0.7);
            },
            Some("common.abilities.axe.maelstrom") => {
                legacy_initialize();
                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0 + 0.5, s_a.ac.1 + 9.0, s_a.ac.2 + 2.5);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3 - 2.25)
                    * Quaternion::rotation_y(s_a.ac.4 - PI)
                    * Quaternion::rotation_z(s_a.ac.5 - 0.2 - move1 * PI);

                next.control.orientation.rotate_x(move1 * 0.9);
                next.chest.orientation.rotate_z(move1 * 1.2);
                next.head.orientation.rotate_z(move1 * -0.5);
                next.belt.orientation.rotate_z(move1 * -0.3);
                next.shorts.orientation.rotate_z(move1 * -0.7);
                next.control.position += Vec3::new(move1 * 4.0, move1 * -12.0, move1 * 11.0);

                next.chest.orientation.rotate_z(move2 * -2.0);
                next.head.orientation.rotate_z(move2 * 0.9);
                next.belt.orientation.rotate_z(move2 * 0.4);
                next.shorts.orientation.rotate_z(move2 * 1.1);
                next.control.orientation.rotate_x(move2 * -5.0);
                next.control.position += Vec3::new(move2 * 5.0, move2 * 12.0, move2 * -17.0);
                next.control.orientation.rotate_y(move2 * -2.0);
                next.control.orientation.rotate_z(move2 * -1.0);
                next.torso.orientation.rotate_z(move2base * -4.0 * PI);
            },
            Some("common.abilities.axe.lacerate") => {
                legacy_initialize();
                let move2_reset = ((move2base - 0.5).abs() - 0.5).abs() * 2.0;

                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2 + 10.0);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0 + 0.5, s_a.ac.1 + 9.0, s_a.ac.2 + 2.5);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3 - 2.25)
                    * Quaternion::rotation_y(s_a.ac.4 - PI)
                    * Quaternion::rotation_z(s_a.ac.5 - 0.2 - move1 * PI * 0.75);

                next.chest.orientation.rotate_z(move1 * 1.2);
                next.head.orientation.rotate_z(move1 * -0.7);
                next.shorts.orientation.rotate_z(move1 * -0.9);
                next.belt.orientation.rotate_z(move1 * -0.3);

                next.chest.orientation.rotate_z(move2 * -2.9);
                next.head.orientation.rotate_z(move2 * 1.2);
                next.shorts.orientation.rotate_z(move2 * 2.0);
                next.belt.orientation.rotate_z(move2 * 0.7);
                next.control.orientation.rotate_x(move2_reset * -1.0);
                next.control.orientation.rotate_z(move2 * -5.0);
                next.control.position += Vec3::new(move2 * 17.0, move2 * 3.0, 0.0);
            },
            Some("common.abilities.axe.riptide") => {
                legacy_initialize();
                let move2_reset = ((move2base - 0.5).abs() - 0.5).abs() * 2.0;

                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0 + 0.5, s_a.ac.1 + 9.0, s_a.ac.2 + 2.5);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3 - 2.25)
                    * Quaternion::rotation_y(s_a.ac.4 - PI)
                    * Quaternion::rotation_z(s_a.ac.5 - 0.2 - move1 * PI * 0.75);

                next.chest.orientation.rotate_z(move1 * 1.2);
                next.head.orientation.rotate_z(move1 * -0.7);
                next.shorts.orientation.rotate_z(move1 * -0.9);
                next.belt.orientation.rotate_z(move1 * -0.3);

                next.chest.orientation.rotate_z(move2 * -2.9);
                next.head.orientation.rotate_z(move2 * 1.2);
                next.shorts.orientation.rotate_z(move2 * 2.0);
                next.belt.orientation.rotate_z(move2 * 0.7);
                next.control.orientation.rotate_x(move2_reset * -1.0);
                next.control.orientation.rotate_z(move2 * -5.0);
                next.control.position += Vec3::new(move2 * 17.0, move2 * 3.0, 0.0);
                next.torso.orientation.rotate_z(move2base * -TAU)
            },
            Some("common.abilities.axe.keelhaul") => {
                legacy_initialize();
                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0 + 0.5, s_a.ac.1 + 9.0, s_a.ac.2 + 2.5);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3 - 2.25)
                    * Quaternion::rotation_y(s_a.ac.4 - PI)
                    * Quaternion::rotation_z(s_a.ac.5 - 0.2);

                next.control.orientation.rotate_z(move1 * -3.3);
                next.control.orientation.rotate_x(move1 * 0.8);
                next.control.position +=
                    Vec3::new(move1 * 4.0, move1 * 4.0 - move2 * 6.0, move1 * 10.0);

                next.chest.orientation.rotate_z(move2 * 1.2);
                next.head.orientation.rotate_z(move2 * -0.5);
                next.belt.orientation.rotate_z(move2 * -0.3);
                next.shorts.orientation.rotate_z(move2 * -0.9);
                next.control.orientation.rotate_z(move2 * -1.2);
            },
            Some("common.abilities.axe.bulkhead") => {
                legacy_initialize();
                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0 + 0.5, s_a.ac.1 + 9.0, s_a.ac.2 + 2.5);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3 - 2.25)
                    * Quaternion::rotation_y(s_a.ac.4 - PI)
                    * Quaternion::rotation_z(
                        s_a.ac.5 - 0.2 + move1 * -PI * 0.75 + move2 * PI * 0.25,
                    );

                next.chest.orientation.rotate_z(move1 * 1.8);
                next.head.orientation.rotate_z(move1 * -0.6);
                next.belt.orientation.rotate_z(move1 * -0.4);
                next.shorts.orientation.rotate_z(move1 * -1.3);
                next.control.orientation.rotate_x(move1 * -0.8);

                next.chest.orientation.rotate_z(move2 * -3.8);
                next.head.orientation.rotate_z(move2 * 1.2);
                next.belt.orientation.rotate_z(move2 * 0.8);
                next.shorts.orientation.rotate_z(move2 * 2.1);
                next.control.orientation.rotate_x(move2 * 0.6);
                next.control.orientation.rotate_z(move2 * -4.0);
                next.control.position += Vec3::new(move2 * 12.0, move2 * -6.0, 0.0);
            },
            Some("common.abilities.axe.capsize") => {
                legacy_initialize();
                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0 + 0.5, s_a.ac.1 + 9.0, s_a.ac.2 + 2.5);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3 - 2.25)
                    * Quaternion::rotation_y(s_a.ac.4 - PI)
                    * Quaternion::rotation_z(
                        s_a.ac.5 - 0.2 + move1 * -PI * 0.75 + move2 * PI * 0.25,
                    );

                next.chest.orientation.rotate_z(move1 * 1.8);
                next.head.orientation.rotate_z(move1 * -0.6);
                next.belt.orientation.rotate_z(move1 * -0.4);
                next.shorts.orientation.rotate_z(move1 * -1.3);
                next.control.orientation.rotate_x(move1 * -0.8);

                next.chest.orientation.rotate_z(move2 * -3.8);
                next.head.orientation.rotate_z(move2 * 1.2);
                next.belt.orientation.rotate_z(move2 * 0.8);
                next.shorts.orientation.rotate_z(move2 * 2.1);
                next.control.orientation.rotate_x(move2 * 0.6);
                next.control.orientation.rotate_z(move2 * -4.0);
                next.control.position += Vec3::new(move2 * 12.0, move2 * -6.0, 0.0);
                next.torso.orientation.rotate_z(move2base * -TAU);
            },
            Some("common.abilities.axe.fracture") => {
                legacy_initialize();
                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0 + 0.5, s_a.ac.1 + 9.0, s_a.ac.2 + 2.5);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3 - 2.25)
                    * Quaternion::rotation_y(s_a.ac.4 - PI)
                    * Quaternion::rotation_z(s_a.ac.5 - 0.2 + move1 * -PI / 2.0 + move2 * -0.5);

                next.control.orientation.rotate_x(move1 * 0.0);
                next.chest.orientation.rotate_x(move1 * -0.5);
                next.chest.orientation.rotate_z(move1 * 0.7);
                next.head.orientation.rotate_z(move1 * -0.3);
                next.belt.orientation.rotate_z(move1 * -0.1);
                next.shorts.orientation.rotate_z(move1 * -0.4);

                next.chest.orientation.rotate_z(move2 * -1.8);
                next.head.orientation.rotate_z(move2 * 0.9);
                next.shorts.orientation.rotate_z(move2 * 1.3);
                next.belt.orientation.rotate_z(move2 * 0.6);
                next.control.orientation.rotate_x(move2 * -0.9);
                next.control.orientation.rotate_z(move2 * -3.5);
                next.control.position += Vec3::new(move2 * 14.0, move2 * 6.0, 0.0);
            },
            Some("common.abilities.axe.berserk") => {
                legacy_initialize();
                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0 + 0.5, s_a.ac.1 + 9.0, s_a.ac.2 + 2.5);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3 - 2.25)
                    * Quaternion::rotation_y(s_a.ac.4 - PI)
                    * Quaternion::rotation_z(s_a.ac.5 - 0.2);

                next.control.orientation.rotate_z(move1 * -2.0);
                next.control.orientation.rotate_x(move1 * 3.5);
                next.control.position += Vec3::new(move1 * 14.0, move1 * -6.0, move1 * 15.0);

                next.head.orientation.rotate_x(move2 * 0.6);
                next.chest.orientation.rotate_x(move2 * 0.4);
            },
            Some("common.abilities.axe.savage_sense") => {
                legacy_initialize();
                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0 + 0.5, s_a.ac.1 + 9.0, s_a.ac.2 + 2.5);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3 - 2.25)
                    * Quaternion::rotation_y(s_a.ac.4 - PI)
                    * Quaternion::rotation_z(s_a.ac.5 - 0.2);

                next.chest.orientation = Quaternion::rotation_z(move1 * 0.6);
                next.head.orientation = Quaternion::rotation_z(move1 * -0.2);
                next.belt.orientation = Quaternion::rotation_z(move1 * -0.3);
                next.shorts.orientation = Quaternion::rotation_z(move1 * -0.1);
                next.foot_r.position += Vec3::new(0.0, move1 * 4.0, move1 * 4.0);
                next.foot_r.orientation.rotate_x(move1 * 1.2);

                next.foot_r.position += Vec3::new(0.0, move2 * 4.0, move2 * -4.0);
                next.foot_r.orientation.rotate_x(move2 * -1.2);
            },
            Some("common.abilities.axe.adrenaline_rush") => {
                legacy_initialize();
                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0 + 0.5, s_a.ac.1 + 9.0, s_a.ac.2 + 2.5);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3 - 2.25)
                    * Quaternion::rotation_y(s_a.ac.4 - PI)
                    * Quaternion::rotation_z(s_a.ac.5 - 0.2 - move1 * PI);

                next.control.orientation.rotate_z(move1 * -1.8);
                next.control.orientation.rotate_y(move1 * 1.5);
                next.control.position += Vec3::new(move1 * 11.0, 0.0, 0.0);

                next.control.orientation.rotate_y(move2 * 0.7);
                next.control.orientation.rotate_z(move2 * 1.6);
                next.control.position += Vec3::new(move2 * -8.0, 0.0, move2 * -3.0);
            },
            Some("common.abilities.axe.bloodfeast") => {
                legacy_initialize();
                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0 + 0.5, s_a.ac.1 + 9.0, s_a.ac.2 + 2.5);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3 - 2.25)
                    * Quaternion::rotation_y(s_a.ac.4 - PI)
                    * Quaternion::rotation_z(s_a.ac.5 - 0.2);

                next.control.orientation.rotate_z(move1 * -3.4);
                next.control.orientation.rotate_x(move1 * 1.1);
                next.control.position += Vec3::new(move1 * 14.0, move1 * -3.0, 0.0);

                next.control.orientation.rotate_x(move2 * 1.7);
                next.control.orientation.rotate_z(move2 * -1.3);
                next.control.orientation.rotate_y(move2 * 0.8);
            },
            Some("common.abilities.axe.furor") => {
                legacy_initialize();
                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0 + 0.5, s_a.ac.1 + 9.0, s_a.ac.2 + 2.5);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3 - 2.25)
                    * Quaternion::rotation_y(s_a.ac.4 - PI)
                    * Quaternion::rotation_z(s_a.ac.5 - 0.2 - move1 * PI);

                next.control.orientation.rotate_x(move1 * -1.0);
                next.control.position += Vec3::new(move1 * 3.0, move1 * -2.0, move1 * 14.0);
                next.control.orientation.rotate_z(move1 * 1.5);

                next.control.orientation.rotate_y(move2 * -1.0);
                next.control.orientation.rotate_z(move2 * -1.6);
                next.control.orientation.rotate_y(move2 * 0.7);
                next.control.orientation.rotate_x(move2 * -0.5);
                next.control.position += Vec3::new(move2 * 9.0, move2 * -3.0, move2 * -14.0);
            },
            Some("common.abilities.axe.sunder") => {
                legacy_initialize();
                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0 + 0.5, s_a.ac.1 + 9.0, s_a.ac.2 + 2.5);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3 - 2.25)
                    * Quaternion::rotation_y(s_a.ac.4 - PI)
                    * Quaternion::rotation_z(s_a.ac.5 - 0.2);

                next.control.orientation.rotate_z(move1 * -1.5);
                next.control.position += Vec3::new(move1 * 12.0, 0.0, move1 * 5.0);
                next.control.orientation.rotate_y(move1 * 0.5);
                next.main.position += Vec3::new(0.0, move1 * 10.0, 0.0);
                next.main.orientation.rotate_z(move1base * TAU);
                next.second.position += Vec3::new(0.0, move1 * 10.0, 0.0);
                next.second.orientation.rotate_z(move1base * -TAU);

                next.main.orientation.rotate_z(move2base * TAU);
                next.main.position += Vec3::new(0.0, move2 * -10.0, 0.0);
                next.second.orientation.rotate_z(move2base * -TAU);
                next.second.position += Vec3::new(0.0, move2 * -10.0, 0.0);
                next.control.position += Vec3::new(0.0, 0.0, move2 * -5.0);
            },
            Some("common.abilities.axe.defiance") => {
                legacy_initialize();
                let tension = (move2base * 20.0).sin();

                next.hand_l.position = Vec3::new(s_a.ahl.0, s_a.ahl.1, s_a.ahl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.ahl.3) * Quaternion::rotation_y(s_a.ahl.4);
                next.hand_r.position = Vec3::new(s_a.ahr.0, s_a.ahr.1, s_a.ahr.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.ahr.3) * Quaternion::rotation_z(s_a.ahr.5);

                next.control.position = Vec3::new(s_a.ac.0 + 0.5, s_a.ac.1 + 9.0, s_a.ac.2 + 2.5);
                next.control.orientation = Quaternion::rotation_x(s_a.ac.3 - 2.25)
                    * Quaternion::rotation_y(s_a.ac.4 - PI)
                    * Quaternion::rotation_z(s_a.ac.5 - 0.2);

                next.control.orientation.rotate_z(move1 * -1.6);
                next.control.orientation.rotate_x(move1 * 1.7);
                next.control.position += Vec3::new(move1 * 12.0, move1 * -10.0, move1 * 18.0);
                next.head.orientation.rotate_x(move1 * 0.6);
                next.head.position += Vec3::new(0.0, 0.0, move1 * -3.0);
                next.control.orientation.rotate_z(move1 * 0.4);

                next.head.orientation.rotate_x(tension * 0.3);
                next.control.position += Vec3::new(0.0, 0.0, tension * 2.0);
            },
            // ==================================
            //               HAMMER
            // ==================================
            Some("common.abilities.hammer.basic_guard") => {
                let pullback = 1.0 - move3base.powi(4);
                let move1 = move1base.powf(0.25) * pullback;
                let move2 = (move2base * 10.0).sin();

                if d.velocity.xy().magnitude_squared() < 0.5_f32.powi(2) {
                    next.chest.position =
                        Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + move1 * -1.0 + move2 * 0.2);
                    next.chest.orientation = Quaternion::rotation_x(move1 * -0.15);
                    next.head.orientation = Quaternion::rotation_x(move1 * 0.25);

                    next.belt.position =
                        Vec3::new(0.0, s_a.belt.0 + move1 * 0.5, s_a.belt.1 + move1 * 0.5);
                    next.shorts.position =
                        Vec3::new(0.0, s_a.shorts.0 + move1 * 1.3, s_a.shorts.1 + move1 * 1.0);

                    next.belt.orientation = Quaternion::rotation_x(move1 * 0.15);
                    next.shorts.orientation = Quaternion::rotation_x(move1 * 0.25);

                    if !d.is_riding {
                        next.foot_l.position =
                            Vec3::new(-s_a.foot.0, s_a.foot.1 + move1 * 2.0, s_a.foot.2);
                        next.foot_l.orientation = Quaternion::rotation_z(move1 * -0.5);

                        next.foot_r.position =
                            Vec3::new(s_a.foot.0, s_a.foot.1 + move1 * -2.0, s_a.foot.2);
                        next.foot_r.orientation = Quaternion::rotation_x(move1 * -0.5);
                    }
                }

                match d.hands {
                    (Some(Hands::Two), _) => {
                        next.hand_l.position = Vec3::new(s_a.hhl.0, s_a.hhl.1, s_a.hhl.2);
                        next.hand_l.orientation = Quaternion::rotation_x(s_a.hhl.3)
                            * Quaternion::rotation_y(s_a.hhl.4)
                            * Quaternion::rotation_z(s_a.hhl.5);
                        next.hand_r.position = Vec3::new(s_a.hhr.0, s_a.hhr.1, s_a.hhr.2);
                        next.hand_r.orientation = Quaternion::rotation_x(s_a.hhr.3)
                            * Quaternion::rotation_y(s_a.hhr.4)
                            * Quaternion::rotation_z(s_a.hhr.5);

                        next.main.position = Vec3::new(0.0, 0.0, 0.0);
                        next.main.orientation = Quaternion::rotation_x(0.0);
                        next.control.position = Vec3::new(
                            s_a.hc.0 + move1 * 3.0,
                            s_a.hc.1 + move1 * 3.0,
                            s_a.hc.2 + move1 * 10.0,
                        );
                        next.control.orientation = Quaternion::rotation_x(s_a.hc.3)
                            * Quaternion::rotation_y(s_a.hc.4)
                            * Quaternion::rotation_z(s_a.hc.5 + move1 * -1.0);
                    },
                    (Some(Hands::One), offhand) => {
                        next.control_l.position =
                            Vec3::new(-7.0, 8.0 + move1 * 3.0, 2.0 + move1 * 3.0);
                        next.control_l.orientation =
                            Quaternion::rotation_x(-0.3) * Quaternion::rotation_y(move1 * 1.0);
                        next.hand_l.position = Vec3::new(0.0, -0.5, 0.0);
                        next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0);

                        next.main.position = Vec3::new(0.0, 0.0, 0.0);
                        next.main.orientation = Quaternion::rotation_x(0.0);

                        if offhand.is_some() {
                            next.control_r.position =
                                Vec3::new(7.0, 8.0 + move1 * 3.0, 2.0 + move1 * 3.0);
                            next.control_r.orientation =
                                Quaternion::rotation_x(-0.3) * Quaternion::rotation_y(move1 * -1.0);
                            next.hand_r.position = Vec3::new(0.0, -0.5, 0.0);
                            next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0);
                            next.second.position = Vec3::new(0.0, 0.0, 0.0);
                            next.second.orientation = Quaternion::rotation_x(0.0);
                        } else {
                            next.hand_r.position = Vec3::new(4.5, 8.0, 5.0);
                            next.hand_r.orientation =
                                Quaternion::rotation_x(1.9) * Quaternion::rotation_y(0.5)
                        }
                    },
                    (_, _) => {},
                }
            },
            Some("common.abilities.hammer.solid_smash") => {
                hammer_start(&mut next, s_a);

                next.control.orientation.rotate_x(move1 * 2.7);
                next.control.orientation.rotate_z(move1 * 1.4);
                next.control.position += Vec3::new(-12.0, 0.0, 0.0) * move1;
                next.control.orientation.rotate_x(move1 * -1.2);
                twist_back(&mut next, move1, 0.8, 0.3, 0.1, 0.5);

                twist_forward(&mut next, move2, 1.4, 0.5, 0.3, 1.0);
                next.control.orientation.rotate_x(move2 * -1.9);
                next.control.orientation.rotate_z(move2 * 0.6);
            },
            Some("common.abilities.hammer.scornful_swipe") => {
                hammer_start(&mut next, s_a);
                let move1_pre = move1.min(0.5) * 2.0;
                let move1_shake = ((move1.max(0.3) - 0.3) * 15.0).sin();
                let move1_late = move1.powi(4);

                next.control.orientation.rotate_x(move1_pre * 2.3);
                next.control.position += Vec3::new(0.0, 2.0, 16.0) * move1_pre;
                next.control.position += Vec3::new(0.0, 0.0, 4.0) * move1_shake;
                next.control.orientation.rotate_y(move1_late * 1.6);
                next.control.position += Vec3::new(-8.0, 0.0, -8.0) * move1_late;
                twist_back(&mut next, move1_late, 1.0, 0.4, 0.2, 0.7);
                next.control.orientation.rotate_z(move1_late * 1.2);

                twist_forward(&mut next, move2, 1.9, 0.9, 0.6, 1.1);
                next.control.orientation.rotate_y(move2 * -1.7);
                next.control.orientation.rotate_z(move2 * -2.7);
            },
            Some("common.abilities.hammer.heavy_whorl") => {
                hammer_start(&mut next, s_a);

                twist_back(&mut next, move1, 2.0, 0.8, 0.4, 1.4);
                next.control.orientation.rotate_x(move1 * 0.6);

                next.torso.orientation.rotate_z(move2base * -2.0 * PI);
                twist_forward(&mut next, move2, 3.4, 1.2, 0.8, 1.8);
                next.control.orientation.rotate_z(move2 * -2.3);
                next.control.position += Vec3::new(6.0, 0.0, 6.0) * move2;
            },
            Some("common.abilities.hammer.dual_heavy_whorl") => {
                dual_wield_start(&mut next);

                twist_back(&mut next, move1, 2.0, 0.8, 0.4, 1.4);
                next.control_l.orientation.rotate_y(move1 * -PI / 2.0);
                next.control_r.orientation.rotate_y(move1 * -PI / 2.0);
                next.control.position += Vec3::new(0.0, 0.0, 4.0) * move1;

                next.torso.orientation.rotate_z(move2base * -2.0 * PI);
                twist_forward(&mut next, move2, 3.4, 1.2, 0.8, 1.8);
                next.control_l.orientation.rotate_z(move2 * -2.3);
                next.control_r.orientation.rotate_z(move2 * -2.3);
            },
            Some("common.abilities.hammer.breach") => {
                hammer_start(&mut next, s_a);

                next.control.orientation.rotate_x(move1 * 2.5);
                next.control.orientation.rotate_z(move1 * -4.8);
                next.control.position += Vec3::new(-12.0, 0.0, 22.0) * move1;
                twist_back(&mut next, move1, 0.6, 0.2, 0.0, 0.3);

                twist_forward(&mut next, move2, 1.6, 0.4, 0.2, 0.7);
                next.control.orientation.rotate_x(move2 * -4.5);
                next.control.position += Vec3::new(0.0, 0.0, -20.0) * move2;
            },
            Some("common.abilities.hammer.pile_driver") => {
                hammer_start(&mut next, s_a);
                let shake = (move1base * 15.0).sin();
                let move1 = (move1base * 2.0).min(1.0) * pullback;

                twist_back(&mut next, move1, 0.9, 0.3, 0.1, 0.5);
                next.control.orientation.rotate_x(move1 * 2.4);
                next.control.position += Vec3::new(-14.0, 0.0, 14.0) * move1;
                next.control.orientation.rotate_z(move1 * 1.8);

                next.control.orientation.rotate_x(shake * 0.15);

                twist_forward(&mut next, move2, 1.6, 0.5, 0.2, 0.9);
                next.control.orientation.rotate_x(move2 * -4.0);
                next.control.orientation.rotate_z(move2 * 0.4);
                next.control.position += Vec3::new(0.0, 0.0, -12.0) * move2;
            },
            Some("common.abilities.hammer.upheaval") => {
                hammer_start(&mut next, s_a);
                let move1_twist = move1 * (move1 * PI * 1.5).sin();

                twist_forward(&mut next, move1_twist, 0.8, 0.3, 0.0, 0.4);
                let angle1 = 5.0;
                let angle2 = 4.0;
                next.control
                    .orientation
                    .rotate_x(move1base * (2.0 - angle1) + move2base * (2.0 - angle2));
                next.control.orientation.rotate_y(move1 * -0.8);
                next.control
                    .orientation
                    .rotate_x(move1base * angle1 + move2base * angle2);
                next.control.orientation.rotate_z(move1 * 1.0);
                next.control.orientation.rotate_x(move2 * 6.0);
                next.control.orientation.rotate_z(move2 * -0.6);
                next.control.position += Vec3::new(-16.0, 0.0, 0.0) * move1;
                next.control.position += Vec3::new(12.0, 0.0, 10.0) * move2;
                twist_forward(&mut next, move2, 1.0, 0.9, 0.4, 1.1);
            },
            Some("common.abilities.hammer.dual_upheaval") => {
                dual_wield_start(&mut next);
                let move1_return = (3.0 * move1base).sin();

                next.control.orientation.rotate_x(4.0 * move1base);
                next.control_l.orientation.rotate_z(move1 * 0.6);
                next.control_r.orientation.rotate_z(move1 * -0.6);
                next.control.position += Vec3::new(0.0, 6.0, 8.0) * move1_return;
                next.control.orientation.rotate_x(3.5 * move2base);
                next.control_l.orientation.rotate_z(move2 * -1.4);
                next.control_r.orientation.rotate_z(move2 * 1.4);
                next.control.position += Vec3::new(0.0, 12.0, 10.0) * move2;
            },
            Some("common.abilities.hammer.wide_wallop") => {
                hammer_start(&mut next, s_a);
                let move1 = chargebase.min(1.0) * pullback;
                let tension = (chargebase * 7.0).sin();

                next.control.orientation.rotate_x(move1 * 1.1 + move2 * 0.6);
                twist_back(&mut next, move1 + tension / 25.0, 1.7, 0.7, 0.3, 1.1);
                next.control.orientation.rotate_y(move1 * -0.8);
                next.control.position += Vec3::new(0.0, 0.0, 6.0) * move1;

                twist_forward(&mut next, move2, 4.8, 1.7, 0.7, 3.2);
                next.control.orientation.rotate_y(move2 * 2.0);
                next.control.orientation.rotate_z(move2 * -1.8);
            },
            Some("common.abilities.hammer.intercept") => {
                hammer_start(&mut next, s_a);
                twist_back(&mut next, move1, 1.6, 0.7, 0.3, 1.1);
                next.control.orientation.rotate_x(move1 * 1.8);

                twist_forward(&mut next, move2, 2.4, 0.9, 0.5, 1.4);
                next.control.orientation.rotate_z(move2 * -2.7);
                next.control.orientation.rotate_x(move2 * 2.0);
                next.control.position += Vec3::new(5.0, 0.0, 11.0) * move2;
            },
            Some("common.abilities.hammer.dual_intercept") => {
                dual_wield_start(&mut next);
                next.control_l.orientation.rotate_x(move1 * -1.4);
                next.control_l.orientation.rotate_z(move1 * 0.8);
                next.control_r.orientation.rotate_x(move1 * -1.4);
                next.control_r.orientation.rotate_z(move1 * -0.8);
                next.control.position += Vec3::new(0.0, 0.0, -6.0) * move1;

                next.control_l.orientation.rotate_z(move2 * -2.6);
                next.control_l.orientation.rotate_x(move2 * 4.0);
                next.control_r.orientation.rotate_z(move2 * 2.6);
                next.control_r.orientation.rotate_x(move2 * 4.0);
                next.control.position += Vec3::new(0.0, 0.0, 20.0) * move2;
            },
            Some("common.abilities.hammer.spine_cracker") => {
                hammer_start(&mut next, s_a);

                twist_back(&mut next, move1, 1.9, 1.5, 0.5, 1.2);
                next.head.position += Vec3::new(-2.0, 2.0, 0.0) * move1;
                next.control.orientation.rotate_x(move1 * 1.8);
                next.control.position += Vec3::new(0.0, 0.0, 8.0) * move1;
                next.control.orientation.rotate_y(move1 * 0.4);

                twist_forward(&mut next, move2, 2.1, 1.6, 0.4, 1.3);
                next.control.orientation.rotate_z(move2 * 1.6);
                next.control.position += Vec3::new(-16.0, 12.0, -8.0) * move2;
            },
            Some("common.abilities.hammer.lung_pummel") => {
                hammer_start(&mut next, s_a);

                twist_back(&mut next, move1, 1.9, 0.7, 0.3, 1.2);
                next.control.orientation.rotate_x(move1 * 1.2);
                next.control.orientation.rotate_z(move1 * 1.0);
                next.control.position += Vec3::new(-12.0, 0.0, 0.0) * move1;

                twist_forward(&mut next, move2, 3.4, 1.4, 0.9, 2.1);
                next.control.orientation.rotate_z(move2 * -4.0);
                next.control.position += Vec3::new(12.0, 0.0, 14.0) * move2;
            },
            Some("common.abilities.hammer.helm_crusher") => {
                hammer_start(&mut next, s_a);

                twist_back(&mut next, move1, 0.8, 0.3, 0.1, 0.5);
                next.control.orientation.rotate_x(move1 * -0.8);
                next.control.orientation.rotate_z(move1 * -1.6);
                next.control.orientation.rotate_x(move1 * 2.8);
                next.control.position += Vec3::new(-9.0, 0.0, 8.0) * move1;
                next.control.orientation.rotate_z(move1 * -0.4);

                twist_forward(&mut next, move2, 1.8, 0.7, 0.4, 1.1);
                next.control.orientation.rotate_x(move2 * -5.0);
                next.control.orientation.rotate_z(move2 * -1.0);
                next.control.position += Vec3::new(-12.0, 0.0, -8.0) * move2;
            },
            Some("common.abilities.hammer.thunderclap") => {
                hammer_start(&mut next, s_a);

                twist_back(&mut next, move1, 1.8, 0.9, 0.5, 1.1);
                next.control.orientation.rotate_x(move1 * 2.4);
                next.control.position += Vec3::new(-16.0, -8.0, 12.0) * move1;
                next.control.orientation.rotate_z(move1 * PI / 2.0);
                next.control.orientation.rotate_x(move1 * 0.6);

                twist_forward(&mut next, move2, 2.4, 1.1, 0.6, 1.4);
                next.control.orientation.rotate_x(move2 * -5.0);
                next.control.position += Vec3::new(4.0, 12.0, -12.0) * move2;
                next.control.orientation.rotate_z(move2 * 0.6);
            },
            Some("common.abilities.hammer.earthshaker") => {
                hammer_start(&mut next, s_a);

                next.hand_l.orientation.rotate_y(move1 * -PI);
                next.hand_r.orientation.rotate_y(move1 * -PI);
                next.control.orientation.rotate_x(2.4 * move1);
                next.control.orientation.rotate_z(move1 * -PI / 2.0);
                next.control.orientation.rotate_x(-0.6 * move1);
                next.control.position += Vec3::new(-8.0, 0.0, 24.0) * move1;
                next.chest.orientation.rotate_x(move1 * 0.5);
                next.torso.position += Vec3::new(0.0, 0.0, 8.0) * move1;

                next.torso.position += Vec3::new(0.0, 0.0, -8.0) * move2;
                next.control.orientation.rotate_x(move2 * -0.8);
                next.control.position += Vec3::new(0.0, 0.0, -10.0) * move2;
                next.chest.orientation.rotate_x(move2 * -0.8);
            },
            Some("common.abilities.hammer.judgement") => {
                hammer_start(&mut next, s_a);

                next.control.orientation.rotate_x(2.4 * move1);
                next.control.orientation.rotate_z(move1 * PI / 2.0);
                next.control.orientation.rotate_x(-0.6 * move1);
                next.control.position += Vec3::new(-8.0, 6.0, 24.0) * move1;
                next.chest.orientation.rotate_x(move1 * 0.5);
                next.torso.position += Vec3::new(0.0, 0.0, 8.0) * move1;

                next.torso.position += Vec3::new(0.0, 0.0, -8.0) * move2;
                next.chest.orientation.rotate_x(-1.5 * move2);
                next.belt.orientation.rotate_x(0.3 * move2);
                next.shorts.orientation.rotate_x(0.6 * move2);
                next.control.orientation.rotate_x(-3.0 * move2);
                next.control.position += Vec3::new(0.0, 0.0, -16.0) * move2;
            },
            Some("common.abilities.hammer.retaliate") => {
                hammer_start(&mut next, s_a);

                twist_back(&mut next, move1, 0.6, 0.2, 0.0, 0.3);
                next.control.orientation.rotate_x(move1 * 1.5);
                next.control.orientation.rotate_y(move1 * 0.4);
                next.control.position += Vec3::new(0.0, 0.0, 16.0) * move1;

                twist_forward(&mut next, move2, 2.1, 0.6, 0.4, 0.9);
                next.control.orientation.rotate_y(move2 * 2.0);
                next.control.orientation.rotate_x(move2 * -2.5);
                next.control.orientation.rotate_z(move2 * -0.6);
                next.control.position += Vec3::new(6.0, -10.0, -14.0) * move2;
            },
            Some("common.abilities.hammer.tenacity") => {
                hammer_start(&mut next, s_a);

                next.control.orientation.rotate_x(move1 * 0.6);
                next.control.orientation.rotate_y(move1 * 0.9);
                next.control.orientation.rotate_x(move1 * -0.6);
                next.chest.orientation.rotate_x(move1 * 0.4);
                next.control.position += Vec3::new(0.0, 4.0, 3.0) * move1;

                next.control.position += Vec3::new(
                    (move2 * 50.0).sin(),
                    (move2 * 67.0).sin(),
                    (move2 * 83.0).sin(),
                );
            },
            Some("common.abilities.hammer.tremor") => {
                hammer_start(&mut next, s_a);

                twist_back(&mut next, move1, 1.4, 0.7, 0.5, 0.9);
                next.foot_l.orientation.rotate_z(move1 * 1.4);
                next.foot_l.position += Vec3::new(-1.0, -3.0, 0.0) * move1;
                next.control.orientation.rotate_x(move1 * 2.6);
                next.control.orientation.rotate_y(move1 * 0.8);

                twist_forward(&mut next, move2, 2.1, 1.2, 0.9, 1.6);
                next.foot_l.orientation.rotate_z(move2 * -1.4);
                next.foot_l.position += Vec3::new(2.0, 7.0, 0.0) * move2;
                next.control.orientation.rotate_z(move2 * 2.1);
                next.control.orientation.rotate_x(move2 * -2.0);
                next.control.orientation.rotate_z(move2 * 1.2);
                next.control.position += Vec3::new(-16.0, 0.0, 0.0) * move2;
                next.chest.orientation.rotate_x(-0.8 * move2);
            },
            Some("common.abilities.hammer.rampart") => {
                hammer_start(&mut next, s_a);

                next.control.orientation.rotate_x(move1 * 0.6);
                next.control.orientation.rotate_y(move1 * -PI / 2.0);
                next.hand_l.orientation.rotate_y(move1 * -PI);
                next.hand_r.orientation.rotate_y(move1 * -PI);
                next.control.position += Vec3::new(-5.0, 0.0, 30.0) * move1;

                next.control.position += Vec3::new(0.0, 0.0, -10.0) * move2;
                next.torso.orientation.rotate_x(move2 * -0.6);
                next.control.orientation.rotate_x(move2 * 0.6);
            },
            Some("common.abilities.hammer.seismic_shock") => {
                hammer_start(&mut next, s_a);

                next.control.orientation.rotate_x(move1 * 2.5);
                next.control.position += Vec3::new(0.0, 0.0, 28.0) * move1;
                next.head.orientation.rotate_x(move1 * 0.3);
                next.chest.orientation.rotate_x(move1 * 0.3);
                next.belt.orientation.rotate_x(move1 * -0.2);
                next.shorts.orientation.rotate_x(move1 * -0.3);

                next.control.orientation.rotate_z(move2 * 2.0);
                next.control.orientation.rotate_x(move2 * -4.0);
                next.control.position += Vec3::new(-6.0, 0.0, -30.0) * move2;
                next.head.orientation.rotate_x(move2 * -0.9);
                next.chest.orientation.rotate_x(move2 * -0.5);
                next.belt.orientation.rotate_x(move2 * 0.2);
                next.shorts.orientation.rotate_x(move2 * 0.4);
            },
            // ==================================
            //                BOW
            // ==================================
            Some(
                "common.abilities.bow.arrow_shot"
                | "common.abilities.bow.lesser_scatterburst"
                | "common.abilities.bow.burning_arrow",
            ) => {
                bow_start(&mut next, s_a);

                let charge = chargebase.min(1.0);
                let tension = (chargebase * 15.0).sin();

                bow_draw(&mut next, move1base, d.look_dir.z);

                next.hand_l.position +=
                    Vec3::new(0.0, charge * -3.0, 0.0) + Vec3::one() * tension * 0.05;
            },
            Some(
                "common.abilities.bow.broadhead"
                | "common.abilities.bow.greater_scatterburst"
                | "common.abilities.bow.burning_broadhead",
            ) => {
                bow_start(&mut next, s_a);

                let charge = chargebase.min(1.0);
                let tension = (chargebase * 50.0).sin();

                next.hold.scale *= 1.3;
                bow_draw(&mut next, move1base, d.look_dir.z);

                next.hand_l.position +=
                    Vec3::new(0.0, charge * -5.0, 0.0) + Vec3::one() * tension * 0.05;
            },
            Some("common.abilities.bow.foothold") => {
                bow_start(&mut next, s_a);

                let move1a = (move1base * 1.5).min(1.0);
                let move1b = (move1base * 3.0 - 2.0).max(0.0).powi(4);
                let move1a_reta = move1a - move1b;
                let move2 = movementbase.min(1.0);
                let move1a_retb = move1a - move2;
                let move1b_ret = move1b - move2;

                twist_back(&mut next, move1a_reta, 1.1, 0.7, 0.5, 0.8);
                next.foot_l.orientation.rotate_z(move1a_retb * 1.4);
                next.foot_l.position += Vec3::new(-2.0, -3.0, 0.0) * move1a_retb;
                next.control.orientation.rotate_z(move1a_reta * -0.9);
                next.control.position += Vec3::new(8.0, 3.0, 0.0) * move1a_reta;

                twist_forward(&mut next, move1b_ret, 1.8, 1.1, 0.6, 1.0);
                next.foot_l.orientation.rotate_z(move1b_ret * -2.4);
                next.foot_l.orientation.rotate_x(move1b_ret * 1.2);
                next.foot_l.position += Vec3::new(11.0, 10.0, 6.0) * move1b_ret;

                bow_draw(&mut next, move2, d.look_dir.z);
            },
            Some("common.abilities.bow.snare_shot") => {
                bow_start(&mut next, s_a);

                let move1 = move1base / 0.5 + move2base / 0.5;

                next.hand_l.position += Vec3::new(2.0, -2.0, -1.0) * move1;
                next.hand_l.orientation.rotate_z(move1 * 0.4);
            },
            Some("common.abilities.bow.barrage") => {
                bow_start(&mut next, s_a);

                next.hand_l.position += Vec3::new(4.0, -6.0, -6.0) * move1;
                next.hand_l.orientation.rotate_z(move1 * 2.0);
            },
            Some("common.abilities.bow.owl_talon") => {
                bow_start(&mut next, s_a);

                next.hand_l.position += Vec3::new(-4.0, 0.0, 4.0) * move1;
                next.hand_l.orientation.rotate_x(1.6 * move1);

                next.hand_l.position += Vec3::new(0.0, 0.0, -10.0) * move2;
            },
            Some("common.abilities.bow.heavy_nock") => {
                bow_start(&mut next, s_a);

                next.hold.scale *= 1.0 + move1base / 2.0;
                next.hold.position += Vec3::new(0.0, 0.0, -2.5) * move1;
            },
            Some("common.abilities.bow.heartseeker") => {
                bow_start(&mut next, s_a);

                next.control.orientation.rotate_y(move1 * 0.4);
                next.control.orientation.rotate_x(move1 * 0.6);
                next.control.position += Vec3::new(4.0, 0.0, 6.0) * move1;
            },
            Some("common.abilities.bow.scatterburst") => {
                bow_start(&mut next, s_a);

                next.hand_l.position += Vec3::new(0.0, 5.0, 0.0) * ((move1 + move2) * 10.0).sin();
            },
            Some("common.abilities.bow.eagle_eye") => {
                bow_start(&mut next, s_a);

                next.control.orientation.rotate_x(move1 * 0.8);
                next.control.position += Vec3::new(5.0, 0.0, 7.0) * move1;
            },
            Some("common.abilities.bow.ignite_arrow") => {
                bow_start(&mut next, s_a);
            },
            // ==================================
            //             FIRE STAFF
            // ==================================
            Some("common.abilities.staff.flamethrower") => {
                let move1 = move1base;
                let move2 = move2base;
                let move3 = move3base;

                next.hand_l.position = Vec3::new(s_a.sthl.0, s_a.sthl.1, s_a.sthl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.sthl.3) * Quaternion::rotation_y(s_a.sthl.4);
                next.hand_r.position = Vec3::new(s_a.sthr.0, s_a.sthr.1, s_a.sthl.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.sthr.3) * Quaternion::rotation_y(s_a.sthr.4);
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);

                next.control.position = Vec3::new(-4.0, 7.0, 4.0);
                next.control.orientation = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.15)
                    * Quaternion::rotation_z(0.0);

                next.control.position = Vec3::new(
                    s_a.stc.0 + (move1 * 16.0) * (1.0 - move3),
                    s_a.stc.1 + (move1 + (move2 * 8.0).sin() * 2.0) * (1.0 - move3),
                    s_a.stc.2 + (move1 * 10.0) * (1.0 - move3),
                );
                next.control.orientation =
                    Quaternion::rotation_x(s_a.stc.3 + (move1 * -1.2) * (1.0 - move3))
                        * Quaternion::rotation_y(
                            s_a.stc.4
                                + (move1 * -1.4 + (move2 * 16.0).sin() * 0.07) * (1.0 - move3),
                        )
                        * Quaternion::rotation_z(
                            (move1 * -1.7 + (move2 * 8.0 + PI / 4.0).sin() * 0.3) * (1.0 - move3),
                        );
                next.head.orientation = Quaternion::rotation_x(0.0);

                next.hand_l.position = Vec3::new(
                    0.0 + (move1 * -1.0 + (move2 * 8.0).sin() * 3.5) * (1.0 - move3),
                    0.0 + (move1 * -5.0 + (move2 * 8.0).sin() * -2.0 + (move2 * 16.0).sin() * -1.5)
                        * (1.0 - move3),
                    -4.0 + (move1 * 19.0 + (move2 * 8.0 + PI / 2.0).sin() * 3.5) * (1.0 - move3),
                );
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.sthr.3 + (move1 * -0.3) * (1.0 - move3))
                        * Quaternion::rotation_y(
                            (move1 * -1.1 + (move2 * 8.0 + PI / 2.0).sin() * -0.3) * (1.0 - move3),
                        )
                        * Quaternion::rotation_z((move1 * -2.8) * (1.0 - move3));

                if d.velocity.magnitude_squared() < 0.5_f32.powi(2) {
                    next.head.orientation =
                        Quaternion::rotation_z(move1 * -0.5 + (move2 * 16.0).sin() * 0.05);

                    if !d.is_riding {
                        next.foot_l.position =
                            Vec3::new(-s_a.foot.0, s_a.foot.1 + move1 * -3.0, s_a.foot.2);
                        next.foot_l.orientation = Quaternion::rotation_x(move1 * -0.5)
                            * Quaternion::rotation_z(move1 * 0.5);

                        next.foot_r.position =
                            Vec3::new(s_a.foot.0, s_a.foot.1 + move1 * 4.0, s_a.foot.2);
                        next.foot_r.orientation = Quaternion::rotation_z(move1 * 0.5);
                    }

                    next.chest.orientation =
                        Quaternion::rotation_x(move1 * -0.2 + (move2 * 8.0).sin() * 0.05)
                            * Quaternion::rotation_z(move1 * 0.5);
                    next.belt.orientation =
                        Quaternion::rotation_x(move1 * 0.1) * Quaternion::rotation_z(move1 * -0.1);
                    next.shorts.orientation =
                        Quaternion::rotation_x(move1 * 0.2) * Quaternion::rotation_z(move1 * -0.2);
                };
            },
            Some("common.abilities.staff.fireshockwave") => {
                let move1 = move1base;
                let move2 = move2base;
                let move3 = move3base;

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

                if d.velocity.magnitude() < 0.5 && !d.is_riding {
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
            },
            Some("common.abilities.staff.firebomb") => {
                let move1 = move1base;
                let move2 = move2base.powf(0.25);
                let move3 = move3base;

                let ori: Vec2<f32> = Vec2::from(d.orientation);
                let last_ori = Vec2::from(d.last_ori);
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
                let ori_angle = d.orientation.y.atan2(d.orientation.x);
                let lookdir_angle = d.look_dir.y.atan2(d.look_dir.x);
                let swivel = lookdir_angle - ori_angle;
                let xmove = (move1 * 6.0 + PI).sin();
                let ymove = (move1 * 6.0 + PI * (0.5)).sin();

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
                    s_a.stc.2 + d.look_dir.z * 4.0,
                );
                next.control.orientation = Quaternion::rotation_x(
                    d.look_dir.z + s_a.stc.3 + (move2 * 0.6) * (1.0 - move3),
                ) * Quaternion::rotation_y(
                    s_a.stc.4 + (move1 * 0.5 + move2 * -0.5),
                ) * Quaternion::rotation_z(
                    s_a.stc.5 - (0.2 + move1 * -0.5 + move2 * 0.8) * (1.0 - move3),
                );

                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
                next.head.orientation = Quaternion::rotation_x(d.look_dir.z * 0.7)
                    * Quaternion::rotation_z(
                        tilt * -2.5 + (move1 * -0.2 + move2 * -0.4) * (1.0 - move3),
                    );
                next.chest.orientation = Quaternion::rotation_z(swivel * 0.8);
                next.torso.orientation = Quaternion::rotation_z(swivel * 0.2);
            },
            // ==================================
            //           NATURE SCEPTRE
            // ==================================
            Some(
                "common.abilities.sceptre.lifestealbeam"
                | "common.abilities.custom.cardinal.steambeam",
            ) => {
                let move1 = move1base;
                let move2 = move2base;
                let move3 = move3base;

                next.hand_l.position = Vec3::new(s_a.sthl.0, s_a.sthl.1, s_a.sthl.2);
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.sthl.3) * Quaternion::rotation_y(s_a.sthl.4);
                next.hand_r.position = Vec3::new(s_a.sthr.0, s_a.sthr.1, s_a.sthl.2);
                next.hand_r.orientation =
                    Quaternion::rotation_x(s_a.sthr.3) * Quaternion::rotation_y(s_a.sthr.4);
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);

                next.control.position = Vec3::new(-4.0, 7.0, 4.0);
                next.control.orientation = Quaternion::rotation_x(-0.3)
                    * Quaternion::rotation_y(0.15)
                    * Quaternion::rotation_z(0.0);

                next.control.position = Vec3::new(
                    s_a.stc.0 + (move1 * 16.0) * (1.0 - move3),
                    s_a.stc.1 + (move1 + (move2 * 8.0).sin() * 2.0) * (1.0 - move3),
                    s_a.stc.2 + (move1 * 10.0) * (1.0 - move3),
                );
                next.control.orientation =
                    Quaternion::rotation_x(s_a.stc.3 + (move1 * -1.2) * (1.0 - move3))
                        * Quaternion::rotation_y(
                            s_a.stc.4
                                + (move1 * -1.4 + (move2 * 16.0).sin() * 0.07) * (1.0 - move3),
                        )
                        * Quaternion::rotation_z(
                            (move1 * -1.7 + (move2 * 8.0 + PI / 4.0).sin() * 0.3) * (1.0 - move3),
                        );
                next.head.orientation = Quaternion::rotation_x(0.0);

                next.hand_l.position = Vec3::new(
                    0.0 + (move1 * -1.0 + (move2 * 8.0).sin() * 3.5) * (1.0 - move3),
                    0.0 + (move1 * -5.0 + (move2 * 8.0).sin() * -2.0 + (move2 * 16.0).sin() * -1.5)
                        * (1.0 - move3),
                    -4.0 + (move1 * 19.0 + (move2 * 8.0 + PI / 2.0).sin() * 3.5) * (1.0 - move3),
                );
                next.hand_l.orientation =
                    Quaternion::rotation_x(s_a.sthr.3 + (move1 * -0.3) * (1.0 - move3))
                        * Quaternion::rotation_y(
                            (move1 * -1.1 + (move2 * 8.0 + PI / 2.0).sin() * -0.3) * (1.0 - move3),
                        )
                        * Quaternion::rotation_z((move1 * -2.8) * (1.0 - move3));

                if d.velocity.magnitude_squared() < 0.5_f32.powi(2) {
                    next.head.orientation =
                        Quaternion::rotation_z(move1 * -0.5 + (move2 * 16.0).sin() * 0.05);

                    if !d.is_riding {
                        next.foot_l.position =
                            Vec3::new(-s_a.foot.0, s_a.foot.1 + move1 * -3.0, s_a.foot.2);
                        next.foot_l.orientation = Quaternion::rotation_x(move1 * -0.5)
                            * Quaternion::rotation_z(move1 * 0.5);

                        next.foot_r.position =
                            Vec3::new(s_a.foot.0, s_a.foot.1 + move1 * 4.0, s_a.foot.2);
                        next.foot_r.orientation = Quaternion::rotation_z(move1 * 0.5);
                    }

                    next.chest.orientation =
                        Quaternion::rotation_x(move1 * -0.2 + (move2 * 8.0).sin() * 0.05)
                            * Quaternion::rotation_z(move1 * 0.5);
                    next.belt.orientation =
                        Quaternion::rotation_x(move1 * 0.1) * Quaternion::rotation_z(move1 * -0.1);
                    next.shorts.orientation =
                        Quaternion::rotation_x(move1 * 0.2) * Quaternion::rotation_z(move1 * -0.2);
                };
            },
            Some(
                "common.abilities.sceptre.healingaura" | "common.abilities.sceptre.wardingaura",
            ) => {
                let move1 = move1base;
                let move2 = move2base;
                let move3 = move3base;

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

                if d.velocity.magnitude() < 0.5 && !d.is_riding {
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
            },
            // ==================================
            //               SHIELD
            // ==================================
            Some("common.abilities.shield.basic_guard" | "common.abilities.shield.power_guard") => {
                legacy_initialize();
                let pullback = 1.0 - move3base.powi(4);
                let move1 = move1base.powf(0.25) * pullback;
                let move2 = (move2base * 10.0).sin();

                if d.velocity.xy().magnitude_squared() < 0.5_f32.powi(2) {
                    next.chest.position =
                        Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + move1 * -1.0 + move2 * 0.2);
                    next.chest.orientation = Quaternion::rotation_x(move1 * -0.15);
                    next.head.orientation = Quaternion::rotation_x(move1 * 0.25);

                    next.belt.position =
                        Vec3::new(0.0, s_a.belt.0 + move1 * 0.5, s_a.belt.1 + move1 * 0.5);
                    next.shorts.position =
                        Vec3::new(0.0, s_a.shorts.0 + move1 * 1.3, s_a.shorts.1 + move1 * 1.0);

                    next.belt.orientation = Quaternion::rotation_x(move1 * 0.15);
                    next.shorts.orientation = Quaternion::rotation_x(move1 * 0.25);

                    if !d.is_riding {
                        next.foot_l.position =
                            Vec3::new(-s_a.foot.0, s_a.foot.1 + move1 * 2.0, s_a.foot.2);
                        next.foot_l.orientation = Quaternion::rotation_z(move1 * -0.5);

                        next.foot_r.position =
                            Vec3::new(s_a.foot.0, s_a.foot.1 + move1 * -2.0, s_a.foot.2);
                        next.foot_r.orientation = Quaternion::rotation_x(move1 * -0.5);
                    }
                }

                if let Some(info) = d.ability_info {
                    match info.hand {
                        Some(HandInfo::MainHand) => {
                            next.control_l.position = Vec3::new(1.5, 8.0, 4.0 + move1 * 3.0);
                            next.control_l.orientation = Quaternion::rotation_x(0.25)
                                * Quaternion::rotation_y(0.0)
                                * Quaternion::rotation_z(-1.5);
                            next.hand_l.position = Vec3::new(0.0, -2.0, 0.0);
                            next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0);

                            next.control_r.position = Vec3::new(9.0, -5.0, 0.0);
                            next.control_r.orientation =
                                Quaternion::rotation_x(-1.75) * Quaternion::rotation_y(0.3);
                            next.hand_r.position = Vec3::new(0.0, -0.5, 0.0);
                            next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0);
                        },
                        Some(HandInfo::OffHand) => {
                            next.control_r.position = Vec3::new(-1.5, 8.0, 4.0 + move1 * 3.0);
                            next.control_r.orientation = Quaternion::rotation_x(0.25)
                                * Quaternion::rotation_y(0.0)
                                * Quaternion::rotation_z(1.5);
                            next.hand_r.position = Vec3::new(0.0, -2.0, 0.0);
                            next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0);

                            next.control_l.position = Vec3::new(-9.0, -5.0, 0.0);
                            next.control_l.orientation =
                                Quaternion::rotation_x(-1.75) * Quaternion::rotation_y(-0.3);
                            next.hand_l.position = Vec3::new(0.0, -0.5, 0.0);
                            next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0);
                        },
                        Some(HandInfo::TwoHanded) | None => {},
                    }
                }
            },
            // ==================================
            //            MISCELLANEOUS
            // ==================================
            Some("common.abilities.pick.swing") => {
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);
                next.second.position = Vec3::new(0.0, 0.0, 0.0);
                next.second.orientation = Quaternion::rotation_z(0.0);
                next.torso.position = Vec3::new(0.0, 0.0, 1.1);
                next.torso.orientation = Quaternion::rotation_z(0.0);

                let move1 = move1base.powf(0.25);
                let move3 = move3base.powi(4);
                let pullback = 1.0 - move3;
                let moveret1 = move1base * pullback;
                let moveret2 = move2base * pullback;

                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
                next.head.orientation = Quaternion::rotation_x(moveret1 * 0.1 + moveret2 * 0.3)
                    * Quaternion::rotation_z(move1 * -0.2 + moveret2 * 0.2);
                next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + moveret2 * -2.0);
                next.chest.orientation = Quaternion::rotation_x(moveret1 * 0.4 + moveret2 * -0.7)
                    * Quaternion::rotation_y(moveret1 * 0.3 + moveret2 * -0.4)
                    * Quaternion::rotation_z(moveret1 * 0.5 + moveret2 * -0.5);

                next.hand_l.position = Vec3::new(s_a.hhl.0, s_a.hhl.1, s_a.hhl.2 + moveret2 * -7.0);
                next.hand_l.orientation = Quaternion::rotation_x(s_a.hhl.3)
                    * Quaternion::rotation_y(s_a.hhl.4)
                    * Quaternion::rotation_z(s_a.hhl.5);
                next.hand_r.position = Vec3::new(s_a.hhr.0, s_a.hhr.1, s_a.hhr.2);
                next.hand_r.orientation = Quaternion::rotation_x(s_a.hhr.3)
                    * Quaternion::rotation_y(s_a.hhr.4)
                    * Quaternion::rotation_z(s_a.hhr.5);

                next.control.position = Vec3::new(
                    s_a.hc.0 + moveret1 * -13.0 + moveret2 * 3.0,
                    s_a.hc.1 + (moveret2 * 5.0),
                    s_a.hc.2 + moveret1 * 8.0 + moveret2 * -6.0,
                );
                next.control.orientation =
                    Quaternion::rotation_x(s_a.hc.3 + (moveret1 * 1.5 + moveret2 * -2.55))
                        * Quaternion::rotation_y(s_a.hc.4 + moveret1 * PI / 2.0 + moveret2 * 0.5)
                        * Quaternion::rotation_z(s_a.hc.5 + (moveret2 * -0.5));

                if skeleton.holding_lantern {
                    next.hand_r.position =
                        Vec3::new(s_a.hand.0, s_a.hand.1 + 5.0, s_a.hand.2 + 12.0);
                    next.hand_r.orientation =
                        Quaternion::rotation_x(2.25) * Quaternion::rotation_z(0.9);

                    next.lantern.position = Vec3::new(-0.5, -0.5, -1.5);
                    next.lantern.orientation = next.hand_r.orientation.inverse();
                }
            },
            Some("common.abilities.shovel.dig") => {
                next.main.position = Vec3::new(0.0, 0.0, 0.0);
                next.main.orientation = Quaternion::rotation_x(0.0);
                next.second.position = Vec3::new(0.0, 0.0, 0.0);
                next.second.orientation = Quaternion::rotation_z(0.0);
                next.torso.position = Vec3::new(0.0, 0.0, 1.1);
                next.torso.orientation = Quaternion::rotation_z(0.0);

                let move1 = move1base.powf(0.25);
                let move3 = move3base.powi(4);
                let pullback = 1.0 - move3;
                let moveret1 = move1base * pullback;
                let moveret2 = move2base * pullback;

                next.head.position = Vec3::new(0.0, s_a.head.0, s_a.head.1);
                next.head.orientation = Quaternion::rotation_x(moveret1 * 0.1 + moveret2 * 0.3)
                    * Quaternion::rotation_z(move1 * -0.2 + moveret2 * 0.2);
                next.chest.position = Vec3::new(0.0, s_a.chest.0, s_a.chest.1 + moveret2 * -2.0);
                next.chest.orientation = Quaternion::rotation_x(moveret1 * 0.4 + moveret2 * -0.7)
                    * Quaternion::rotation_y(moveret1 * 0.3 + moveret2 * -0.4)
                    * Quaternion::rotation_z(moveret1 * 0.5 + moveret2 * -0.5);

                next.hand_l.position = Vec3::new(8.0, 6.0, 3.0);
                next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0);
                next.hand_r.position = Vec3::new(8.0, 6.0, 15.0);
                next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0);
                next.main.position = Vec3::new(7.5, 7.5, 13.2);
                next.main.orientation = Quaternion::rotation_y(PI);

                next.control.position = Vec3::new(-11.0 + moveret1 * 8.0, 1.8, 4.0);
                next.control.orientation = Quaternion::rotation_x(moveret1 * 0.3 + moveret2 * 0.2)
                    * Quaternion::rotation_y(0.8 - moveret1 * 0.7 + moveret2 * 0.7)
                    * Quaternion::rotation_z(moveret2 * 0.1 - moveret1 * 0.4);

                if skeleton.holding_lantern {
                    next.hand_r.position =
                        Vec3::new(s_a.hand.0, s_a.hand.1 + 5.0, s_a.hand.2 + 12.0);
                    next.hand_r.orientation =
                        Quaternion::rotation_x(2.25) * Quaternion::rotation_z(0.9);

                    next.lantern.position = Vec3::new(-0.5, -0.5, -1.5);
                    next.lantern.orientation = next.hand_r.orientation.inverse();
                }
            },
            _ => {},
        }

        next
    }
}
