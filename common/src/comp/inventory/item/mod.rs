pub mod armor;
pub mod modular;
pub mod tool;

// Reexports
pub use modular::{ModularComponent, ModularComponentKind, ModularComponentTag};
pub use tool::{AbilitySet, AbilitySpec, Hands, MaterialStatManifest, Tool, ToolKind};

use crate::{
    assets::{self, AssetExt, Error},
    comp::{
        inventory::{item::tool::AbilityMap, InvSlot},
        CharacterAbility,
    },
    effect::Effect,
    lottery::{LootSpec, Lottery},
    recipe::RecipeInput,
    terrain::{Block, SpriteKind},
};
use core::{
    convert::TryFrom,
    mem,
    num::{NonZeroU32, NonZeroU64},
    ops::Deref,
};
use crossbeam_utils::atomic::AtomicCell;
use serde::{de, Deserialize, Serialize, Serializer};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;
use std::{fmt, sync::Arc};
use tracing::error;
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
    White,
    Yellow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Utility {
    Coins,
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
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy, PartialOrd, Ord)]
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

pub trait TagExampleInfo {
    fn name(&self) -> &'static str;
    /// What item to show in the crafting hud if the player has nothing with the
    /// tag
    fn exemplar_identifier(&self) -> &'static str;
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ItemTag {
    ClothItem,
    LeatherItem,
    ModularComponent(ModularComponentTag),
    MetalIngot,
    Cultist,
    Potion,
    Food,
    BaseMaterial, // Cloth-scraps, Leather...
    CraftingTool, // Pickaxe, Craftsman-Hammer, Sewing-Set
    Utility,
    Bag,
}

impl TagExampleInfo for ItemTag {
    fn name(&self) -> &'static str {
        match self {
            ItemTag::ClothItem => "cloth item",
            ItemTag::LeatherItem => "leather item",
            ItemTag::ModularComponent(kind) => kind.name(),
            ItemTag::MetalIngot => "metal ingot",
            ItemTag::Cultist => "cultist",
            ItemTag::Potion => "potion",
            ItemTag::Food => "food",
            ItemTag::BaseMaterial => "basemat",
            ItemTag::CraftingTool => "tool",
            ItemTag::Utility => "utility",
            ItemTag::Bag => "bag",
        }
    }

    // TODO: Autogenerate these?
    fn exemplar_identifier(&self) -> &'static str {
        match self {
            ItemTag::ClothItem => "common.items.tag_examples.cloth_item",
            ItemTag::LeatherItem => "common.items.tag_examples.leather_item",
            ItemTag::ModularComponent(tag) => tag.exemplar_identifier(),
            ItemTag::MetalIngot => "common.items.tag_examples.metal_ingot",
            ItemTag::Cultist => "common.items.tag_examples.cultist",
            ItemTag::Potion => "common.items.tag_examples.placeholder",
            ItemTag::Food => "common.items.tag_examples.placeholder",
            ItemTag::BaseMaterial => "common.items.tag_examples.placeholder",
            ItemTag::CraftingTool => "common.items.tag_examples.placeholder",
            ItemTag::Utility => "common.items.tag_examples.placeholder",
            ItemTag::Bag => "common.items.tag_examples.placeholder",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ItemKind {
    /// Something wieldable
    Tool(tool::Tool),
    ModularComponent(modular::ModularComponent),
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
    TagExamples {
        /// A list of item names to lookup the appearences of and animate
        /// through
        item_ids: Vec<String>,
    },
}

impl ItemKind {
    pub fn is_equippable(&self) -> bool {
        matches!(
            self,
            ItemKind::Tool(_) | ItemKind::Armor { .. } | ItemKind::Glider(_) | ItemKind::Lantern(_)
        )
    }
}

pub type ItemId = AtomicCell<Option<NonZeroU64>>;

/* /// The only way to access an item id outside this module is to mutably, atomically update it using
/// this structure.  It has a single method, `try_assign_id`, which attempts to set the id if and
/// only if it's not already set.
pub struct CreateDatabaseItemId {
    item_id: Arc<ItemId>,
}*/

/// NOTE: Do not call `Item::clone` without consulting the core devs!  It only
/// exists due to being required for message serialization at the moment, and
/// should not be used for any other purpose.
///
/// FIXME: Turn on a Clippy lint forbidding the use of `Item::clone` using the
/// `disallowed_method` feature.
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
    #[serde(
        serialize_with = "serialize_item_def",
        deserialize_with = "deserialize_item_def"
    )]
    item_def: Arc<ItemDef>,
    /// components is hidden to maintain the following invariants:
    /// - It should only contain modular components (and enhancements, once they
    ///   exist)
    /// - Enhancements (once they exist) should be compatible with the available
    ///   slot shapes
    /// - Modular components should agree with the tool kind
    /// - There should be exactly one damage component and exactly one held
    ///   component for modular
    /// weapons
    components: Vec<Item>,
    /// amount is hidden because it needs to maintain the invariant that only
    /// stackable items can have > 1 amounts.
    amount: NonZeroU32,
    /// The slots for items that this item has
    slots: Vec<InvSlot>,
    item_config: Option<Box<ItemConfig>>,
}

