pub mod armor;
pub mod tool;

// Reexports
pub use tool::{Hands, Tool, ToolCategory, ToolKind};

use crate::{
    assets::{self, Asset, Error},
    effect::Effect,
    lottery::Lottery,
    terrain::{Block, SpriteKind},
};
use crossbeam::atomic::AtomicCell;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use std::{
    fs::File,
    io::BufReader,
    num::{NonZeroU32, NonZeroU64},
    sync::Arc,
};
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

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Glider {
    pub kind: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Copy)]
pub enum Quality {
    Low,       // Grey
    Common,    // UI Main Color
    Moderate,  // Green
    High,      // Blue
    Epic,      // Purple
    Legendary, // Gold
    Artifact,  // Orange
    Debug,     // Red
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ItemKind {
    /// Something wieldable
    Tool(tool::Tool),
    Lantern(Lantern),
    Armor(armor::Armor),
    Glider(Glider),
    Consumable {
        kind: String,
        effect: Vec<Effect>,
    },
    Throwable {
        kind: Throwable,
    },
    Utility {
        kind: Utility,
    },
    Ingredient {
        kind: String,
    },
}

pub type ItemId = AtomicCell<Option<NonZeroU64>>;

/* /// The only way to access an item id outside this module is to mutably, atomically update it using
/// this structure.  It has a single method, `try_assign_id`, which attempts to set the id if and
/// only if it's not already set.
pub struct CreateDatabaseItemId {
    item_id: Arc<ItemId>,
}

pub struct CreateDatabaseItemId {
    item_id: Arc<ItemId>,
} */

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    /// item_id is hidden because it represents the persistent, storage entity
    /// ID for any item that has been saved to the database.  Additionally,
    /// it (currently) holds interior mutable state, making it very
    /// dangerous to expose.  We will work to eliminate this issue soon; for
    /// now, we try to make the system as foolproof as possible by greatly
    /// restricting opportunities for cloning the item_id.
    #[serde(skip)]
    item_id: Arc<ItemId>,
    /// item_def is hidden because changing the item definition for an item
    /// could change invariants like whether it was stackable (invalidating
    /// the amount).
    item_def: Arc<ItemDef>,
    /// amount is hidden because it needs to maintain the invariant that only
    /// stackable items can have > 1 amounts.
    amount: NonZeroU32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemDef {
    #[serde(default)]
    item_definition_id: String,
    pub name: String,
    pub description: String,
    pub kind: ItemKind,
    pub quality: Quality,
}

impl PartialEq for ItemDef {
    fn eq(&self, other: &Self) -> bool { self.item_definition_id == other.item_definition_id }
}

impl ItemDef {
    pub fn is_stackable(&self) -> bool {
        matches!(self.kind, ItemKind::Consumable { .. }
            | ItemKind::Ingredient { .. }
            | ItemKind::Throwable { .. }
            | ItemKind::Utility { .. })
    }
}

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        self.item_def.item_definition_id == other.item_def.item_definition_id
    }
}

impl Asset for ItemDef {
    const ENDINGS: &'static [&'static str] = &["ron"];

    fn parse(buf_reader: BufReader<File>, specifier: &str) -> Result<Self, assets::Error> {
        let item: Result<Self, Error> =
            ron::de::from_reader(buf_reader).map_err(Error::parse_error);

        // Some commands like /give_item provide the asset specifier separated with \
        // instead of .
        let specifier = specifier.replace('\\', ".");

        item.map(|item| ItemDef {
            item_definition_id: specifier,
            ..item
        })
    }
}

impl Item {
    // TODO: consider alternatives such as default abilities that can be added to a
    // loadout when no weapon is present
    pub fn empty() -> Self { Item::new(ItemDef::load_expect("common.items.weapons.empty.empty")) }

    pub fn new(inner_item: Arc<ItemDef>) -> Self {
        Item {
            item_id: Arc::new(AtomicCell::new(None)),
            item_def: inner_item,
            amount: NonZeroU32::new(1).unwrap(),
        }
    }

    /// Creates a new instance of an `Item` from the provided asset identifier
    /// Panics if the asset does not exist.
    pub fn new_from_asset_expect(asset_specifier: &str) -> Self {
        let inner_item = ItemDef::load_expect(asset_specifier);
        Item::new(inner_item)
    }

    /// Creates a Vec containing one of each item that matches the provided
    /// asset glob pattern
    pub fn new_from_asset_glob(asset_glob: &str) -> Result<Vec<Self>, Error> {
        let items = ItemDef::load_glob(asset_glob)?;

        let result = items
            .iter()
            .map(|item_def| Item::new(Arc::clone(item_def)))
            .collect::<Vec<_>>();

        Ok(result)
    }

    /// Creates a new instance of an `Item from the provided asset identifier if
    /// it exists
    pub fn new_from_asset(asset: &str) -> Result<Self, Error> {
        let inner_item = ItemDef::load(asset)?;
        Ok(Item::new(inner_item))
    }

    /// Duplicates an item, creating an exact copy but with a new item ID
    pub fn duplicate(&self) -> Self { Item::new(Arc::clone(&self.item_def)) }

    /// FIXME: HACK: In order to set the entity ID asynchronously, we currently
    /// start it at None, and then atomically set it when it's saved for the
    /// first time in the database.  Because this requires shared mutable
    /// state if these aren't synchronized by the program structure,
    /// currently we use an Atomic inside an Arc; this is clearly very
    /// dangerous, so in the future we will hopefully have a better way of
    /// dealing with this.
    #[doc(hidden)]
    pub fn get_item_id_for_database(&self) -> Arc<ItemId> { Arc::clone(&self.item_id) }

