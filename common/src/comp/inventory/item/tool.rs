// Note: If you changes here "break" old character saves you can change the
// version in voxygen\src\meta.rs in order to reset save files to being empty

use crate::{
    comp::{
        body::object,
        buff::{BuffCategory, BuffData, BuffKind},
        projectile, Body, CharacterAbility, Gravity, LightEmitter, Projectile,
    },
    effect::{BuffEffect, Effect},
    states::combo_melee,
    Damage, DamageSource, Explosion, GroupTarget, Knockback, RadiusEffect,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolKind {
    Sword,
    Axe,
    Hammer,
    Bow,
    Dagger,
    Staff,
    Sceptre,
    Shield,
    Unique(UniqueKind),
    Debug,
    Farming,
    /// This is an placeholder item, it is used by non-humanoid npcs to attack
    Empty,
}

impl ToolKind {
    pub fn hands(&self) -> Hands {
        match self {
            ToolKind::Sword => Hands::TwoHand,
            ToolKind::Axe => Hands::TwoHand,
            ToolKind::Hammer => Hands::TwoHand,
            ToolKind::Bow => Hands::TwoHand,
            ToolKind::Dagger => Hands::OneHand,
            ToolKind::Staff => Hands::TwoHand,
            ToolKind::Sceptre => Hands::TwoHand,
            ToolKind::Shield => Hands::OneHand,
            ToolKind::Unique(_) => Hands::TwoHand,
            ToolKind::Debug => Hands::TwoHand,
            ToolKind::Farming => Hands::TwoHand,
            ToolKind::Empty => Hands::OneHand,
        }
    }
}

pub enum Hands {
    OneHand,
    TwoHand,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    equip_time_millis: u32,
    power: f32,
    speed: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    pub kind: ToolKind,
    pub stats: Stats,
    // TODO: item specific abilities
}

impl Tool {
    pub fn empty() -> Self {
        Self {
            kind: ToolKind::Empty,
            stats: Stats {
                equip_time_millis: 0,
                power: 1.00,
                speed: 1.00,
            },
        }
    }

    // Keep power between 0.5 and 2.00
    pub fn base_power(&self) -> f32 { self.stats.power }

    pub fn base_speed(&self) -> f32 { self.stats.speed }

    pub fn equip_time(&self) -> Duration {
        Duration::from_millis(self.stats.equip_time_millis as u64)
    }

    /// Converts milliseconds to a `Duration` adjusted by `base_speed()`
    pub fn adjusted_duration(&self, millis: u64) -> Duration {
        Duration::from_millis(millis).div_f32(self.base_speed())
    }

    pub fn get_abilities(&self) -> Vec<CharacterAbility> {
        use CharacterAbility::*;
        use ToolKind::*;

        use UniqueKind::*;
        match &self.kind {
            Sword => vec![
                ComboMelee {
                    stage_data: vec![
                        combo_melee::Stage {
                            stage: 1,
                            base_damage: (100.0 * self.base_power()) as u32,
                            max_damage: (120.0 * self.base_power()) as u32,
                            damage_increase: (10.0 * self.base_power()) as u32,
                            knockback: 10.0,
                            range: 4.0,
                            angle: 30.0,
                            base_buildup_duration: self.adjusted_duration(350),
                            base_swing_duration: self.adjusted_duration(100),
                            base_recover_duration: self.adjusted_duration(400),
                            forward_movement: 0.5,
                        },
                        combo_melee::Stage {
                            stage: 2,
                            base_damage: (80.0 * self.base_power()) as u32,
                            max_damage: (110.0 * self.base_power()) as u32,
                            damage_increase: (15.0 * self.base_power()) as u32,
                            knockback: 12.0,
                            range: 3.5,
                            angle: 180.0,
                            base_buildup_duration: self.adjusted_duration(400),
                            base_swing_duration: self.adjusted_duration(600),
                            base_recover_duration: self.adjusted_duration(400),
                            forward_movement: 0.0,
                        },
                        combo_melee::Stage {
                            stage: 3,
                            base_damage: (130.0 * self.base_power()) as u32,
                            max_damage: (170.0 * self.base_power()) as u32,
                            damage_increase: (20.0 * self.base_power()) as u32,
                            knockback: 14.0,
                            range: 6.0,
                            angle: 10.0,
                            base_buildup_duration: self.adjusted_duration(500),
                            base_swing_duration: self.adjusted_duration(200),
                            base_recover_duration: self.adjusted_duration(300),
                            forward_movement: 1.2,
                        },
                    ],
                    initial_energy_gain: 0,
                    max_energy_gain: 100,
                    energy_increase: 20,
                    speed_increase: 0.05,
                    max_speed_increase: 1.8,
                    is_interruptible: true,
                },
                DashMelee {
                    energy_cost: 200,
                    base_damage: (120.0 * self.base_power()) as u32,
                    max_damage: (240.0 * self.base_power()) as u32,
                    base_knockback: 8.0,
                    max_knockback: 15.0,
                    range: 5.0,
                    angle: 45.0,
                    energy_drain: 500,
                    forward_speed: 4.0,
                    buildup_duration: self.adjusted_duration(250),
                    charge_duration: Duration::from_millis(600),
                    swing_duration: self.adjusted_duration(100),
                    recover_duration: self.adjusted_duration(500),
                    infinite_charge: true,
                    is_interruptible: true,
                },
                SpinMelee {
                    buildup_duration: self.adjusted_duration(750),
                    swing_duration: self.adjusted_duration(500),
                    recover_duration: self.adjusted_duration(500),
                    base_damage: (140.0 * self.base_power()) as u32,
                    knockback: 10.0,
                    range: 3.5,
                    energy_cost: 200,
                    is_infinite: false,
                    is_helicopter: false,
                    is_interruptible: true,
                    forward_speed: 1.0,
                    num_spins: 3,
                },
            ],
            Axe => vec![
                ComboMelee {
                    stage_data: vec![
                        combo_melee::Stage {
                            stage: 1,
                            base_damage: (90.0 * self.base_power()) as u32,
                            max_damage: (110.0 * self.base_power()) as u32,
                            damage_increase: (10.0 * self.base_power()) as u32,
                            knockback: 8.0,
                            range: 3.5,
                            angle: 50.0,
                            base_buildup_duration: self.adjusted_duration(350),
                            base_swing_duration: self.adjusted_duration(75),
                            base_recover_duration: self.adjusted_duration(400),
                            forward_movement: 0.5,
                        },
                        combo_melee::Stage {
                            stage: 2,
                            base_damage: (130.0 * self.base_power()) as u32,
                            max_damage: (160.0 * self.base_power()) as u32,
                            damage_increase: (15.0 * self.base_power()) as u32,
                            knockback: 12.0,
                            range: 3.5,
                            angle: 30.0,
                            base_buildup_duration: self.adjusted_duration(500),
                            base_swing_duration: self.adjusted_duration(100),
                            base_recover_duration: self.adjusted_duration(500),
                            forward_movement: 0.25,
                        },
                    ],
                    initial_energy_gain: 0,
                    max_energy_gain: 100,
                    energy_increase: 20,
                    speed_increase: 0.05,
                    max_speed_increase: 1.6,
                    is_interruptible: false,
                },
                SpinMelee {
                    buildup_duration: self.adjusted_duration(100),
                    swing_duration: self.adjusted_duration(250),
                    recover_duration: self.adjusted_duration(100),
                    base_damage: (60.0 * self.base_power()) as u32,
                    knockback: 0.0,
                    range: 3.5,
                    energy_cost: 100,
                    is_infinite: true,
                    is_helicopter: true,
                    is_interruptible: false,
                    forward_speed: 0.0,
                    num_spins: 1,
                },
                LeapMelee {
                    energy_cost: 450,
                    buildup_duration: self.adjusted_duration(200),
                    movement_duration: Duration::from_millis(200),
                    swing_duration: self.adjusted_duration(200),
                    recover_duration: self.adjusted_duration(200),
                    base_damage: (240.0 * self.base_power()) as u32,
                    knockback: 12.0,
                    range: 4.5,
                    max_angle: 30.0,
                    forward_leap_strength: 28.0,
                    vertical_leap_strength: 8.0,
                },
            ],
            Hammer => vec![
                ComboMelee {
                    stage_data: vec![combo_melee::Stage {
                        stage: 1,
                        base_damage: (120.0 * self.base_power()) as u32,
                        max_damage: (150.0 * self.base_power()) as u32,
                        damage_increase: (10.0 * self.base_power()) as u32,
                        knockback: 0.0,
                        range: 3.5,
                        angle: 20.0,
                        base_buildup_duration: self.adjusted_duration(600),
                        base_swing_duration: self.adjusted_duration(60),
                        base_recover_duration: self.adjusted_duration(300),
                        forward_movement: 0.0,
                    }],
                    initial_energy_gain: 0,
                    max_energy_gain: 100,
                    energy_increase: 20,
                    speed_increase: 0.05,
                    max_speed_increase: 1.4,
                    is_interruptible: false,
                },
                ChargedMelee {
                    energy_cost: 1,
                    energy_drain: 300,
                    initial_damage: (10.0 * self.base_power()) as u32,
                    max_damage: (170.0 * self.base_power()) as u32,
                    initial_knockback: 10.0,
                    max_knockback: 60.0,
                    range: 3.5,
                    max_angle: 30.0,
                    speed: self.base_speed(),
                    charge_duration: Duration::from_millis(1200),
                    swing_duration: self.adjusted_duration(200),
                    recover_duration: self.adjusted_duration(300),
                },
                LeapMelee {
                    energy_cost: 700,
                    buildup_duration: self.adjusted_duration(100),
                    movement_duration: Duration::from_millis(800),
                    swing_duration: self.adjusted_duration(150),
                    recover_duration: self.adjusted_duration(200),
                    base_damage: (240.0 * self.base_power()) as u32,
                    knockback: 25.0,
                    range: 4.5,
                    max_angle: 360.0,
                    forward_leap_strength: 28.0,
                    vertical_leap_strength: 8.0,
                },
            ],
            Farming => vec![BasicMelee {
                energy_cost: 1,
                buildup_duration: self.adjusted_duration(600),
                swing_duration: self.adjusted_duration(100),
                recover_duration: self.adjusted_duration(150),
                base_damage: (50.0 * self.base_power()) as u32,
                knockback: 0.0,
                range: 3.5,
                max_angle: 20.0,
            }],
            Bow => vec![
                BasicRanged {
                    energy_cost: 0,
                    buildup_duration: self.adjusted_duration(200),
                    recover_duration: self.adjusted_duration(300),
                    projectile: Projectile {
                        hit_solid: vec![projectile::Effect::Stick],
                        hit_entity: vec![
                            projectile::Effect::Damage(Some(GroupTarget::OutOfGroup), Damage {
                                source: DamageSource::Projectile,
                                value: 40.0 * self.base_power(),
                            }),
                            projectile::Effect::Knockback(Knockback::Away(10.0)),
                            projectile::Effect::RewardEnergy(50),
                            projectile::Effect::Vanish,
                            projectile::Effect::Buff {
                                buff: BuffEffect {
                                    kind: BuffKind::Bleeding,
                                    data: BuffData {
                                        strength: 20.0 * self.base_power(),
                                        duration: Some(Duration::from_secs(5)),
                                    },
                                    cat_ids: vec![BuffCategory::Physical],
                                },
                                chance: Some(0.10),
                            },
                        ],
                        time_left: Duration::from_secs(15),
                        owner: None,
                        ignore_group: true,
                    },
                    projectile_body: Body::Object(object::Body::Arrow),
                    projectile_light: None,
                    projectile_gravity: Some(Gravity(0.2)),
                    projectile_speed: 100.0,
                    can_continue: true,
                },
                ChargedRanged {
                    energy_cost: 0,
                    energy_drain: 300,
                    initial_damage: (40.0 * self.base_power()) as u32,
                    max_damage: (200.0 * self.base_power()) as u32,
                    initial_knockback: 10.0,
                    max_knockback: 20.0,
                    speed: self.base_speed(),
                    buildup_duration: self.adjusted_duration(100),
                    charge_duration: Duration::from_millis(1500),
                    recover_duration: self.adjusted_duration(500),
                    projectile_body: Body::Object(object::Body::MultiArrow),
                    projectile_light: None,
                    projectile_gravity: Some(Gravity(0.2)),
                    initial_projectile_speed: 100.0,
                    max_projectile_speed: 500.0,
                },
                RepeaterRanged {
                    energy_cost: 450,
                    movement_duration: Duration::from_millis(300),
                    buildup_duration: self.adjusted_duration(200),
                    shoot_duration: self.adjusted_duration(200),
                    recover_duration: self.adjusted_duration(800),
                    leap: Some(5.0),
                    projectile: Projectile {
                        hit_solid: vec![projectile::Effect::Stick],
                        hit_entity: vec![
                            projectile::Effect::Damage(Some(GroupTarget::OutOfGroup), Damage {
                                source: DamageSource::Projectile,
                                value: 40.0 * self.base_power(),
                            }),
                            projectile::Effect::Knockback(Knockback::Away(10.0)),
                            projectile::Effect::Vanish,
                            projectile::Effect::Buff {
                                buff: BuffEffect {
                                    kind: BuffKind::Bleeding,
                                    data: BuffData {
                                        strength: 20.0 * self.base_power(),
                                        duration: Some(Duration::from_secs(5)),
                                    },
                                    cat_ids: vec![BuffCategory::Physical],
                                },
                                chance: Some(0.10),
                            },
                        ],
                        time_left: Duration::from_secs(15),
                        owner: None,
                        ignore_group: true,
                    },
                    projectile_body: Body::Object(object::Body::Arrow),
                    projectile_light: None,
                    projectile_gravity: Some(Gravity(0.2)),
                    projectile_speed: 100.0,
                    reps_remaining: 5,
                },
            ],
            Dagger => vec![BasicMelee {
                energy_cost: 0,
                buildup_duration: self.adjusted_duration(100),
                swing_duration: self.adjusted_duration(100),
                recover_duration: self.adjusted_duration(300),
                base_damage: (50.0 * self.base_power()) as u32,
                knockback: 0.0,
                range: 3.5,
                max_angle: 20.0,
            }],
            Sceptre => vec![
                BasicBeam {
                    buildup_duration: self.adjusted_duration(250),
                    recover_duration: self.adjusted_duration(250),
                    beam_duration: Duration::from_secs(1),
                    base_hps: (60.0 * self.base_power()) as u32,
                    base_dps: (60.0 * self.base_power()) as u32,
                    tick_rate: 2.0 * self.base_speed(),
                    range: 25.0,
                    max_angle: 1.0,
                    lifesteal_eff: 0.20,
                    energy_regen: 50,
                    energy_cost: 100,
                    energy_drain: 0,
                },
                BasicRanged {
                    energy_cost: 800,
                    buildup_duration: self.adjusted_duration(800),
                    recover_duration: self.adjusted_duration(50),
                    projectile: Projectile {
                        hit_solid: vec![
                            projectile::Effect::Explode(Explosion {
                                effects: vec![
                                    RadiusEffect::Entity(
                                        Some(GroupTarget::OutOfGroup),
                                        Effect::Damage(Damage {
                                            source: DamageSource::Explosion,
                                            value: 50.0 * self.base_power(),
                                        }),
                                    ),
                                    RadiusEffect::Entity(
                                        Some(GroupTarget::InGroup),
                                        Effect::Damage(Damage {
                                            source: DamageSource::Healing,
                                            value: 140.0 * self.base_power(),
                                        }),
                                    ),
                                ],
                                radius: 3.0 + 2.5 * self.base_power(),
                                energy_regen: 0,
                            }),
                            projectile::Effect::Vanish,
                        ],
                        hit_entity: vec![
                            projectile::Effect::Explode(Explosion {
                                effects: vec![
                                    RadiusEffect::Entity(
                                        Some(GroupTarget::OutOfGroup),
                                        Effect::Damage(Damage {
                                            source: DamageSource::Explosion,
                                            value: 50.0 * self.base_power(),
                                        }),
                                    ),
                                    RadiusEffect::Entity(
                                        Some(GroupTarget::InGroup),
                                        Effect::Damage(Damage {
                                            source: DamageSource::Healing,
                                            value: 140.0 * self.base_power(),
                                        }),
                                    ),
                                ],
                                radius: 3.0 + 2.5 * self.base_power(),
                                energy_regen: 0,
                            }),
                            projectile::Effect::Vanish,
                        ],
                        time_left: Duration::from_secs(20),
                        owner: None,
                        ignore_group: true,
                    },
                    projectile_body: Body::Object(object::Body::BoltNature),
                    projectile_light: Some(LightEmitter {
                        col: (0.0, 1.0, 0.0).into(),
                        ..Default::default()
                    }),
                    projectile_gravity: Some(Gravity(0.5)),
                    projectile_speed: 40.0,
                    can_continue: false,
                },
            ],
            Staff => vec![
                BasicRanged {
                    energy_cost: 0,
                    buildup_duration: self.adjusted_duration(500),
                    recover_duration: self.adjusted_duration(350),
                    projectile: Projectile {
                        hit_solid: vec![
                            projectile::Effect::Explode(Explosion {
                                effects: vec![RadiusEffect::Entity(
                                    Some(GroupTarget::OutOfGroup),
                                    Effect::Damage(Damage {
                                        source: DamageSource::Explosion,
                                        value: 100.0 * self.base_power(),
                                    }),
                                )],
                                radius: 5.0,
                                energy_regen: 50,
                            }),
                            projectile::Effect::Vanish,
                        ],
                        hit_entity: vec![
                            projectile::Effect::Explode(Explosion {
                                effects: vec![RadiusEffect::Entity(
                                    Some(GroupTarget::OutOfGroup),
                                    Effect::Damage(Damage {
                                        source: DamageSource::Explosion,
                                        value: 100.0 * self.base_power(),
                                    }),
                                )],
                                radius: 5.0,
                                energy_regen: 50,
                            }),
                            projectile::Effect::Vanish,
                        ],
                        time_left: Duration::from_secs(20),
                        owner: None,
                        ignore_group: true,
                    },
                    projectile_body: Body::Object(object::Body::BoltFire),
                    projectile_light: Some(LightEmitter {
                        col: (1.0, 0.75, 0.11).into(),
                        ..Default::default()
                    }),
                    projectile_gravity: Some(Gravity(0.3)),
                    projectile_speed: 60.0,
                    can_continue: true,
                },
                BasicBeam {
                    buildup_duration: self.adjusted_duration(250),
                    recover_duration: self.adjusted_duration(250),
                    beam_duration: self.adjusted_duration(500),
                    base_hps: 0,
                    base_dps: (150.0 * self.base_power()) as u32,
                    tick_rate: 3.0 * self.base_speed(),
                    range: 15.0,
                    max_angle: 22.5,
                    lifesteal_eff: 0.0,
                    energy_regen: 0,
                    energy_cost: 0,
                    energy_drain: 350,
                },
                Shockwave {
                    energy_cost: 600,
                    buildup_duration: self.adjusted_duration(700),
                    swing_duration: self.adjusted_duration(100),
                    recover_duration: self.adjusted_duration(300),
                    damage: (200.0 * self.base_power()) as u32,
                    knockback: Knockback::Away(25.0),
                    shockwave_angle: 360.0,
                    shockwave_vertical_angle: 90.0,
                    shockwave_speed: 20.0,
                    shockwave_duration: Duration::from_millis(500),
                    requires_ground: false,
                    move_efficiency: 0.1,
                },
            ],
            Shield => vec![
                BasicMelee {
                    energy_cost: 0,
                    buildup_duration: self.adjusted_duration(100),
                    swing_duration: self.adjusted_duration(100),
                    recover_duration: self.adjusted_duration(300),
                    base_damage: (40.0 * self.base_power()) as u32,
                    knockback: 0.0,
                    range: 3.0,
                    max_angle: 120.0,
                },
            ],
            Unique(StoneGolemFist) => vec![
                BasicMelee {
                    energy_cost: 0,
                    buildup_duration: self.adjusted_duration(400),
                    swing_duration: self.adjusted_duration(100),
                    recover_duration: self.adjusted_duration(250),
                    knockback: 25.0,
                    base_damage: 200,
                    range: 5.0,
                    max_angle: 120.0,
                },
                Shockwave {
                    energy_cost: 0,
                    buildup_duration: self.adjusted_duration(500),
                    swing_duration: self.adjusted_duration(200),
                    recover_duration: self.adjusted_duration(800),
                    damage: 500,
                    knockback: Knockback::TowardsUp(40.0),
                    shockwave_angle: 90.0,
                    shockwave_vertical_angle: 15.0,
                    shockwave_speed: 20.0,
                    shockwave_duration: Duration::from_millis(2000),
                    requires_ground: true,
                    move_efficiency: 0.05,
                },
            ],
            Unique(BeastClaws) => vec![BasicMelee {
                energy_cost: 0,
                buildup_duration: self.adjusted_duration(250),
                swing_duration: self.adjusted_duration(250),
                recover_duration: self.adjusted_duration(250),
                knockback: 25.0,
                base_damage: 200,
                range: 5.0,
                max_angle: 120.0,
            }],
            Debug => vec![
                CharacterAbility::Boost {
                    movement_duration: Duration::from_millis(50),
                    only_up: false,
                },
                CharacterAbility::Boost {
                    movement_duration: Duration::from_millis(50),
                    only_up: true,
                },
                BasicRanged {
                    energy_cost: 0,
                    buildup_duration: Duration::from_millis(0),
                    recover_duration: self.adjusted_duration(10),
                    projectile: Projectile {
                        hit_solid: vec![projectile::Effect::Stick],
                        hit_entity: vec![projectile::Effect::Stick, projectile::Effect::Possess],
                        time_left: Duration::from_secs(10),
                        owner: None,
                        ignore_group: false,
                    },
                    projectile_body: Body::Object(object::Body::ArrowSnake),
                    projectile_light: Some(LightEmitter {
                        col: (0.0, 1.0, 0.33).into(),
                        ..Default::default()
                    }),
                    projectile_gravity: None,
                    projectile_speed: 100.0,
                    can_continue: false,
                },
            ],
            Empty => vec![BasicMelee {
                energy_cost: 0,
                buildup_duration: Duration::from_millis(0),
                swing_duration: self.adjusted_duration(100),
                recover_duration: self.adjusted_duration(900),
                base_damage: 20,
                knockback: 0.0,
                range: 3.5,
                max_angle: 15.0,
            }],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UniqueKind {
    StoneGolemFist,
    BeastClaws,
}
