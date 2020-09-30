// Note: If you changes here "break" old character saves you can change the
// version in voxygen\src\meta.rs in order to reset save files to being empty

use crate::{
    comp::{body::object, projectile, Body, CharacterAbility, Gravity, LightEmitter, Projectile},
    states::combo_melee,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolKind {
    Sword(String),
    Axe(String),
    Hammer(String),
    Bow(String),
    Dagger(String),
    Staff(String),
    Sceptre(String),
    Shield(String),
    NpcWeapon(String),
    Debug(String),
    Farming(String),
    /// This is an placeholder item, it is used by non-humanoid npcs to attack
    Empty,
}

impl ToolKind {
    pub fn hands(&self) -> Hands {
        match self {
            ToolKind::Sword(_) => Hands::TwoHand,
            ToolKind::Axe(_) => Hands::TwoHand,
            ToolKind::Hammer(_) => Hands::TwoHand,
            ToolKind::Bow(_) => Hands::TwoHand,
            ToolKind::Dagger(_) => Hands::OneHand,
            ToolKind::Staff(_) => Hands::TwoHand,
            ToolKind::Sceptre(_) => Hands::TwoHand,
            ToolKind::Shield(_) => Hands::OneHand,
            ToolKind::NpcWeapon(_) => Hands::TwoHand,
            ToolKind::Debug(_) => Hands::TwoHand,
            ToolKind::Farming(_) => Hands::TwoHand,
            ToolKind::Empty => Hands::OneHand,
        }
    }
}

pub enum Hands {
    OneHand,
    TwoHand,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolCategory {
    Sword,
    Axe,
    Hammer,
    Bow,
    Dagger,
    Staff,
    Sceptre,
    Shield,
    NpcWeapon,
    Debug,
    Farming,
    Empty,
}

impl From<&ToolKind> for ToolCategory {
    fn from(kind: &ToolKind) -> ToolCategory {
        match kind {
            ToolKind::Sword(_) => ToolCategory::Sword,
            ToolKind::Axe(_) => ToolCategory::Axe,
            ToolKind::Hammer(_) => ToolCategory::Hammer,
            ToolKind::Bow(_) => ToolCategory::Bow,
            ToolKind::Dagger(_) => ToolCategory::Dagger,
            ToolKind::Staff(_) => ToolCategory::Staff,
            ToolKind::Sceptre(_) => ToolCategory::Sceptre,
            ToolKind::Shield(_) => ToolCategory::Shield,
            ToolKind::NpcWeapon(_) => ToolCategory::NpcWeapon,
            ToolKind::Debug(_) => ToolCategory::Debug,
            ToolKind::Farming(_) => ToolCategory::Farming,
            ToolKind::Empty => ToolCategory::Empty,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    equip_time_millis: u32,
    power: f32,
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
            },
        }
    }

    // Keep power between 0.5 and 2.00
    pub fn base_power(&self) -> f32 { self.stats.power }

    pub fn equip_time(&self) -> Duration {
        Duration::from_millis(self.stats.equip_time_millis as u64)
    }

    pub fn get_abilities(&self) -> Vec<CharacterAbility> {
        use CharacterAbility::*;
        use ToolKind::*;

        match &self.kind {
            Sword(_) => vec![
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
                            base_buildup_duration: Duration::from_millis(350),
                            base_swing_duration: Duration::from_millis(100),
                            base_recover_duration: Duration::from_millis(400),
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
                            base_buildup_duration: Duration::from_millis(400),
                            base_swing_duration: Duration::from_millis(600),
                            base_recover_duration: Duration::from_millis(400),
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
                            base_buildup_duration: Duration::from_millis(500),
                            base_swing_duration: Duration::from_millis(200),
                            base_recover_duration: Duration::from_millis(300),
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
                    max_damage: (260.0 * self.base_power()) as u32,
                    base_knockback: 10.0,
                    max_knockback: 20.0,
                    range: 5.0,
                    angle: 45.0,
                    energy_drain: 500,
                    forward_speed: 4.0,
                    buildup_duration: Duration::from_millis(250),
                    charge_duration: Duration::from_millis(400),
                    swing_duration: Duration::from_millis(100),
                    recover_duration: Duration::from_millis(500),
                    infinite_charge: true,
                    is_interruptible: true,
                },
                SpinMelee {
                    buildup_duration: Duration::from_millis(750),
                    swing_duration: Duration::from_millis(500),
                    recover_duration: Duration::from_millis(500),
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
            Axe(_) => vec![
                BasicMelee {
                    energy_cost: 0,
                    buildup_duration: Duration::from_millis(700),
                    recover_duration: Duration::from_millis(300),
                    base_healthchange: (-120.0 * self.base_power()) as i32,
                    knockback: 0.0,
                    range: 3.5,
                    max_angle: 20.0,
                },
                SpinMelee {
                    buildup_duration: Duration::from_millis(100),
                    swing_duration: Duration::from_millis(250),
                    recover_duration: Duration::from_millis(100),
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
                    energy_cost: 600,
                    buildup_duration: Duration::from_millis(100),
                    movement_duration: Duration::from_millis(600),
                    swing_duration: Duration::from_millis(100),
                    recover_duration: Duration::from_millis(100),
                    base_damage: (240.0 * self.base_power()) as u32,
                    knockback: 12.0,
                    range: 4.5,
                    max_angle: 30.0,
                    forward_leap_strength: 20.0,
                    vertical_leap_strength: 8.0,
                },
            ],
            Hammer(_) => vec![
                BasicMelee {
                    energy_cost: 0,
                    buildup_duration: Duration::from_millis(700),
                    recover_duration: Duration::from_millis(300),
                    base_healthchange: (-120.0 * self.base_power()) as i32,
                    knockback: 0.0,
                    range: 3.5,
                    max_angle: 20.0,
                },
                ChargedMelee {
                    energy_cost: 0,
                    energy_drain: 300,
                    initial_damage: (20.0 * self.base_power()) as u32,
                    max_damage: (170.0 * self.base_power()) as u32,
                    initial_knockback: 12.0,
                    max_knockback: 60.0,
                    range: 3.5,
                    max_angle: 30.0,
                    charge_duration: Duration::from_millis(1200),
                    swing_duration: Duration::from_millis(100),
                    recover_duration: Duration::from_millis(500),
                },
                LeapMelee {
                    energy_cost: 0,
                    buildup_duration: Duration::from_millis(200),
                    movement_duration: Duration::from_millis(650),
                    swing_duration: Duration::from_millis(150),
                    recover_duration: Duration::from_millis(100),
                    base_damage: (240.0 * self.base_power()) as u32,
                    knockback: 25.0,
                    range: 4.5,
                    max_angle: 360.0,
                    forward_leap_strength: 28.0,
                    vertical_leap_strength: 8.0,
                },
            ],
            Farming(_) => vec![BasicMelee {
                energy_cost: 1,
                buildup_duration: Duration::from_millis(700),
                recover_duration: Duration::from_millis(150),
                base_healthchange: (-50.0 * self.base_power()) as i32,
                knockback: 0.0,
                range: 3.5,
                max_angle: 20.0,
            }],
            Bow(_) => vec![
                BasicRanged {
                    energy_cost: 0,
                    holdable: true,
                    prepare_duration: Duration::from_millis(100),
                    recover_duration: Duration::from_millis(400),
                    projectile: Projectile {
                        hit_solid: vec![projectile::Effect::Stick],
                        hit_entity: vec![
                            projectile::Effect::Damage((-40.0 * self.base_power()) as i32),
                            projectile::Effect::Knockback(10.0),
                            projectile::Effect::RewardEnergy(50),
                            projectile::Effect::Vanish,
                        ],
                        time_left: Duration::from_secs(15),
                        owner: None,
                        ignore_group: true,
                    },
                    projectile_body: Body::Object(object::Body::Arrow),
                    projectile_light: None,
                    projectile_gravity: Some(Gravity(0.2)),
                    projectile_speed: 100.0,
                },
                ChargedRanged {
                    energy_cost: 0,
                    energy_drain: 300,
                    initial_damage: (40.0 * self.base_power()) as u32,
                    max_damage: (200.0 * self.base_power()) as u32,
                    initial_knockback: 10.0,
                    max_knockback: 20.0,
                    prepare_duration: Duration::from_millis(100),
                    charge_duration: Duration::from_millis(1500),
                    recover_duration: Duration::from_millis(500),
                    projectile_body: Body::Object(object::Body::MultiArrow),
                    projectile_light: None,
                    projectile_gravity: Some(Gravity(0.2)),
                    initial_projectile_speed: 100.0,
                    max_projectile_speed: 500.0,
                },
                RepeaterRanged {
                    energy_cost: 0,
                    movement_duration: Duration::from_millis(200),
                    buildup_duration: Duration::from_millis(100),
                    shoot_duration: Duration::from_millis(100),
                    recover_duration: Duration::from_millis(500),
                    leap: Some(10.0),
                    projectile: Projectile {
                        hit_solid: vec![projectile::Effect::Stick],
                        hit_entity: vec![
                            projectile::Effect::Damage((-40.0 * self.base_power()) as i32),
                            projectile::Effect::Knockback(10.0),
                            projectile::Effect::RewardEnergy(50),
                            projectile::Effect::Vanish,
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
            Dagger(_) => vec![BasicMelee {
                energy_cost: 0,
                buildup_duration: Duration::from_millis(100),
                recover_duration: Duration::from_millis(400),
                base_healthchange: (-50.0 * self.base_power()) as i32,
                knockback: 0.0,
                range: 3.5,
                max_angle: 20.0,
            }],
            Sceptre(_) => vec![
                BasicBeam {
                    energy_cost: 0,
                    buildup_duration: Duration::from_millis(250),
                    recover_duration: Duration::from_millis(250),
                    beam_duration: Duration::from_secs(1),
                    base_hps: (60.0 * self.base_power()) as u32,
                    base_dps: (40.0 * self.base_power()) as u32,
                    tick_rate: 2.0,
                    range: 25.0,
                    max_angle: 1.0,
                    lifesteal_eff: 0.25,
                    energy_regen: 50,
                    energy_drain: 100,
                },
                BasicRanged {
                    energy_cost: 800,
                    holdable: true,
                    prepare_duration: Duration::from_millis(800),
                    recover_duration: Duration::from_millis(50),
                    projectile: Projectile {
                        hit_solid: vec![
                            projectile::Effect::Explode {
                                power: 1.4 * self.base_power(),
                                percent_damage: 0.2,
                            },
                            projectile::Effect::Vanish,
                        ],
                        hit_entity: vec![
                            projectile::Effect::Explode {
                                power: 1.4 * self.base_power(),
                                percent_damage: 0.2,
                            },
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
                },
            ],
            Staff(_) => vec![
                BasicMelee {
                    energy_cost: 0,
                    buildup_duration: Duration::from_millis(100),
                    recover_duration: Duration::from_millis(300),
                    base_healthchange: (-40.0 * self.base_power()) as i32,
                    knockback: 0.0,
                    range: 3.5,
                    max_angle: 20.0,
                },
                BasicRanged {
                    energy_cost: 0,
                    holdable: false,
                    prepare_duration: Duration::from_millis(250),
                    recover_duration: Duration::from_millis(600),
                    projectile: Projectile {
                        hit_solid: vec![projectile::Effect::Vanish],
                        hit_entity: vec![
                            projectile::Effect::Damage((-40.0 * self.base_power()) as i32),
                            projectile::Effect::RewardEnergy(150),
                            projectile::Effect::Vanish,
                        ],
                        time_left: Duration::from_secs(20),
                        owner: None,
                        ignore_group: true,
                    },
                    projectile_body: Body::Object(object::Body::BoltFire),
                    projectile_light: Some(LightEmitter {
                        col: (0.85, 0.5, 0.11).into(),
                        ..Default::default()
                    }),
                    projectile_gravity: None,
                    projectile_speed: 100.0,
                },
                BasicRanged {
                    energy_cost: 400,
                    holdable: true,
                    prepare_duration: Duration::from_millis(800),
                    recover_duration: Duration::from_millis(50),
                    projectile: Projectile {
                        hit_solid: vec![
                            projectile::Effect::Explode {
                                power: 1.4 * self.base_power(),
                                percent_damage: 1.0,
                            },
                            projectile::Effect::Vanish,
                        ],
                        hit_entity: vec![
                            projectile::Effect::Explode {
                                power: 1.4 * self.base_power(),
                                percent_damage: 1.0,
                            },
                            projectile::Effect::Vanish,
                        ],
                        time_left: Duration::from_secs(20),
                        owner: None,
                        ignore_group: true,
                    },
                    projectile_body: Body::Object(object::Body::BoltFireBig),
                    projectile_light: Some(LightEmitter {
                        col: (1.0, 0.75, 0.11).into(),
                        ..Default::default()
                    }),
                    projectile_gravity: None,
                    projectile_speed: 100.0,
                },
            ],
            Shield(_) => vec![
                BasicMelee {
                    energy_cost: 0,
                    buildup_duration: Duration::from_millis(100),
                    recover_duration: Duration::from_millis(400),
                    base_healthchange: (-40.0 * self.base_power()) as i32,
                    knockback: 0.0,
                    range: 3.0,
                    max_angle: 120.0,
                },
                BasicBlock,
            ],
            NpcWeapon(kind) => {
                if kind == "StoneGolemsFist" {
                    vec![
                        BasicMelee {
                            energy_cost: 0,
                            buildup_duration: Duration::from_millis(500),
                            recover_duration: Duration::from_millis(250),
                            knockback: 25.0,
                            base_healthchange: -200,
                            range: 5.0,
                            max_angle: 120.0,
                        },
                        GroundShockwave {
                            energy_cost: 0,
                            buildup_duration: Duration::from_millis(500),
                            recover_duration: Duration::from_millis(1000),
                            damage: 500,
                            knockback: -40.0,
                            shockwave_angle: 90.0,
                            shockwave_speed: 20.0,
                            shockwave_duration: Duration::from_millis(2000),
                            requires_ground: true,
                        },
                    ]
                } else {
                    vec![BasicMelee {
                        energy_cost: 0,
                        buildup_duration: Duration::from_millis(100),
                        recover_duration: Duration::from_millis(300),
                        base_healthchange: -10,
                        knockback: 0.0,
                        range: 1.0,
                        max_angle: 30.0,
                    }]
                }
            },
            Debug(kind) => {
                if kind == "Boost" {
                    vec![
                        CharacterAbility::Boost {
                            duration: Duration::from_millis(50),
                            only_up: false,
                        },
                        CharacterAbility::Boost {
                            duration: Duration::from_millis(50),
                            only_up: true,
                        },
                        BasicRanged {
                            energy_cost: 0,
                            holdable: false,
                            prepare_duration: Duration::from_millis(0),
                            recover_duration: Duration::from_millis(10),
                            projectile: Projectile {
                                hit_solid: vec![projectile::Effect::Stick],
                                hit_entity: vec![
                                    projectile::Effect::Stick,
                                    projectile::Effect::Possess,
                                ],
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
                        },
                    ]
                } else {
                    vec![]
                }
            },
            Empty => vec![BasicMelee {
                energy_cost: 0,
                buildup_duration: Duration::from_millis(0),
                recover_duration: Duration::from_millis(1000),
                base_healthchange: -20,
                knockback: 0.0,
                range: 3.5,
                max_angle: 15.0,
            }],
        }
    }

    /// Determines whether two tools are superficially equivalent to one another
    /// (i.e: one may be substituted for the other in crafting recipes or
    /// item possession checks).
    pub fn superficially_eq(&self, other: &Self) -> bool {
        ToolCategory::from(&self.kind) == ToolCategory::from(&other.kind)
    }
}
