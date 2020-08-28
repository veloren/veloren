pub mod armor;
pub mod tool;

// Reexports
pub use tool::{Hands, Tool, ToolCategory, ToolKind};

use crate::{
    assets::{self, Asset, Ron},
    effect::Effect,
    lottery::Lottery,
    terrain::{Block, BlockKind},
};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use vek::Rgb;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Throwable {
    Bomb,
    TrainingDummy,
    Firework(Reagent),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Reagent {
    Blue,
    Green,
    Purple,
    Red,
    Yellow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Utility {
    Collar,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Lantern {
    pub kind: String,
    color: Rgb<u32>,
    strength_thousandths: u32,
    flicker_thousandths: u32,
}

impl Lantern {
    pub fn strength(&self) -> f32 { self.strength_thousandths as f32 / 1000_f32 }

    pub fn color(&self) -> Rgb<f32> { self.color.map(|c| c as f32 / 255.0) }
}

fn default_amount() -> u32 { 1 }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ItemKind {
    /// Something wieldable
    Tool(tool::Tool),
    Lantern(Lantern),
    Armor(armor::Armor),
    Consumable {
        kind: String,
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
        kind: String,
        #[serde(default = "default_amount")]
        amount: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    name: String,
    description: String,
    pub kind: ItemKind,
}

pub type ItemAsset = Ron<Item>;

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

    pub fn expect_from_asset(asset: &str) -> Self { (*ItemAsset::load_expect(asset)).clone() }

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

    pub fn amount(&self) -> u32 {
        match &self.kind {
            ItemKind::Tool(_) => 1,
            ItemKind::Lantern(_) => 1,
            ItemKind::Armor { .. } => 1,
            ItemKind::Consumable { amount, .. } => *amount,
            ItemKind::Throwable { amount, .. } => *amount,
            ItemKind::Utility { amount, .. } => *amount,
            ItemKind::Ingredient { amount, .. } => *amount,
        }
    }

    pub fn try_reclaim_from_block(block: Block) -> Option<Self> {
        let chosen;
        let mut rng = rand::thread_rng();
        Some(ItemAsset::load_expect_cloned(match block.kind() {
            BlockKind::Apple => "common.items.food.apple",
            BlockKind::Mushroom => "common.items.food.mushroom",
            BlockKind::Velorite => "common.items.ore.velorite",
            BlockKind::VeloriteFrag => "common.items.ore.veloritefrag",
            BlockKind::BlueFlower => "common.items.flowers.blue",
            BlockKind::PinkFlower => "common.items.flowers.pink",
            BlockKind::PurpleFlower => "common.items.flowers.purple",
            BlockKind::RedFlower => "common.items.flowers.red",
            BlockKind::WhiteFlower => "common.items.flowers.white",
            BlockKind::YellowFlower => "common.items.flowers.yellow",
            BlockKind::Sunflower => "common.items.flowers.sun",
            BlockKind::LongGrass => "common.items.grasses.long",
            BlockKind::MediumGrass => "common.items.grasses.medium",
            BlockKind::ShortGrass => "common.items.grasses.short",
            BlockKind::Coconut => "common.items.food.coconut",
            BlockKind::Chest => {
                chosen = Lottery::<String>::load_expect(match rng.gen_range(0, 7) {
                    0 => "common.loot_tables.loot_table_weapon_uncommon",
                    1 => "common.loot_tables.loot_table_weapon_common",
                    2 => "common.loot_tables.loot_table_armor_light",
                    3 => "common.loot_tables.loot_table_armor_cloth",
                    4 => "common.loot_tables.loot_table_armor_heavy",
                    _ => "common.loot_tables.loot_table_armor_misc",
                });
                chosen.choose()
            },
            BlockKind::Crate => {
                chosen = Lottery::<String>::load_expect("common.loot_tables.loot_table_food");
                chosen.choose()
            },
            BlockKind::Stones => "common.items.crafting_ing.stones",
            BlockKind::Twigs => "common.items.crafting_ing.twigs",
            BlockKind::ShinyGem => "common.items.crafting_ing.shiny_gem",
            _ => return None,
        }))
    }

    /// Determines whether two items are superficially equivalent to one another
    /// (i.e: one may be substituted for the other in crafting recipes or
    /// item possession checks).
    pub fn superficially_eq(&self, other: &Self) -> bool {
        match (&self.kind, &other.kind) {
            (ItemKind::Tool(a), ItemKind::Tool(b)) => a.superficially_eq(b),
            // TODO: Differentiate between lantern colors?
            (ItemKind::Lantern(_), ItemKind::Lantern(_)) => true,
            (ItemKind::Armor(a), ItemKind::Armor(b)) => a.superficially_eq(b),
            (ItemKind::Consumable { kind: a, .. }, ItemKind::Consumable { kind: b, .. }) => a == b,
            (ItemKind::Throwable { kind: a, .. }, ItemKind::Throwable { kind: b, .. }) => a == b,
            (ItemKind::Utility { kind: a, .. }, ItemKind::Utility { kind: b, .. }) => a == b,
            (ItemKind::Ingredient { kind: a, .. }, ItemKind::Ingredient { kind: b, .. }) => a == b,
            _ => false,
        }
    }
}

impl Component for Item {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemDrop(pub Item);

impl Component for ItemDrop {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
