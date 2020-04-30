pub mod armor;
pub mod tool;

// Reexports
pub use tool::{DebugKind, SwordKind, Tool, ToolKind};

use crate::{
    assets::{self, Asset},
    effect::Effect,
    terrain::{Block, BlockKind},
};
use rand::seq::SliceRandom;
use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;
use std::{fs::File, io::BufReader};

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
pub enum Lantern {
    Black0 = 1,
    Green0 = 2,
}
pub const ALL_LANTERNS: [Lantern; 2] = [Lantern::Black0, Lantern::Green0];

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
            BlockKind::Coconut => Some(assets::load_expect_cloned("common.items.coconut")),
            BlockKind::Chest => Some(assets::load_expect_cloned(
                [
                    "common.items.apple",
                    "common.items.velorite",
                    "common.items.veloritefrag",
                    "common.items.cheese",
                    "common.items.potion_minor",
                    "common.items.collar",
                    "common.items.weapons.sword.starter_sword",
                    "common.items.weapons.axe.starter_axe",
                    "common.items.weapons.staff.staff_nature",
                    "common.items.weapons.hammer.starter_hammer",
                    "common.items.weapons.bow.starter_bow",
                    "common.items.weapons.staff.starter_staff",
                    "common.items.armor.belt.plate_0",
                    "common.items.armor.belt.leather_0",
                    "common.items.armor.chest.plate_green_0",
                    "common.items.armor.chest.leather_0",
                    "common.items.armor.foot.plate_0",
                    "common.items.armor.foot.leather_0",
                    "common.items.armor.pants.plate_green_0",
                    "common.items.armor.belt.leather_0",
                    "common.items.armor.shoulder.plate_0",
                    "common.items.armor.shoulder.leather_1",
                    "common.items.armor.shoulder.leather_0",
                    "common.items.armor.hand.leather_0",
                    "common.items.armor.hand.plate_0",
                    "common.items.weapons.sword.wood_sword",
                    "common.items.weapons.sword.short_sword_0",
                    "common.items.armor.belt.cloth_blue_0",
                    "common.items.armor.chest.cloth_blue_0",
                    "common.items.armor.foot.cloth_blue_0",
                    "common.items.armor.pants.cloth_blue_0",
                    "common.items.armor.shoulder.cloth_blue_0",
                    "common.items.armor.hand.cloth_blue_0",
                    "common.items.armor.belt.cloth_green_0",
                    "common.items.armor.chest.cloth_green_0",
                    "common.items.armor.foot.cloth_green_0",
                    "common.items.armor.pants.cloth_green_0",
                    "common.items.armor.shoulder.cloth_green_0",
                    "common.items.armor.hand.cloth_green_0",
                    "common.items.armor.belt.cloth_purple_0",
                    "common.items.armor.chest.cloth_purple_0",
                    "common.items.armor.foot.cloth_purple_0",
                    "common.items.armor.pants.cloth_purple_0",
                    "common.items.armor.shoulder.cloth_purple_0",
                    "common.items.armor.hand.cloth_purple_0",
                    "common.items.armor.ring.ring_0",
                    "common.items.armor.back.short_0",
                    "common.items.armor.neck.neck_0",
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
