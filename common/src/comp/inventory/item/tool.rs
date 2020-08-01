// Note: If you changes here "break" old character saves you can change the
// version in voxygen\src\meta.rs in order to reset save files to being empty

use crate::comp::{
    body::object, projectile, Body, CharacterAbility, Gravity, LightEmitter, Projectile,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SwordKind {
    BasicSword,
    Rapier,
    Zweihander0,
    WoodTraining,
    Short0,
    GreatswordDam0,
    GreatswordDam1,
    GreatswordDam2,
    GreatswordSimple0,
    GreatswordSimple1,
    GreatswordSimple2,
    GreatswordOrn0,
    GreatswordOrn1,
    GreatswordOrn2,
    GreatswordFine0,
    GreatswordFine1,
    GreatswordFine2,
    LongDam0,
    LongDam1,
    LongDam2,
    LongDam3,
    LongDam4,
    LongDam5,
    LongSimple0,
    LongSimple1,
    LongSimple2,
    LongSimple3,
    LongSimple4,
    LongSimple5,
    LongOrn0,
    LongOrn1,
    LongOrn2,
    LongOrn3,
    LongOrn4,
    LongOrn5,
    LongFine0,
    LongFine1,
    LongFine2,
    LongFine3,
    LongFine4,
    LongFine5,
    CultPurp0,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AxeKind {
    BasicAxe,
    OrcAxe0,
    WornIronAxe0,
    WornIronAxe1,
    WornIronAxe2,
    WornIronAxe3,
    WornIronAxe4,
    BronzeAxe0,
    BronzeAxe1,
    IronAxe0,
    IronAxe1,
    IronAxe2,
    IronAxe3,
    IronAxe4,
    IronAxe5,
    IronAxe6,
    IronAxe7,
    IronAxe8,
    IronAxe9,
    SteelAxe0,
    SteelAxe1,
    SteelAxe2,
    SteelAxe3,
    SteelAxe4,
    SteelAxe5,
    SteelAxe6,
    BloodsteelAxe0,
    BloodsteelAxe1,
    BloodsteelAxe2,
    CobaltAxe0,
    MalachiteAxe0,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HammerKind {
    BasicHammer,
    FlimsyHammer,
    WoodHammer0,
    StoneHammer0,
    StoneHammer1,
    StoneHammer2,
    StoneHammer3,
    WornIronHammer0,
    WornIronHammer1,
    WornIronHammer2,
    WornIronHammer3,
    BronzeHammer0,
    BronzeHammer1,
    IronHammer0,
    IronHammer1,
    IronHammer2,
    IronHammer3,
    IronHammer4,
    IronHammer5,
    IronHammer6,
    IronHammer7,
    IronHammer8,
    SteelHammer0,
    SteelHammer1,
    SteelHammer2,
    SteelHammer3,
    SteelHammer4,
    SteelHammer5,
    CobaltHammer0,
    CobaltHammer1,
    RunicHammer,
    RamsheadHammer,
    Mjolnir,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BowKind {
    ShortBow0,
    WoodShortbow0,
    WoodShortbow1,
    LeafyShortbow0,
    WoodLongbow0,
    WoodLongbow1,
    LeafyLongbow0,
    HornLongbow0,
    IronLongbow0,
    RareLongbow,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DaggerKind {
    BasicDagger,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StaffKind {
    Sceptre,
    BasicStaff,
    BoneStaff,
    AmethystStaff,
    CultistStaff,
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

impl ToolKind {
    pub fn into_hands(self) -> Hands {
        match self {
            ToolKind::Sword(_) => Hands::TwoHand,
            ToolKind::Axe(_) => Hands::TwoHand,
            ToolKind::Hammer(_) => Hands::TwoHand,
            ToolKind::Bow(_) => Hands::TwoHand,
            ToolKind::Dagger(_) => Hands::OneHand,
            ToolKind::Staff(_) => Hands::TwoHand,
            ToolKind::Shield(_) => Hands::OneHand,
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
    Shield,
    Debug,
    Farming,
    Empty,
}

impl From<ToolKind> for ToolCategory {
    fn from(kind: ToolKind) -> ToolCategory {
        match kind {
            ToolKind::Sword(_) => ToolCategory::Sword,
            ToolKind::Axe(_) => ToolCategory::Axe,
            ToolKind::Hammer(_) => ToolCategory::Hammer,
            ToolKind::Bow(_) => ToolCategory::Bow,
            ToolKind::Dagger(_) => ToolCategory::Dagger,
            ToolKind::Staff(_) => ToolCategory::Staff,
            ToolKind::Shield(_) => ToolCategory::Shield,
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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
        //use DebugKind::*;
        use ToolKind::*;

        match self.kind {
            Sword(_) => vec![
                TripleStrike {
                    base_damage: (60.0 * self.base_power()) as u32,
                    needs_timing: false,
                },
                DashMelee {
                    energy_cost: 700,
                    buildup_duration: Duration::from_millis(500),
                    recover_duration: Duration::from_millis(500),
                    base_damage: (120.0 * self.base_power()) as u32,
                },
            ],
            Axe(_) => vec![
                TripleStrike {
                    base_damage: (80.0 * self.base_power()) as u32,
                    needs_timing: true,
                },
                SpinMelee {
                    energy_cost: 100,
                    buildup_duration: Duration::from_millis(125),
                    recover_duration: Duration::from_millis(125),
                    base_damage: (60.0 * self.base_power()) as u32,
                },
            ],
            Hammer(_) => vec![
                BasicMelee {
                    energy_cost: 0,
                    buildup_duration: Duration::from_millis(700),
                    recover_duration: Duration::from_millis(300),
                    base_healthchange: (-120.0 * self.base_power()) as i32,
                    range: 3.5,
                    max_angle: 60.0,
                },
                LeapMelee {
                    energy_cost: 800,
                    movement_duration: Duration::from_millis(500),
                    buildup_duration: Duration::from_millis(1000),
                    recover_duration: Duration::from_millis(100),
                    base_damage: (240.0 * self.base_power()) as u32,
                },
            ],
            Farming(_) => vec![BasicMelee {
                energy_cost: 1,
                buildup_duration: Duration::from_millis(700),
                recover_duration: Duration::from_millis(150),
                base_healthchange: (-50.0 * self.base_power()) as i32,
                range: 3.0,
                max_angle: 60.0,
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
                            projectile::Effect::RewardEnergy(100),
                            projectile::Effect::Vanish,
                        ],
                        time_left: Duration::from_secs(15),
                        owner: None,
                    },
                    projectile_body: Body::Object(object::Body::Arrow),
                    projectile_light: None,
                    projectile_gravity: Some(Gravity(0.2)),
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
                    projectile_body: Body::Object(object::Body::Arrow),
                    projectile_light: None,
                },
            ],
            Dagger(_) => vec![
                BasicMelee {
                    energy_cost: 0,
                    buildup_duration: Duration::from_millis(100),
                    recover_duration: Duration::from_millis(400),
                    base_healthchange: (-50.0 * self.base_power()) as i32,
                    range: 3.5,
                    max_angle: 60.0,
                },
                DashMelee {
                    energy_cost: 700,
                    buildup_duration: Duration::from_millis(500),
                    recover_duration: Duration::from_millis(500),
                    base_damage: (100.0 * self.base_power()) as u32,
                },
            ],
            Staff(StaffKind::Sceptre) => vec![
                BasicMelee {
                    energy_cost: 0,
                    buildup_duration: Duration::from_millis(0),
                    recover_duration: Duration::from_millis(300),
                    base_healthchange: (-10.0 * self.base_power()) as i32,
                    range: 10.0,
                    max_angle: 45.0,
                },
                BasicMelee {
                    energy_cost: 350,
                    buildup_duration: Duration::from_millis(0),
                    recover_duration: Duration::from_millis(1000),
                    base_healthchange: (150.0 * self.base_power()) as i32,
                    range: 10.0,
                    max_angle: 45.0,
                },
            ],
            Staff(_) => vec![
                BasicMelee {
                    energy_cost: 0,
                    buildup_duration: Duration::from_millis(100),
                    recover_duration: Duration::from_millis(300),
                    base_healthchange: (-40.0 * self.base_power()) as i32,
                    range: 10.0,
                    max_angle: 45.0,
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
                    },
                    projectile_body: Body::Object(object::Body::BoltFire),
                    projectile_light: Some(LightEmitter {
                        col: (0.85, 0.5, 0.11).into(),
                        ..Default::default()
                    }),

                    projectile_gravity: None,
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
                            },
                            projectile::Effect::Vanish,
                        ],
                        hit_entity: vec![
                            projectile::Effect::Explode {
                                power: 1.4 * self.base_power(),
                            },
                            projectile::Effect::Vanish,
                        ],
                        time_left: Duration::from_secs(20),
                        owner: None,
                    },
                    projectile_body: Body::Object(object::Body::BoltFireBig),
                    projectile_light: Some(LightEmitter {
                        col: (1.0, 0.75, 0.11).into(),
                        ..Default::default()
                    }),

                    projectile_gravity: None,
                },
            ],
            Shield(_) => vec![
                BasicMelee {
                    energy_cost: 0,
                    buildup_duration: Duration::from_millis(100),
                    recover_duration: Duration::from_millis(400),
                    base_healthchange: (-40.0 * self.base_power()) as i32,
                    range: 3.0,
                    max_angle: 120.0,
                },
                BasicBlock,
            ],
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
                        },
                        projectile_body: Body::Object(object::Body::ArrowSnake),
                        projectile_light: Some(LightEmitter {
                            col: (0.0, 1.0, 0.33).into(),
                            ..Default::default()
                        }),
                        projectile_gravity: None,
                    },
                ],
            },
            Empty => vec![BasicMelee {
                energy_cost: 0,
                buildup_duration: Duration::from_millis(0),
                recover_duration: Duration::from_millis(1000),
                base_healthchange: -20,
                range: 5.0,
                max_angle: 60.0,
            }],
        }
    }

    /// Determines whether two tools are superficially equivalent to one another
    /// (i.e: one may be substituted for the other in crafting recipes or
    /// item possession checks).
    pub fn superficially_eq(&self, other: &Self) -> bool {
        ToolCategory::from(self.kind) == ToolCategory::from(other.kind)
    }
}
