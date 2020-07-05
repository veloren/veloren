pub mod armor;
pub mod lottery;
pub mod tool;

// Reexports
pub use tool::{DebugKind, Hands, SwordKind, Tool, ToolCategory, ToolKind};

use crate::{
    assets::{self, Asset},
    effect::Effect,
    terrain::{Block, BlockKind},
};
use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;
use std::{fs::File, io::BufReader};
use vek::Rgb;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Consumable {
    Coconut,
    Apple,
    Cheese,
    Potion,
    Mushroom,
    Velorite,
    VeloriteFrag,
    PotionMinor,
    PotionExp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Throwable {
    Bomb,
    TrainingDummy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Utility {
    Collar,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Ingredient {
    Flower,
    Grass,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum LanternKind {
    Black0 = 1,
    Green0 = 2,
    Red0 = 3,
    Blue0 = 4,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Lantern {
    pub kind: LanternKind,
    color: Rgb<u32>,
    strength_thousandths: u32,
    flicker_thousandths: u32,
}

pub const ALL_LANTERNS: [LanternKind; 4] = [
    LanternKind::Black0,
    LanternKind::Green0,
    LanternKind::Red0,
    LanternKind::Blue0,
];

impl Lantern {
    pub fn strength(&self) -> f32 { self.strength_thousandths as f32 / 1000_f32 }

    pub fn color(&self) -> Rgb<f32> { self.color.map(|c| c as f32 / 255.0) }
}

fn default_amount() -> u32 { 1 }

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemKind {
    /// Something wieldable
    Tool(tool::Tool),
    Lantern(Lantern),
    Armor {
        kind: armor::Armor,
        stats: armor::Stats,
    },
    Consumable {
        kind: Consumable,
        effect: Effect,
        #[serde(default = "default_amount")]
        amount: u32,
    },
    Throwable {
        kind: Throwable,
        #[serde(default = "default_amount")]
        amount: u32,
    },
    Utility {
        kind: Utility,
        #[serde(default = "default_amount")]
        amount: u32,
    },
    Ingredient {
        kind: Ingredient,
        #[serde(default = "default_amount")]
        amount: u32,
    },
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
        ron::de::from_reader(buf_reader).map_err(assets::Error::parse_error)
    }
}

impl Item {
    // TODO: consider alternatives such as default abilities that can be added to a
    // loadout when no weapon is present
    pub fn empty() -> Self {
        Self {
            name: "Empty Item".to_owned(),
            description: "This item may grant abilities, but is invisible".to_owned(),
            kind: ItemKind::Tool(Tool::empty()),
        }
    }

    pub fn expect_from_asset(asset: &str) -> Self { (*assets::load_expect::<Self>(asset)).clone() }

    pub fn set_amount(&mut self, give_amount: u32) -> Result<(), assets::Error> {
        use ItemKind::*;
        match self.kind {
            Consumable { ref mut amount, .. }
            | Throwable { ref mut amount, .. }
            | Utility { ref mut amount, .. }
            | Ingredient { ref mut amount, .. } => {
                *amount = give_amount;
                Ok(())
            },
            Tool { .. } | Lantern { .. } | Armor { .. } => {
                // Tools and armor don't stack
                Err(assets::Error::InvalidType)
            },
        }
    }

    pub fn name(&self) -> &str { &self.name }

    pub fn description(&self) -> &str { &self.description }

    pub fn try_reclaim_from_block(block: Block) -> Option<Self> {
        match block.kind() {
            BlockKind::Apple => Some(assets::load_expect_cloned("common.items.apple")),
            BlockKind::Mushroom => Some(assets::load_expect_cloned("common.items.mushroom")),
            BlockKind::Velorite => Some(assets::load_expect_cloned("common.items.velorite")),
            BlockKind::VeloriteFrag => {
                Some(assets::load_expect_cloned("common.items.veloritefrag"))
            },
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
            BlockKind::Coconut => Some(assets::load_expect_cloned("common.items.coconut")),
            BlockKind::Chest => {
                let chosen = assets::load_expect::<lottery::Lottery<_>>("common.loot_table");
                let chosen = chosen.choose();

                Some(assets::load_expect_cloned(chosen))
            },
            _ => None,
        }
    }
}

impl Component for Item {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemDrop(pub Item);

impl Component for ItemDrop {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
