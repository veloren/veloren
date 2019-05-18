use crate::inventory::Inventory;
use rand::prelude::*;
use specs::{Component, FlaggedStorage, VecStorage};
use vek::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Race {
    Danari,
    Dwarf,
    Elf,
    Human,
    Orc,
    Undead,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BodyType {
    Female,
    Male,
    Unspecified,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Head {
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Chest {
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Belt {
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pants {
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Hand {
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Foot {
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Shoulder {
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Weapon {
    Daggers,
    SwordShield,
    Sword,
    Axe,
    Hammer,
    Bow,
    Staff,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Draw {
    Default,
}
////
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PigHead {
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PigChest {
    Default,
}
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PigLegL {
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PigLegR {
    Default,
}

const ALL_RACES: [Race; 6] = [
    Race::Danari,
    Race::Dwarf,
    Race::Elf,
    Race::Human,
    Race::Orc,
    Race::Undead,
];
const ALL_BODY_TYPES: [BodyType; 3] = [BodyType::Female, BodyType::Male, BodyType::Unspecified];
const ALL_HEADS: [Head; 1] = [Head::Default];
const ALL_CHESTS: [Chest; 1] = [Chest::Default];
const ALL_BELTS: [Belt; 1] = [Belt::Default];
const ALL_PANTS: [Pants; 1] = [Pants::Default];
const ALL_HANDS: [Hand; 1] = [Hand::Default];
const ALL_FEET: [Foot; 1] = [Foot::Default];
const ALL_WEAPONS: [Weapon; 7] = [
    Weapon::Daggers,
    Weapon::SwordShield,
    Weapon::Sword,
    Weapon::Axe,
    Weapon::Hammer,
    Weapon::Bow,
    Weapon::Staff,
];
const ALL_SHOULDERS: [Shoulder; 1] = [Shoulder::Default];
const ALL_DRAW: [Draw; 1] = [Draw::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HumanoidBody {
    pub race: Race,
    pub body_type: BodyType,
    pub head: Head,
    pub chest: Chest,
    pub belt: Belt,
    pub pants: Pants,
    pub hand: Hand,
    pub foot: Foot,
    pub weapon: Weapon,
    pub shoulder: Shoulder,
    pub draw: Draw,
}

impl HumanoidBody {
    pub fn random() -> Self {
        Self {
            race: *thread_rng().choose(&ALL_RACES).unwrap(),
            body_type: *thread_rng().choose(&ALL_BODY_TYPES).unwrap(),
            head: *thread_rng().choose(&ALL_HEADS).unwrap(),
            chest: *thread_rng().choose(&ALL_CHESTS).unwrap(),
            belt: *thread_rng().choose(&ALL_BELTS).unwrap(),
            pants: *thread_rng().choose(&ALL_PANTS).unwrap(),
            hand: *thread_rng().choose(&ALL_HANDS).unwrap(),
            foot: *thread_rng().choose(&ALL_FEET).unwrap(),
            weapon: *thread_rng().choose(&ALL_WEAPONS).unwrap(),
            shoulder: *thread_rng().choose(&ALL_SHOULDERS).unwrap(),
            draw: *thread_rng().choose(&ALL_DRAW).unwrap(),
        }
    }
}
const ALL_QRACES: [Race; 6] = [
    Race::Danari,
    Race::Dwarf,
    Race::Elf,
    Race::Human,
    Race::Orc,
    Race::Undead,
];
const ALL_QBODY_TYPES: [BodyType; 3] = [BodyType::Female, BodyType::Male, BodyType::Unspecified];
const ALL_QPIG_HEADS: [PigHead; 1] = [PigHead::Default];
const ALL_QPIG_CHESTS: [PigChest; 1] = [PigChest::Default];
const ALL_QPIG_LEG_LS: [PigLegL; 1] = [PigLegL::Default];
const ALL_QPIG_LEG_RS: [PigLegR; 1] = [PigLegR::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QuadrupedBody {
    pub race: Race,
    pub body_type: BodyType,
    pub pig_head: PigHead,
    pub pig_chest: PigChest,
    pub pig_leg_l: PigLegL,
    pub pig_leg_r: PigLegR,
}

impl QuadrupedBody {
    pub fn random() -> Self {
        Self {
            race: *thread_rng().choose(&ALL_QRACES).unwrap(),
            body_type: *thread_rng().choose(&ALL_QBODY_TYPES).unwrap(),
            pig_head: *thread_rng().choose(&ALL_QPIG_HEADS).unwrap(),
            pig_chest: *thread_rng().choose(&ALL_QPIG_CHESTS).unwrap(),
            pig_leg_l: *thread_rng().choose(&ALL_QPIG_LEG_LS).unwrap(),
            pig_leg_r: *thread_rng().choose(&ALL_QPIG_LEG_RS).unwrap(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Body {
    Humanoid(HumanoidBody),
    Quadruped(QuadrupedBody),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Actor {
    Character { name: String, body: Body },
}

impl Component for Actor {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct AnimationHistory {
    pub last: Option<Animation>,
    pub current: Animation,
    pub time: f64,
}

impl AnimationHistory {
    pub fn new(animation: Animation) -> Self {
        Self {
            last: None,
            current: animation,
            time: 0.0,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Animation {
    Idle,
    Run,
    Jump,
    Gliding,
}

impl Component for AnimationHistory {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}
