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
pub enum Gender {
    Female,
    Male,
    Unspecified,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Head {
    DefaultHead,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Chest {
    DefaultChest,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Belt {
    DefaultBelt,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pants {
    DefaultPants,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Hand {
    DefaultHand,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Foot {
    DefaultFoot,
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
pub enum Shoulder {
    DefaultShoulder
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Draw {
    DefaultDraw
}


use Belt::*;
use Chest::*;
use Foot::*;
use Gender::*;
use Hand::*;
use Head::*;
use Pants::*;
use Race::*;
use Weapon::*;
use Shoulder::*;
use Draw::*;


const ALL_RACES: [Race; 6] = [Danari, Dwarf, Elf, Human, Orc, Undead];
const ALL_GENDERS: [Gender; 3] = [Female, Male, Unspecified];
const ALL_HEADS: [Head; 1] = [DefaultHead];
const ALL_CHESTS: [Chest; 1] = [DefaultChest];
const ALL_BELTS: [Belt; 1] = [DefaultBelt];
const ALL_PANTS: [Pants; 1] = [DefaultPants];
const ALL_HANDS: [Hand; 1] = [DefaultHand];
const ALL_FEET: [Foot; 1] = [DefaultFoot];
const ALL_WEAPONS: [Weapon; 7] = [Daggers, SwordShield, Sword, Axe, Hammer, Bow, Staff];
const ALL_SHOULDERS: [Shoulder; 1] = [DefaultShoulder];
const ALL_DRAW: [Draw; 1] = [DefaultDraw];



#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Character {
    pub race: Race,
    pub gender: Gender,
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

impl Character {
    pub fn random() -> Self {
        Self {
            race: *thread_rng().choose(&ALL_RACES).unwrap(),
            gender: *thread_rng().choose(&ALL_GENDERS).unwrap(),
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

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct AnimationHistory {
    pub last: Option<Animation>,
    pub current: Animation,
    pub time: f64,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Animation {
    Idle,
    Run,
    Jump,
}

impl Component for Character {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

impl Component for AnimationHistory {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}