    /// Resets the item's item ID to None, giving it a new identity. Used when
    /// dropping items into the world so that a new database record is
    /// created when they are picked up again.
    ///
    /// NOTE: The creation of a new `Arc` when resetting the item ID is critical
    /// because every time a new `Item` instance is created, it is cloned from
    /// a single asset which results in an `Arc` pointing to the same value in
    /// memory. Therefore, every time an item instance is created this
    /// method must be called in order to give it a unique identity.
    fn reset_item_id(&mut self) {
        if let Some(item_id) = Arc::get_mut(&mut self.item_id) {
            *item_id = AtomicCell::new(None);
        } else {
            self.item_id = Arc::new(AtomicCell::new(None));
        }
    }

    /// Removes the unique identity of an item - used when dropping an item on
    /// the floor. In the future this will need to be changed if we want to
    /// maintain a unique ID for an item even when it's dropped and picked
    /// up by another player.
    pub fn put_in_world(&mut self) { self.reset_item_id() }

    pub fn increase_amount(&mut self, increase_by: u32) -> Result<(), assets::Error> {
        let amount = u32::from(self.amount);
        self.amount = amount
            .checked_add(increase_by)
            .and_then(NonZeroU32::new)
            .ok_or(assets::Error::InvalidType)?;
        Ok(())
    }

    pub fn decrease_amount(&mut self, decrease_by: u32) -> Result<(), assets::Error> {
        let amount = u32::from(self.amount);
        self.amount = amount
            .checked_sub(decrease_by)
            .and_then(NonZeroU32::new)
            .ok_or(assets::Error::InvalidType)?;
        Ok(())
    }

    pub fn set_amount(&mut self, give_amount: u32) -> Result<(), assets::Error> {
        if give_amount == 1 || self.item_def.is_stackable() {
            self.amount = NonZeroU32::new(give_amount).ok_or(assets::Error::InvalidType)?;
            Ok(())
        } else {
            Err(assets::Error::InvalidType)
        }
    }

    pub fn item_definition_id(&self) -> &str { &self.item_def.item_definition_id }

    pub fn is_same_item_def(&self, item_def: &ItemDef) -> bool {
        self.item_def.item_definition_id == item_def.item_definition_id
    }

    pub fn is_stackable(&self) -> bool { self.item_def.is_stackable() }

    pub fn name(&self) -> &str { &self.item_def.name }

    pub fn description(&self) -> &str { &self.item_def.description }

    pub fn kind(&self) -> &ItemKind { &self.item_def.kind }

    pub fn amount(&self) -> u32 { u32::from(self.amount) }

    pub fn quality(&self) -> Quality { self.item_def.quality }

    pub fn try_reclaim_from_block(block: Block) -> Option<Self> {
        let chosen;
        let mut rng = rand::thread_rng();
        Some(Item::new_from_asset_expect(match block.get_sprite()? {
            SpriteKind::Apple => "common.items.food.apple",
            SpriteKind::Mushroom => "common.items.food.mushroom",
            SpriteKind::Velorite => "common.items.ore.velorite",
            SpriteKind::VeloriteFrag => "common.items.ore.veloritefrag",
            SpriteKind::BlueFlower => "common.items.flowers.blue",
            SpriteKind::PinkFlower => "common.items.flowers.pink",
            SpriteKind::PurpleFlower => "common.items.flowers.purple",
            SpriteKind::RedFlower => "common.items.flowers.red",
            SpriteKind::WhiteFlower => "common.items.flowers.white",
            SpriteKind::YellowFlower => "common.items.flowers.yellow",
            SpriteKind::Sunflower => "common.items.flowers.sunflower",
            SpriteKind::LongGrass => "common.items.grasses.long",
            SpriteKind::MediumGrass => "common.items.grasses.medium",
            SpriteKind::ShortGrass => "common.items.grasses.short",
            SpriteKind::Coconut => "common.items.food.coconut",
            SpriteKind::Chest => {
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
            SpriteKind::Crate => {
                chosen = Lottery::<String>::load_expect(match rng.gen_range(0, 4) {
                    0 => "common.loot_tables.loot_table_crafting",
                    _ => "common.loot_tables.loot_table_food",
                });
                chosen.choose()
            },
            SpriteKind::Beehive => "common.items.crafting_ing.honey",
            SpriteKind::Stones => "common.items.crafting_ing.stones",
            SpriteKind::Twigs => "common.items.crafting_ing.twigs",
            SpriteKind::ShinyGem => "common.items.crafting_ing.shiny_gem",
            SpriteKind::VialEmpty => "common.items.crafting_ing.empty_vial",
            SpriteKind::PotionMinor => "common.items.consumable.potion_minor",
            _ => return None,
        }))
    }
}

/// Provides common methods providing details about an item definition
/// for either an `Item` containing the definition, or the actual `ItemDef`
pub trait ItemDesc {
    fn description(&self) -> &str;
    fn name(&self) -> &str;
    fn kind(&self) -> &ItemKind;
    fn quality(&self) -> &Quality;
}

impl ItemDesc for Item {
    fn description(&self) -> &str { &self.item_def.description }

    fn name(&self) -> &str { &self.item_def.name }

    fn kind(&self) -> &ItemKind { &self.item_def.kind }

    fn quality(&self) -> &Quality { &self.item_def.quality }
}

impl ItemDesc for ItemDef {
    fn description(&self) -> &str { &self.description }

    fn name(&self) -> &str { &self.name }

    fn kind(&self) -> &ItemKind { &self.kind }

    fn quality(&self) -> &Quality { &self.quality }
}

impl Component for Item {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemDrop(pub Item);

impl Component for ItemDrop {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