// Custom serialization for ItemDef, we only want to send the item_definition_id
// over the network, the client will use deserialize_item_def to fetch the
// ItemDef from assets.
fn serialize_item_def<S: Serializer>(field: &Arc<ItemDef>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&field.item_definition_id)
}

// Custom de-serialization for ItemDef to retrieve the ItemDef from assets using
// its asset specifier (item_definition_id)
fn deserialize_item_def<'de, D>(deserializer: D) -> Result<Arc<ItemDef>, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct ItemDefStringVisitor;

    impl<'de> de::Visitor<'de> for ItemDefStringVisitor {
        type Value = Arc<ItemDef>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("item def string")
        }

        fn visit_str<E>(self, item_definition_id: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Arc::<ItemDef>::load_expect_cloned(item_definition_id))
        }
    }

    deserializer.deserialize_str(ItemDefStringVisitor)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemDef {
    #[serde(default)]
    item_definition_id: String,
    pub name: String,
    pub description: String,
    pub kind: ItemKind,
    pub quality: Quality,
    pub tags: Vec<ItemTag>,
    #[serde(default)]
    pub slots: u16,
    /// Used to specify a custom ability set for a weapon. Leave None (or don't
    /// include field in ItemDef) to use default ability set for weapon kind.
    pub ability_spec: Option<AbilitySpec>,
}

impl PartialEq for ItemDef {
    fn eq(&self, other: &Self) -> bool { self.item_definition_id == other.item_definition_id }
}

// TODO: Look into removing ItemConfig and just using AbilitySet
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemConfig {
    pub abilities: AbilitySet<CharacterAbility>,
}

#[derive(Debug)]
pub enum ItemConfigError {
    BadItemKind,
}

impl TryFrom<(&Item, &AbilityMap, &MaterialStatManifest)> for ItemConfig {
    type Error = ItemConfigError;

    fn try_from(
        (item, ability_map, msm): (&Item, &AbilityMap, &MaterialStatManifest),
    ) -> Result<Self, Self::Error> {
        if let ItemKind::Tool(tool) = &item.kind {
            // If no custom ability set is specified, fall back to abilityset of tool kind.
            let tool_default = ability_map
                .get_ability_set(&AbilitySpec::Tool(tool.kind))
                .cloned();
            let abilities = if let Some(set_key) = item.ability_spec() {
                if let Some(set) = ability_map.get_ability_set(set_key) {
                    set.clone().modified_by_tool(&tool, msm, &item.components)
                } else {
                    error!(
                        "Custom ability set: {:?} references non-existent set, falling back to \
                         default ability set.",
                        set_key
                    );
                    tool_default.unwrap_or_default()
                }
            } else if let Some(set) = tool_default {
                set.modified_by_tool(&tool, msm, &item.components)
            } else {
                error!(
                    "No ability set defined for tool: {:?}, falling back to default ability set.",
                    tool.kind
                );
                Default::default()
            };

            Ok(ItemConfig { abilities })
        } else {
            Err(ItemConfigError::BadItemKind)
        }
    }
}

impl ItemDef {
    pub fn is_stackable(&self) -> bool {
        matches!(
            self.kind,
            ItemKind::Consumable { .. }
                | ItemKind::Ingredient { .. }
                | ItemKind::Throwable { .. }
                | ItemKind::Utility { .. }
        )
    }

    pub fn is_modular(&self) -> bool {
        matches!(
            &self.kind,
            ItemKind::Tool(tool::Tool {
                stats: tool::StatKind::Modular,
                ..
            }) | ItemKind::ModularComponent(_)
        )
    }

