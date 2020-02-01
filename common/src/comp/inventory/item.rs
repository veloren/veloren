use crate::{
    assets::{self, Asset},
    effect::Effect,
    terrain::{Block, BlockKind},
};
//use rand::prelude::*;
use rand::seq::SliceRandom;
use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;
use std::{fs::File, io::BufReader, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tool {
    Sword,
    Axe,
    Hammer,
    Bow,
    Dagger,
    Staff,
    Shield,
    Debug(Debug),
}

// TODO: Allow override in item ron?
impl Tool {
    pub fn wield_duration(&self) -> Duration {
        match self {
            Tool::Sword => Duration::from_millis(800),
            Tool::Axe => Duration::from_millis(1000),
            Tool::Hammer => Duration::from_millis(1000),
            Tool::Bow => Duration::from_millis(800),
            Tool::Dagger => Duration::from_millis(300),
            Tool::Staff => Duration::from_millis(800),
            Tool::Shield => Duration::from_millis(1000),
            Tool::Debug(_) => Duration::from_millis(0),
        }
    }

    pub fn attack_buildup_duration(&self) -> Duration {
        match self {
            Tool::Sword => Duration::from_millis(100),
            Tool::Axe => Duration::from_millis(700),
            Tool::Hammer => Duration::from_millis(700),
            Tool::Bow => Duration::from_millis(0),
            Tool::Dagger => Duration::from_millis(100),
            Tool::Staff => Duration::from_millis(400),
            Tool::Shield => Duration::from_millis(100),
            Tool::Debug(_) => Duration::from_millis(0),
        }
    }

    pub fn attack_recover_duration(&self) -> Duration {
        match self {
            Tool::Sword => Duration::from_millis(500),
            Tool::Axe => Duration::from_millis(100),
            Tool::Hammer => Duration::from_millis(100),
            Tool::Bow => Duration::from_millis(800),
            Tool::Dagger => Duration::from_millis(400),
            Tool::Staff => Duration::from_millis(300),
            Tool::Shield => Duration::from_millis(1000),
            Tool::Debug(_) => Duration::from_millis(0),
        }
    }

    pub fn attack_duration(&self) -> Duration {
        self.attack_buildup_duration() + self.attack_recover_duration()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Debug {
    Boost,
    Possess,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Armor {
    // TODO: Don't make armor be a body part. Wearing enemy's head is funny but also a creepy
    // thing to do.
    Helmet,
    Shoulders,
    Chestplate,
    Belt,
    Gloves,
    Pants,
    Boots,
    Back,
    Tabard,
    Gem,
    Necklace,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Consumable {
    Apple,
    Cheese,
    Potion,
    Mushroom,
    Velorite,
    VeloriteFrag,
    PotionMinor,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Utility {
    Collar,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Ingredient {
    Flower,
    Grass,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemKind {
    Tool { kind: Tool, power: u32 },
    Armor { kind: Armor, power: u32 },
    Consumable { kind: Consumable, effect: Effect },
    Utility { kind: Utility },
    Ingredient(Ingredient),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Item {
    name: String,
    description: String,
    pub kind: ItemKind,
}

impl Asset for Item {
    const ENDINGS: &'static [&'static str] = &["ron"];

    fn parse(buf_reader: BufReader<File>) -> Result<Self, assets::Error> {
        Ok(ron::de::from_reader(buf_reader).unwrap())
    }
}

impl Item {
    pub fn name(&self) -> &str { &self.name }

    pub fn description(&self) -> &str { &self.description }

    pub fn try_reclaim_from_block(block: Block) -> Option<Self> {
        match block.kind() {
            BlockKind::Apple => Some(assets::load_expect_cloned("common.items.apple")),
            BlockKind::Mushroom => Some(assets::load_expect_cloned("common.items.mushroom")),
            BlockKind::Velorite => Some(assets::load_expect_cloned("common.items.velorite")),
            BlockKind::BlueFlower => Some(assets::load_expect_cloned("common.items.flowers.blue")),
            BlockKind::PinkFlower => Some(assets::load_expect_cloned("common.items.flowers.pink")),
            BlockKind::PurpleFlower => {
                Some(assets::load_expect_cloned("common.items.flowers.purple"))
            },
            BlockKind::RedFlower => Some(assets::load_expect_cloned("common.items.flowers.red")),
            BlockKind::WhiteFlower => {
                Some(assets::load_expect_cloned("common.items.flowers.white"))
            },
            BlockKind::YellowFlower => {
                Some(assets::load_expect_cloned("common.items.flowers.yellow"))
            },
            BlockKind::Sunflower => Some(assets::load_expect_cloned("common.items.flowers.sun")),
            BlockKind::LongGrass => Some(assets::load_expect_cloned("common.items.grasses.long")),
            BlockKind::MediumGrass => {
                Some(assets::load_expect_cloned("common.items.grasses.medium"))
            },
            BlockKind::ShortGrass => Some(assets::load_expect_cloned("common.items.grasses.short")),
            BlockKind::Chest => Some(assets::load_expect_cloned(
                [
                    "common.items.apple",
                    "common.items.velorite",
                    "common.items.veloritefrag",
                    "common.items.cheese",
                    "common.items.potion_minor",
                    "common.items.collar",
                    "common.items.weapons.starter_sword",
                    "common.items.weapons.starter_axe",
                    "common.items.weapons.starter_hammer",
                    "common.items.weapons.starter_bow",
                    "common.items.weapons.starter_staff",
                ]
                .choose(&mut rand::thread_rng())
                .unwrap(), // Can't fail
            )),
            _ => None,
        }
    }
}

impl Component for Item {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
