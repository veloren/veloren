use rand::{seq::SliceRandom, thread_rng};
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
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Chest {
    Blue,
    Brown,
    Dark,
    Green,
    Orange,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Belt {
    Dark,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pants {
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
    Dark,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Shoulder {
    None,
    Brown1,
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
pub const ALL_BODY_TYPES: [BodyType; 2] = [BodyType::Female, BodyType::Male];
pub const ALL_CHESTS: [Chest; 5] = [
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
pub const ALL_PANTS: [Pants; 5] = [
    Pants::Blue,
    Pants::Brown,
    Pants::Dark,
    Pants::Green,
    Pants::Orange,
];
pub const ALL_HANDS: [Hand; 1] = [Hand::Default];
pub const ALL_FEET: [Foot; 1] = [Foot::Dark];
pub const ALL_WEAPONS: [Weapon; 7] = [
    Weapon::Daggers,
    Weapon::SwordShield,
    Weapon::Sword,
    Weapon::Axe,
    Weapon::Hammer,
    Weapon::Bow,
    Weapon::Staff,
];
pub const ALL_SHOULDERS: [Shoulder; 2] = [Shoulder::None, Shoulder::Brown1];
pub const ALL_DRAW: [Draw; 1] = [Draw::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HumanoidBody {
    pub race: Race,
    pub body_type: BodyType,
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
        let mut rng = thread_rng();
        Self {
            race: *(&ALL_RACES).choose(&mut rng).unwrap(),
            body_type: *(&ALL_BODY_TYPES).choose(&mut rng).unwrap(),
            chest: *(&ALL_CHESTS).choose(&mut rng).unwrap(),
            belt: *(&ALL_BELTS).choose(&mut rng).unwrap(),
            pants: *(&ALL_PANTS).choose(&mut rng).unwrap(),
            hand: *(&ALL_HANDS).choose(&mut rng).unwrap(),
            foot: *(&ALL_FEET).choose(&mut rng).unwrap(),
            weapon: *(&ALL_WEAPONS).choose(&mut rng).unwrap(),
            shoulder: *(&ALL_SHOULDERS).choose(&mut rng).unwrap(),
            draw: *(&ALL_DRAW).choose(&mut rng).unwrap(),
        }
    }
}
///////////
const ALL_QBODY_TYPES: [BodyType; 2] = [BodyType::Female, BodyType::Male];
const ALL_QPIG_HEADS: [PigHead; 1] = [PigHead::Default];
const ALL_QPIG_CHESTS: [PigChest; 1] = [PigChest::Default];
const ALL_QPIG_LEG_LS: [PigLegL; 1] = [PigLegL::Default];
const ALL_QPIG_LEG_RS: [PigLegR; 1] = [PigLegR::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QuadrupedBody {
    pub body_type: BodyType,
    pub pig_head: PigHead,
    pub pig_chest: PigChest,
    pub pig_leg_l: PigLegL,
    pub pig_leg_r: PigLegR,
}

impl QuadrupedBody {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        Self {
            body_type: *(&ALL_QBODY_TYPES).choose(&mut rng).unwrap(),
            pig_head: *(&ALL_QPIG_HEADS).choose(&mut rng).unwrap(),
            pig_chest: *(&ALL_QPIG_CHESTS).choose(&mut rng).unwrap(),
            pig_leg_l: *(&ALL_QPIG_LEG_LS).choose(&mut rng).unwrap(),
            pig_leg_r: *(&ALL_QPIG_LEG_RS).choose(&mut rng).unwrap(),
        }
    }
}
/////////////
const ALL_QMBODY_TYPES: [BodyType; 2] = [BodyType::Female, BodyType::Male];
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
        let mut rng = thread_rng();
        Self {
            body_type: *(&ALL_QMBODY_TYPES).choose(&mut rng).unwrap(),
            wolf_head_upper: *(&ALL_QMWOLF_HEADS_UPPER).choose(&mut rng).unwrap(),
            wolf_jaw: *(&ALL_QMWOLF_JAWS).choose(&mut rng).unwrap(),
            wolf_head_lower: *(&ALL_QMWOLF_HEADS_LOWER).choose(&mut rng).unwrap(),
            wolf_tail: *(&ALL_QMWOLF_TAILS).choose(&mut rng).unwrap(),
            wolf_torso_back: *(&ALL_QMWOLF_TORSOS_BACK).choose(&mut rng).unwrap(),
            wolf_torso_mid: *(&ALL_QMWOLF_TORSOS_MID).choose(&mut rng).unwrap(),
            wolf_ears: *(&ALL_QMWOLF_EARS).choose(&mut rng).unwrap(),
            wolf_foot_lf: *(&ALL_QMWOLF_FEET_LF).choose(&mut rng).unwrap(),
            wolf_foot_rf: *(&ALL_QMWOLF_FEET_RF).choose(&mut rng).unwrap(),
            wolf_foot_lb: *(&ALL_QMWOLF_FEET_LB).choose(&mut rng).unwrap(),
            wolf_foot_rb: *(&ALL_QMWOLF_FEET_RB).choose(&mut rng).unwrap(),
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