    pub fn is_component(&self, kind: ModularComponentKind) -> bool {
        if let ItemKind::ModularComponent(ModularComponent { modkind, .. }) = self.kind {
            kind == modkind
        } else {
            false
        }
    }

    // currently needed by trade_pricing
    pub fn id(&self) -> &str { &self.item_definition_id }

    #[cfg(test)]
    pub fn new_test(
        item_definition_id: String,
        kind: ItemKind,
        quality: Quality,
        tags: Vec<ItemTag>,
        slots: u16,
    ) -> Self {
        Self {
            item_definition_id,
            name: "test item name".to_owned(),
            description: "test item description".to_owned(),
            kind,
            quality,
            tags,
            slots,
            ability_spec: None,
        }
    }
}

/// NOTE: This PartialEq instance is pretty broken!  It doesn't check item
/// amount or any child items (and, arguably, doing so should be able to ignore
/// things like item order within the main inventory or within each bag, and
/// possibly even coalesce amounts, though these may be more controversial).
/// Until such time as we find an actual need for a proper PartialEq instance,
/// please don't rely on this for anything!
impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        self.item_def.item_definition_id == other.item_def.item_definition_id
    }
}

impl Deref for Item {
    type Target = ItemDef;

    fn deref(&self) -> &Self::Target { &self.item_def }
}

