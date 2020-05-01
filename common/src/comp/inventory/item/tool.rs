// Note: If you changes here "break" old character saves you can change the
// version in voxygen\src\meta.rs in order to reset save files to being empty

use crate::comp::{
    body::object, projectile, Body, CharacterAbility, Gravity, HealthChange, HealthSource,
    LightEmitter, Projectile,
};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SwordKind {
    BasicSword,
    Rapier,
    Zweihander0,
    WoodTraining,
    Short0,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AxeKind {
    BasicAxe,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HammerKind {
    BasicHammer,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BowKind {
    ShortBow0,
    ShortBow1,
    LongBow0,
    LongBow1,
    RareBow0,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DaggerKind {
    BasicDagger,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StaffKind {
    BasicStaff,
    Sceptre,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShieldKind {
    BasicShield,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FarmKind {
    Broom,
    Hoe0,
    Hoe1,
    Pitchfork,
    Rake,
    FishingRod0,
    FishingRod1,
    Pickaxe0,
    Shovel0,
    Shovel1,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DebugKind {
    Boost,
    Possess,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolKind {
    Sword(SwordKind),
    Axe(AxeKind),
    Hammer(HammerKind),
    Bow(BowKind),
    Dagger(DaggerKind),
    Staff(StaffKind),
    Shield(ShieldKind),
    Debug(DebugKind),
    Farming(FarmKind),
    /// This is an placeholder item, it is used by non-humanoid npcs to attack
    Empty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tool {
    pub kind: ToolKind,
    equip_time_millis: u32,
    // TODO: item specific abilities
}

impl Tool {
    pub fn empty() -> Self {
        Self {
            kind: ToolKind::Empty,
            equip_time_millis: 0,
        }
    }

    pub fn equip_time(&self) -> Duration { Duration::from_millis(self.equip_time_millis as u64) }

    pub fn get_abilities(&self) -> Vec<CharacterAbility> {
        use CharacterAbility::*;
        use DebugKind::*;
        use ToolKind::*;

        match self.kind {
            Sword(_) => vec![
                TripleStrike {
                    base_damage: 5,
                    needs_timing: false,
                },
                DashMelee {
                    buildup_duration: Duration::from_millis(500),
                    recover_duration: Duration::from_millis(500),
                    base_damage: 10,
                },
            ],
            Axe(_) => vec![
                TripleStrike {
                    base_damage: 7,
                    needs_timing: true,
                },
                BasicMelee {
                    energy_cost: 100,
                    buildup_duration: Duration::from_millis(700),
                    recover_duration: Duration::from_millis(100),
                    base_healthchange: -12,
                    range: 3.5,
                    max_angle: 30.0,
                },
            ],
            Hammer(_) => vec![BasicMelee {
                energy_cost: 0,
                buildup_duration: Duration::from_millis(700),
                recover_duration: Duration::from_millis(300),
                base_healthchange: -10,
                range: 3.5,
                max_angle: 60.0,
            }],
            Farming(_) => vec![BasicMelee {
                energy_cost: 1,
                buildup_duration: Duration::from_millis(700),
                recover_duration: Duration::from_millis(150),
                base_healthchange: -5,
                range: 3.0,
                max_angle: 60.0,
            }],
            Bow(_) => vec![BasicRanged {
                energy_cost: 0,
                holdable: true,
                prepare_duration: Duration::from_millis(100),
                recover_duration: Duration::from_millis(500),
                projectile: Projectile {
                    hit_solid: vec![projectile::Effect::Stick],
                    hit_entity: vec![
                        projectile::Effect::Damage(HealthChange {
                            // TODO: This should not be fixed (?)
                            amount: -5,
                            cause: HealthSource::Projectile { owner: None },
                        }),
                        projectile::Effect::Knockback(10.0),
                        projectile::Effect::RewardEnergy(100),
                        projectile::Effect::Vanish,
                    ],
                    time_left: Duration::from_secs(15),
                    owner: None,
                },
                projectile_body: Body::Object(object::Body::Arrow),
                projectile_light: None,
                projectile_gravity: Some(Gravity(0.1)),
            }],
            Dagger(_) => vec![BasicMelee {
                energy_cost: 0,
                buildup_duration: Duration::from_millis(100),
                recover_duration: Duration::from_millis(400),
                base_healthchange: -5,
                range: 3.5,
                max_angle: 60.0,
            }],
            Staff(StaffKind::BasicStaff) => vec![
                BasicMelee {
                    energy_cost: 0,
                    buildup_duration: Duration::from_millis(0),
                    recover_duration: Duration::from_millis(300),
                    base_healthchange: -1,
                    range: 10.0,
                    max_angle: 45.0,
                },
                BasicRanged {
                    energy_cost: 0,
                    holdable: false,
                    prepare_duration: Duration::from_millis(0),
                    recover_duration: Duration::from_millis(200),
                    projectile: Projectile {
                        hit_solid: vec![projectile::Effect::Vanish],
                        hit_entity: vec![
                            projectile::Effect::Damage(HealthChange {
                                // TODO: This should not be fixed (?)
                                amount: -1,
                                cause: HealthSource::Projectile { owner: None },
                            }),
                            projectile::Effect::RewardEnergy(100),
                            projectile::Effect::Vanish,
                        ],
                        time_left: Duration::from_secs(20),
                        owner: None,
                    },
                    projectile_body: Body::Object(object::Body::BoltFire),
                    projectile_light: Some(LightEmitter {
                        col: (0.72, 0.11, 0.11).into(),
                        ..Default::default()
                    }),

                    projectile_gravity: None,
                },
                BasicRanged {
                    energy_cost: 400,
                    holdable: false,
                    prepare_duration: Duration::from_millis(800),
                    recover_duration: Duration::from_millis(50),
                    projectile: Projectile {
                        hit_solid: vec![
                            projectile::Effect::Explode { power: 1.4 },
                            projectile::Effect::Vanish,
                        ],
                        hit_entity: vec![
                            projectile::Effect::Explode { power: 1.4 },
                            projectile::Effect::Vanish,
                        ],
                        time_left: Duration::from_secs(20),
                        owner: None,
                    },
                    projectile_body: Body::Object(object::Body::BoltFireBig),
                    projectile_light: Some(LightEmitter {
                        col: (0.72, 0.11, 0.11).into(),
                        ..Default::default()
                    }),

                    projectile_gravity: None,
                },
            ],
            Staff(StaffKind::Sceptre) => vec![
                BasicMelee {
                    energy_cost: 0,
                    buildup_duration: Duration::from_millis(0),
                    recover_duration: Duration::from_millis(300),
                    base_healthchange: -1,
                    range: 10.0,
                    max_angle: 45.0,
                },
                BasicMelee {
                    energy_cost: 350,
                    buildup_duration: Duration::from_millis(0),
                    recover_duration: Duration::from_millis(1000),
                    base_healthchange: 15,
                    range: 10.0,
                    max_angle: 45.0,
                },
            ],
            Shield(_) => vec![BasicBlock],
            Debug(kind) => match kind {
                DebugKind::Boost => vec![
                    CharacterAbility::Boost {
                        duration: Duration::from_millis(50),
                        only_up: false,
                    },
                    CharacterAbility::Boost {
                        duration: Duration::from_millis(50),
                        only_up: true,
                    },
                ],
                Possess => vec![BasicRanged {
                    energy_cost: 0,
                    holdable: false,
                    prepare_duration: Duration::from_millis(0),
                    recover_duration: Duration::from_millis(300),
                    projectile: Projectile {
                        hit_solid: vec![projectile::Effect::Stick],
                        hit_entity: vec![projectile::Effect::Stick, projectile::Effect::Possess],
                        time_left: Duration::from_secs(10),
                        owner: None,
                    },
                    projectile_body: Body::Object(object::Body::ArrowSnake),
                    projectile_light: None,
                    projectile_gravity: None,
                }],
            },
            Empty => vec![],
        }
    }
}
