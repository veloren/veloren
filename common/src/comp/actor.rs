use rand::prelude::*;
use specs::{Component, FlaggedStorage, VecStorage};

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
    Blue,
    Brown,
    Dark,
    Green,
    Orange,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Belt {
    //Default,
    Dark,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pants {
    Default,
    Blue,
    Brown,
    Dark,
    Green,
    Orange,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Hand {
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Foot {
    Default,
    Dark,
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
/////
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WolfHeadUpper {
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WolfJaw {
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WolfHeadLower {
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WolfTail {
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WolfTorsoBack {
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WolfTorsoMid {
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WolfEars {
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WolfFootLF {
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WolfFootRF {
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WolfFootLB {
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WolfFootRB {
    Default,
}

pub const ALL_RACES: [Race; 6] = [
    Race::Danari,
    Race::Dwarf,
    Race::Elf,
    Race::Human,
    Race::Orc,
    Race::Undead,
];
pub const ALL_BODY_TYPES: [BodyType; 3] = [BodyType::Female, BodyType::Male, BodyType::Unspecified];
pub const ALL_HEADS: [Head; 1] = [Head::Default];
pub const ALL_CHESTS: [Chest; 6] = [
    Chest::Default,
    Chest::Blue,
    Chest::Brown,
    Chest::Dark,
    Chest::Green,
    Chest::Orange,
];
pub const ALL_BELTS: [Belt; 1] = [
    //Belt::Default,
    Belt::Dark,
];
pub const ALL_PANTS: [Pants; 6] = [
    Pants::Default,
    Pants::Blue,
    Pants::Brown,
    Pants::Dark,
    Pants::Green,
    Pants::Orange,
];
pub const ALL_HANDS: [Hand; 1] = [Hand::Default];
pub const ALL_FEET: [Foot; 2] = [Foot::Default, Foot::Dark];
pub const ALL_WEAPONS: [Weapon; 7] = [
    Weapon::Daggers,
    Weapon::SwordShield,
    Weapon::Sword,
    Weapon::Axe,
    Weapon::Hammer,
    Weapon::Bow,
    Weapon::Staff,
];
pub const ALL_SHOULDERS: [Shoulder; 1] = [Shoulder::Default];
pub const ALL_DRAW: [Draw; 1] = [Draw::Default];

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
///////////
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
/////////////
const ALL_QMRACES: [Race; 6] = [
    Race::Danari,
    Race::Dwarf,
    Race::Elf,
    Race::Human,
    Race::Orc,
    Race::Undead,
];
const ALL_QMBODY_TYPES: [BodyType; 3] = [BodyType::Female, BodyType::Male, BodyType::Unspecified];
const ALL_QMWOLF_HEADS_UPPER: [WolfHeadUpper; 1] = [WolfHeadUpper::Default];
const ALL_QMWOLF_JAWS: [WolfJaw; 1] = [WolfJaw::Default];
const ALL_QMWOLF_HEADS_LOWER: [WolfHeadLower; 1] = [WolfHeadLower::Default];
const ALL_QMWOLF_TAILS: [WolfTail; 1] = [WolfTail::Default];
const ALL_QMWOLF_TORSOS_BACK: [WolfTorsoBack; 1] = [WolfTorsoBack::Default];
const ALL_QMWOLF_TORSOS_MID: [WolfTorsoMid; 1] = [WolfTorsoMid::Default];
const ALL_QMWOLF_EARS: [WolfEars; 1] = [WolfEars::Default];
const ALL_QMWOLF_FEET_LF: [WolfFootLF; 1] = [WolfFootLF::Default];
const ALL_QMWOLF_FEET_RF: [WolfFootRF; 1] = [WolfFootRF::Default];
const ALL_QMWOLF_FEET_LB: [WolfFootLB; 1] = [WolfFootLB::Default];
const ALL_QMWOLF_FEET_RB: [WolfFootRB; 1] = [WolfFootRB::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QuadrupedMediumBody {
    pub race: Race,
    pub body_type: BodyType,
    pub wolf_head_upper: WolfHeadUpper,
    pub wolf_jaw: WolfJaw,
    pub wolf_head_lower: WolfHeadLower,
    pub wolf_tail: WolfTail,
    pub wolf_torso_back: WolfTorsoBack,
    pub wolf_torso_mid: WolfTorsoMid,
    pub wolf_ears: WolfEars,
    pub wolf_foot_lf: WolfFootLF,
    pub wolf_foot_rf: WolfFootRF,
    pub wolf_foot_lb: WolfFootLB,
    pub wolf_foot_rb: WolfFootRB,
}

impl QuadrupedMediumBody {
    pub fn random() -> Self {
        Self {
            race: *thread_rng().choose(&ALL_QMRACES).unwrap(),
            body_type: *thread_rng().choose(&ALL_QMBODY_TYPES).unwrap(),
            wolf_head_upper: *thread_rng().choose(&ALL_QMWOLF_HEADS_UPPER).unwrap(),
            wolf_jaw: *thread_rng().choose(&ALL_QMWOLF_JAWS).unwrap(),
            wolf_head_lower: *thread_rng().choose(&ALL_QMWOLF_HEADS_LOWER).unwrap(),
            wolf_tail: *thread_rng().choose(&ALL_QMWOLF_TAILS).unwrap(),
            wolf_torso_back: *thread_rng().choose(&ALL_QMWOLF_TORSOS_BACK).unwrap(),
            wolf_torso_mid: *thread_rng().choose(&ALL_QMWOLF_TORSOS_MID).unwrap(),
            wolf_ears: *thread_rng().choose(&ALL_QMWOLF_EARS).unwrap(),
            wolf_foot_lf: *thread_rng().choose(&ALL_QMWOLF_FEET_LF).unwrap(),
            wolf_foot_rf: *thread_rng().choose(&ALL_QMWOLF_FEET_RF).unwrap(),
            wolf_foot_lb: *thread_rng().choose(&ALL_QMWOLF_FEET_LB).unwrap(),
            wolf_foot_rb: *thread_rng().choose(&ALL_QMWOLF_FEET_RB).unwrap(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Body {
    Humanoid(HumanoidBody),
    Quadruped(QuadrupedBody),
    QuadrupedMedium(QuadrupedMediumBody),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Actor {
    Character { name: String, body: Body },
}

impl Component for Actor {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}