impl assets::Compound for ItemDef {
    fn load<S: assets::source::Source>(
        cache: &assets::AssetCache<S>,
        specifier: &str,
    ) -> Result<Self, Error> {
        // load from the filesystem first, but if the file doesn't exist, see if it's a
        // programmaticly-generated asset
        let raw = cache
            .load_owned::<RawItemDef>(specifier)
            .or_else(|e| modular::synthesize_modular_asset(specifier).ok_or(e))?;

        let RawItemDef {
            name,
            description,
            kind,
            quality,
            tags,
            slots,
            ability_spec,
        } = raw;

        // Some commands like /give_item provide the asset specifier separated with \
        // instead of .
        //
        // TODO: This probably does not belong here
        let item_definition_id = specifier.replace('\\', ".");

        Ok(ItemDef {
            item_definition_id,
            name,
            description,
            kind,
            quality,
            tags,
            slots,
            ability_spec,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename = "ItemDef")]
struct RawItemDef {
    name: String,
    description: String,
    kind: ItemKind,
    quality: Quality,
    tags: Vec<ItemTag>,
    #[serde(default)]
    slots: u16,
    ability_spec: Option<AbilitySpec>,
}

impl assets::Asset for RawItemDef {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

#[derive(Debug)]
pub struct OperationFailure;

#[derive(Clone)]
struct ItemList(Vec<Item>);
impl assets::Compound for ItemList {
    fn load<S: assets::source::Source>(
        cache: &assets::AssetCache<S>,
        specifier: &str,
    ) -> Result<Self, Error> {
        let list = cache
            .load::<assets::Directory>(specifier)?
            .read()
            .iter()
            .map(|spec| Item::new_from_asset(spec))
            .collect::<Result<_, Error>>()?;

        Ok(ItemList(list))
    }
}

impl Item {
    // TODO: consider alternatives such as default abilities that can be added to a
    // loadout when no weapon is present
    pub fn empty() -> Self { Item::new_from_asset_expect("common.items.weapons.empty.empty") }

    pub fn new_from_item_def(
        inner_item: Arc<ItemDef>,
        input_components: &[Item],
        ability_map: &AbilityMap,
        msm: &MaterialStatManifest,
    ) -> Self {
        let mut components = Vec::new();
        if inner_item.is_modular() {
            // recipe ensures that types match (i.e. no axe heads on a sword hilt, or double
            // sword blades)
            components.extend(
                input_components
                    .iter()
                    .map(|comp| comp.duplicate(ability_map, msm)),
            );
        }

        let mut item = Item {
            item_id: Arc::new(AtomicCell::new(None)),
            amount: NonZeroU32::new(1).unwrap(),
            components,
            slots: vec![None; inner_item.slots as usize],
            item_def: inner_item,
            item_config: None,
        };
        item.update_item_config(ability_map, msm);
        item
    }

    /// Creates a new instance of an `Item` from the provided asset identifier
    /// Panics if the asset does not exist.
    pub fn new_from_asset_expect(asset_specifier: &str) -> Self {
        let inner_item = Arc::<ItemDef>::load_expect_cloned(asset_specifier);
        // TODO: Figure out better way to get msm and ability_map
        let msm = MaterialStatManifest::default();
        let ability_map = AbilityMap::default();
        Item::new_from_item_def(inner_item, &[], &ability_map, &msm)
    }

    /// Creates a Vec containing one of each item that matches the provided
    /// asset glob pattern
    pub fn new_from_asset_glob(asset_glob: &str) -> Result<Vec<Self>, Error> {
        Ok(ItemList::load_cloned(asset_glob)?.0)
    }

    /// Creates a new instance of an `Item from the provided asset identifier if
    /// it exists
    pub fn new_from_asset(asset: &str) -> Result<Self, Error> {
        let inner_item = Arc::<ItemDef>::load_cloned(asset)?;
        // TODO: Get msm and ability_map less hackily
        let msm = MaterialStatManifest::default();
        let ability_map = AbilityMap::default();
        Ok(Item::new_from_item_def(inner_item, &[], &ability_map, &msm))
    }

    /// Duplicates an item, creating an exact copy but with a new item ID
    pub fn duplicate(&self, ability_map: &AbilityMap, msm: &MaterialStatManifest) -> Self {
        let mut new_item = Item::new_from_item_def(
            Arc::clone(&self.item_def),
            &self.components,
            ability_map,
            msm,
        );
        new_item.set_amount(self.amount()).expect(
            "`new_item` has the same `item_def` and as an invariant, \
             self.set_amount(self.amount()) should always succeed.",
        );
        new_item.slots_mut().iter_mut().zip(self.slots()).for_each(
            |(new_item_slot, old_item_slot)| {
                *new_item_slot = old_item_slot
                    .as_ref()
                    .map(|old_item| old_item.duplicate(ability_map, msm));
            },
        );
        new_item
    }

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

    pub fn increase_amount(&mut self, increase_by: u32) -> Result<(), OperationFailure> {
        let amount = u32::from(self.amount);
        self.amount = amount
            .checked_add(increase_by)
            .filter(|&amount| amount <= self.max_amount())
            .and_then(NonZeroU32::new)
            .ok_or(OperationFailure)?;
        Ok(())
    }

    pub fn decrease_amount(&mut self, decrease_by: u32) -> Result<(), OperationFailure> {
        let amount = u32::from(self.amount);
        self.amount = amount
            .checked_sub(decrease_by)
            .and_then(NonZeroU32::new)
            .ok_or(OperationFailure)?;
        Ok(())
    }

    pub fn set_amount(&mut self, give_amount: u32) -> Result<(), OperationFailure> {
        if give_amount <= self.max_amount() {
            self.amount = NonZeroU32::new(give_amount).ok_or(OperationFailure)?;
            Ok(())
        } else {
            Err(OperationFailure)
        }
    }

    pub fn add_component(
        &mut self,
        component: Item,
        ability_map: &AbilityMap,
        msm: &MaterialStatManifest,
    ) {
        // TODO: hook for typechecking (not needed atm if this is only used by DB
        // persistence, but will definitely be needed once enhancement slots are
        // added to prevent putting a sword into another sword)
        self.components.push(component);
        // adding a component changes the stats, so recalculate the ItemConfig
        self.update_item_config(ability_map, msm);
    }

    fn update_item_config(&mut self, ability_map: &AbilityMap, msm: &MaterialStatManifest) {
        if let Ok(item_config) = ItemConfig::try_from((&*self, ability_map, msm)) {
            self.item_config = Some(Box::new(item_config));
        }
    }

    /// Returns an iterator that drains items contained within the item's slots
    pub fn drain(&mut self) -> impl Iterator<Item = Item> + '_ {
        self.slots.iter_mut().filter_map(|x| mem::take(x))
    }

    pub fn item_definition_id(&self) -> &str { &self.item_def.item_definition_id }

    pub fn is_same_item_def(&self, item_def: &ItemDef) -> bool {
        self.item_def.item_definition_id == item_def.item_definition_id
    }

    pub fn matches_recipe_input(&self, recipe_input: &RecipeInput) -> bool {
        match recipe_input {
            RecipeInput::Item(item_def) => self.is_same_item_def(item_def),
            RecipeInput::Tag(tag) => self.item_def.tags.contains(tag),
        }
    }

    pub fn name(&self) -> &str { &self.item_def.name }

    pub fn description(&self) -> &str { &self.item_def.description }

    pub fn kind(&self) -> &ItemKind { &self.item_def.kind }

    pub fn amount(&self) -> u32 { u32::from(self.amount) }

    /// NOTE: invariant that amount() ≤ max_amount(), 1 ≤ max_amount(),
    /// and if !self.is_stackable(), self.max_amount() = 1.
    pub fn max_amount(&self) -> u32 { if self.is_stackable() { u32::MAX } else { 1 } }

    pub fn quality(&self) -> Quality { self.item_def.quality }

    pub fn components(&self) -> &[Item] { &self.components }

    pub fn slots(&self) -> &[InvSlot] { &self.slots }

    pub fn slots_mut(&mut self) -> &mut [InvSlot] { &mut self.slots }

    pub fn item_config_expect(&self) -> &ItemConfig {
        &self
            .item_config
            .as_ref()
            .expect("Item was expected to have an ItemConfig")
    }

    pub fn free_slots(&self) -> usize { self.slots.iter().filter(|x| x.is_none()).count() }

    pub fn populated_slots(&self) -> usize { self.slots().len().saturating_sub(self.free_slots()) }

    pub fn slot(&self, slot: usize) -> Option<&InvSlot> { self.slots.get(slot) }

    pub fn slot_mut(&mut self, slot: usize) -> Option<&mut InvSlot> { self.slots.get_mut(slot) }

    pub fn try_reclaim_from_block(block: Block) -> Option<Self> {
        Some(Item::new_from_asset_expect(match block.get_sprite()? {
            SpriteKind::Apple => "common.items.food.apple",
            SpriteKind::Mushroom => "common.items.food.mushroom",
            SpriteKind::CaveMushroom => "common.items.food.mushroom",
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
            SpriteKind::Beehive => "common.items.crafting_ing.honey",
            SpriteKind::Stones => "common.items.crafting_ing.stones",
            SpriteKind::Twigs => "common.items.crafting_ing.twigs",
            SpriteKind::VialEmpty => "common.items.crafting_ing.empty_vial",
            SpriteKind::Bowl => "common.items.crafting_ing.bowl",
            SpriteKind::PotionMinor => "common.items.consumable.potion_minor",
            SpriteKind::Amethyst => "common.items.crafting_ing.amethyst",
            SpriteKind::Ruby => "common.items.crafting_ing.ruby",
            SpriteKind::Diamond => "common.items.crafting_ing.diamond",
            SpriteKind::Sapphire => "common.items.crafting_ing.sapphire",
            SpriteKind::Topaz => "common.items.crafting_ing.topaz",
            SpriteKind::Emerald => "common.items.crafting_ing.emerald",
            SpriteKind::AmethystSmall => "common.items.crafting_ing.amethyst",
            SpriteKind::TopazSmall => "common.items.crafting_ing.topaz",
            SpriteKind::DiamondSmall => "common.items.crafting_ing.diamond",
            SpriteKind::RubySmall => "common.items.crafting_ing.ruby",
            SpriteKind::EmeraldSmall => "common.items.crafting_ing.emerald",
            SpriteKind::SapphireSmall => "common.items.crafting_ing.sapphire",
            SpriteKind::Seashells => "common.items.crafting_ing.seashells",
            // Containers
            // IMPORTANT: Add any new container to `SpriteKind::is_container`
            container
            @
            (SpriteKind::DungeonChest0
            | SpriteKind::DungeonChest1
            | SpriteKind::DungeonChest2
            | SpriteKind::DungeonChest3
            | SpriteKind::DungeonChest4
            | SpriteKind::DungeonChest5
            | SpriteKind::Chest
            | SpriteKind::Mud
            | SpriteKind::Crate
            | SpriteKind::ChestBuried) => {
                return Item::from_container(container);
            },
            _ => return None,
        }))
    }

    fn from_container(container: SpriteKind) -> Option<Item> {
        let chosen;
        match container {
            SpriteKind::DungeonChest0 => {
                chosen =
                    Lottery::<LootSpec>::load_expect("common.loot_tables.dungeon.tier-0.chest")
                        .read();
                return Some(chosen.choose().to_item());
            },
            SpriteKind::DungeonChest1 => {
                chosen =
                    Lottery::<LootSpec>::load_expect("common.loot_tables.dungeon.tier-1.chest")
                        .read();
                return Some(chosen.choose().to_item());
            },
            SpriteKind::DungeonChest2 => {
                chosen =
                    Lottery::<LootSpec>::load_expect("common.loot_tables.dungeon.tier-2.chest")
                        .read();
                return Some(chosen.choose().to_item());
            },
            SpriteKind::DungeonChest3 => {
                chosen =
                    Lottery::<LootSpec>::load_expect("common.loot_tables.dungeon.tier-3.chest")
                        .read();
                return Some(chosen.choose().to_item());
            },
            SpriteKind::DungeonChest4 => {
                chosen =
                    Lottery::<LootSpec>::load_expect("common.loot_tables.dungeon.tier-4.chest")
                        .read();
                return Some(chosen.choose().to_item());
            },
            SpriteKind::DungeonChest5 => {
                chosen =
                    Lottery::<LootSpec>::load_expect("common.loot_tables.dungeon.tier-5.chest")
                        .read();
                return Some(chosen.choose().to_item());
            },
            SpriteKind::Chest => {
                chosen = Lottery::<LootSpec>::load_expect("common.loot_tables.sprite.chest").read();
                return Some(chosen.choose().to_item());
            },
            SpriteKind::ChestBuried => {
                chosen = Lottery::<LootSpec>::load_expect("common.loot_tables.sprite.chest-buried")
                    .read();
                return Some(chosen.choose().to_item());
            },
            SpriteKind::Mud => {
                chosen = Lottery::<LootSpec>::load_expect("common.loot_tables.sprite.mud").read();
                return Some(chosen.choose().to_item());
            },
            SpriteKind::Crate => {
                chosen = Lottery::<LootSpec>::load_expect("common.loot_tables.sprite.crate").read();
                return Some(chosen.choose().to_item());
            },
            _ => None,
        }
    }

    pub fn ability_spec(&self) -> Option<&AbilitySpec> { self.item_def.ability_spec.as_ref() }
}

/// Provides common methods providing details about an item definition
/// for either an `Item` containing the definition, or the actual `ItemDef`
pub trait ItemDesc {
    fn description(&self) -> &str;
    fn name(&self) -> &str;
    fn kind(&self) -> &ItemKind;
    fn quality(&self) -> &Quality;
    fn num_slots(&self) -> u16;
    fn item_definition_id(&self) -> &str;
    fn components(&self) -> &[Item];
    fn tags(&self) -> &[ItemTag];

    fn tool(&self) -> Option<&Tool> {
        if let ItemKind::Tool(tool) = self.kind() {
            Some(tool)
        } else {
            None
        }
    }
}

impl ItemDesc for Item {
    fn description(&self) -> &str { &self.item_def.description }

    fn name(&self) -> &str { &self.item_def.name }

    fn kind(&self) -> &ItemKind { &self.item_def.kind }

    fn quality(&self) -> &Quality { &self.item_def.quality }

    fn num_slots(&self) -> u16 { self.item_def.slots }

    fn item_definition_id(&self) -> &str { &self.item_def.item_definition_id }

    fn components(&self) -> &[Item] { &self.components }

    fn tags(&self) -> &[ItemTag] { &self.item_def.tags }
}

impl ItemDesc for ItemDef {
    fn description(&self) -> &str { &self.description }

    fn name(&self) -> &str { &self.name }

    fn kind(&self) -> &ItemKind { &self.kind }

    fn quality(&self) -> &Quality { &self.quality }

    fn num_slots(&self) -> u16 { self.slots }

    fn item_definition_id(&self) -> &str { &self.item_definition_id }

    fn components(&self) -> &[Item] { &[] }

    fn tags(&self) -> &[ItemTag] { &self.tags }
}

impl Component for Item {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemDrop(pub Item);

impl Component for ItemDrop {
    type Storage = IdvStorage<Self>;
}

impl<'a, T: ItemDesc + ?Sized> ItemDesc for &'a T {
    fn description(&self) -> &str { (*self).description() }

    fn name(&self) -> &str { (*self).name() }

    fn kind(&self) -> &ItemKind { (*self).kind() }

    fn quality(&self) -> &Quality { (*self).quality() }

    fn num_slots(&self) -> u16 { (*self).num_slots() }

    fn item_definition_id(&self) -> &str { (*self).item_definition_id() }

    fn components(&self) -> &[Item] { (*self).components() }

    fn tags(&self) -> &[ItemTag] { (*self).tags() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assets_items() {
        // TODO: Figure out how to get file name in error so only a single glob is
        // needed

        // Separated out into subsections so that error more descriptive
        Item::new_from_asset_glob("common.items.armor.*").expect("Failed to iterate over armors.");

        Item::new_from_asset_glob("common.items.boss_drops.*")
            .expect("Failed to iterate over boss drops.");

        Item::new_from_asset_glob("common.items.consumable.*")
            .expect("Failed to iterate over consumables.");

        Item::new_from_asset_glob("common.items.crafting_ing.*")
            .expect("Failed to iterate over crafting ingredients.");

        Item::new_from_asset_glob("common.items.crafting_tools.*")
            .expect("Failed to iterate over crafting tools.");

        Item::new_from_asset_glob("common.items.debug.*")
            .expect("Failed to iterate over debug items.");

        Item::new_from_asset_glob("common.items.flowers.*")
            .expect("Failed to iterate over flower items.");

        Item::new_from_asset_glob("common.items.food.*")
            .expect("Failed to iterate over food items.");

        Item::new_from_asset_glob("common.items.glider.*")
            .expect("Failed to iterate over gliders.");

        Item::new_from_asset_glob("common.items.grasses.*")
            .expect("Failed to iterate over grasses.");

        Item::new_from_asset_glob("common.items.lantern.*")
            .expect("Failed to iterate over lanterns.");

        Item::new_from_asset_glob("common.items.npc_armor.*")
            .expect("Failed to iterate over npc armors.");

        Item::new_from_asset_glob("common.items.npc_weapons.*")
            .expect("Failed to iterate over npc weapons.");

        Item::new_from_asset_glob("common.items.ore.*").expect("Failed to iterate over ores.");

        Item::new_from_asset_glob("common.items.tag_examples.*")
            .expect("Failed to iterate over tag examples.");

        Item::new_from_asset_glob("common.items.testing.*")
            .expect("Failed to iterate over testing items.");

        Item::new_from_asset_glob("common.items.utility.*")
            .expect("Failed to iterate over utility items.");

        // Checks each weapon type to allow errors to be located more easily
        Item::new_from_asset_glob("common.items.weapons.axe.*")
            .expect("Failed to iterate over axes.");

        Item::new_from_asset_glob("common.items.weapons.axe_1h.*")
            .expect("Failed to iterate over 1h axes.");

        Item::new_from_asset_glob("common.items.weapons.bow.*")
            .expect("Failed to iterate over bows.");

        Item::new_from_asset_glob("common.items.weapons.dagger.*")
            .expect("Failed to iterate over daggers.");

        Item::new_from_asset_glob("common.items.weapons.empty.*")
            .expect("Failed to iterate over empty.");

        Item::new_from_asset_glob("common.items.weapons.hammer.*")
            .expect("Failed to iterate over hammers.");

        Item::new_from_asset_glob("common.items.weapons.hammer_1h.*")
            .expect("Failed to iterate over 1h hammers.");

        Item::new_from_asset_glob("common.items.weapons.sceptre.*")
            .expect("Failed to iterate over sceptres.");

        Item::new_from_asset_glob("common.items.weapons.shield.*")
            .expect("Failed to iterate over shields.");

        Item::new_from_asset_glob("common.items.weapons.staff.*")
            .expect("Failed to iterate over staffs.");

        Item::new_from_asset_glob("common.items.weapons.sword.*")
            .expect("Failed to iterate over swords.");

        Item::new_from_asset_glob("common.items.weapons.sword_1h.*")
            .expect("Failed to iterate over 1h swords.");

        Item::new_from_asset_glob("common.items.weapons.tool.*")
            .expect("Failed to iterate over tools.");

        // Checks all weapons should more weapons be added later
        Item::new_from_asset_glob("common.items.weapons.*")
            .expect("Failed to iterate over weapons.");

        // Final at the end to account for a new folder being added
        Item::new_from_asset_glob("common.items.*").expect("Failed to iterate over item folders.");
    }
}
