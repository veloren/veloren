pub mod armor;
pub mod item_key;
pub mod modular;
pub mod tool;

// Reexports
pub use modular::{ModularBase, ModularComponent};
pub use tool::{AbilitySet, AbilitySpec, Hands, MaterialStatManifest, Tool, ToolKind};

use crate::{
    assets::{self, AssetExt, BoxedError, Error},
    comp::inventory::{item::tool::AbilityMap, InvSlot},
    effect::Effect,
    recipe::RecipeInput,
    terrain::Block,
};
use core::{
    convert::TryFrom,
    mem,
    num::{NonZeroU32, NonZeroU64},
};
use crossbeam_utils::atomic::AtomicCell;
use serde::{de, Deserialize, Serialize, Serializer};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;
use std::{borrow::Cow, collections::hash_map::DefaultHasher, fmt, sync::Arc};
use strum::{EnumString, IntoStaticStr};
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
    Common,    // Light blue
    Moderate,  // Green
    High,      // Blue
    Epic,      // Purple
    Legendary, // Gold
    Artifact,  // Orange
    Debug,     // Red
}

impl Quality {
    pub const MIN: Self = Self::Low;
}

pub trait TagExampleInfo {
    fn name(&self) -> &str;
    /// What item to show in the crafting hud if the player has nothing with the
    /// tag
    fn exemplar_identifier(&self) -> &str;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, IntoStaticStr)]
pub enum MaterialKind {
    Metal,
    Wood,
    Stone,
    Cloth,
    Hide,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, IntoStaticStr, EnumString,
)]
#[strum(serialize_all = "snake_case")]
pub enum Material {
    Bronze,
    Iron,
    Steel,
    Cobalt,
    Bloodsteel,
    Orichalcum,
    Wood,
    Bamboo,
    Hardwood,
    Ironwood,
    Frostwood,
    Eldwood,
    Rock,
    Granite,
    Bone,
    Basalt,
    Obsidian,
    Velorite,
    Linen,
    Wool,
    Silk,
    Lifecloth,
    Moonweave,
    Sunsilk,
    Rawhide,
    Leather,
    Scale,
    Carapace,
    Plate,
    Dragonscale,
}

impl Material {
    pub fn material_kind(&self) -> MaterialKind {
        match self {
            Material::Bronze
            | Material::Iron
            | Material::Steel
            | Material::Cobalt
            | Material::Bloodsteel
            | Material::Orichalcum => MaterialKind::Metal,
            Material::Wood
            | Material::Bamboo
            | Material::Hardwood
            | Material::Ironwood
            | Material::Frostwood
            | Material::Eldwood => MaterialKind::Wood,
            Material::Rock
            | Material::Granite
            | Material::Bone
            | Material::Basalt
            | Material::Obsidian
            | Material::Velorite => MaterialKind::Stone,
            Material::Linen
            | Material::Wool
            | Material::Silk
            | Material::Lifecloth
            | Material::Moonweave
            | Material::Sunsilk => MaterialKind::Cloth,
            Material::Rawhide
            | Material::Leather
            | Material::Scale
            | Material::Carapace
            | Material::Plate
            | Material::Dragonscale => MaterialKind::Hide,
        }
    }

    pub fn asset_identifier(&self) -> Option<&'static str> {
        match self {
            Material::Bronze => Some("common.items.mineral.ingot.bronze"),
            Material::Iron => Some("common.items.mineral.ingot.iron"),
            Material::Steel => Some("common.items.mineral.ingot.steel"),
            Material::Cobalt => Some("common.items.mineral.ingot.cobalt"),
            Material::Bloodsteel => Some("common.items.mineral.ingot.bloodsteel"),
            Material::Orichalcum => Some("common.items.mineral.ingot.orichalcum"),
            Material::Wood => Some("common.items.log.wood"),
            Material::Bamboo => Some("common.items.log.bamboo"),
            Material::Hardwood => Some("common.items.log.hardwood"),
            Material::Ironwood => Some("common.items.log.ironwood"),
            Material::Frostwood => Some("common.items.log.frostwood"),
            Material::Eldwood => Some("common.items.log.eldwood"),
            Material::Rock
            | Material::Granite
            | Material::Bone
            | Material::Basalt
            | Material::Obsidian
            | Material::Velorite => None,
            Material::Linen => Some("common.items.crafting_ing.cloth.linen"),
            Material::Wool => Some("common.items.crafting_ing.cloth.wool"),
            Material::Silk => Some("common.items.crafting_ing.cloth.silk"),
            Material::Lifecloth => Some("common.items.crafting_ing.cloth.lifecloth"),
            Material::Moonweave => Some("common.items.crafting_ing.cloth.moonweave"),
            Material::Sunsilk => Some("common.items.crafting_ing.cloth.sunsilk"),
            Material::Rawhide => Some("common.items.crafting_ing.leather.simple_leather"),
            Material::Leather => Some("common.items.crafting_ing.leather.thick_leather"),
            Material::Scale => Some("common.items.crafting_ing.hide.scales"),
            Material::Carapace => Some("common.items.crafting_ing.hide.carapace"),
            Material::Plate => Some("common.items.crafting_ing.hide.plate"),
            Material::Dragonscale => Some("common.items.crafting_ing.hide.dragon_scale"),
        }
    }
}

impl TagExampleInfo for Material {
    fn name(&self) -> &str { self.into() }

    fn exemplar_identifier(&self) -> &str { self.asset_identifier().unwrap_or("") }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ItemTag {
    /// Used to indicate that an item is composed of this material
    Material(Material),
    /// Used to indicate that an item is composed of this material kind
    MaterialKind(MaterialKind),
    Cultist,
    Potion,
    Food,
    BaseMaterial, // Cloth-scraps, Leather...
    CraftingTool, // Pickaxe, Craftsman-Hammer, Sewing-Set
    Utility,
    Bag,
    SalvageInto(Material),
}

impl TagExampleInfo for ItemTag {
    fn name(&self) -> &str {
        match self {
            ItemTag::Material(material) => material.name(),
            ItemTag::MaterialKind(material_kind) => material_kind.into(),
            ItemTag::Cultist => "cultist",
            ItemTag::Potion => "potion",
            ItemTag::Food => "food",
            ItemTag::BaseMaterial => "basemat",
            ItemTag::CraftingTool => "tool",
            ItemTag::Utility => "utility",
            ItemTag::Bag => "bag",
            ItemTag::SalvageInto(_) => "salvage",
        }
    }

    // TODO: Autogenerate these?
    fn exemplar_identifier(&self) -> &str {
        match self {
            ItemTag::Material(_) => "common.items.tag_examples.placeholder",
            ItemTag::MaterialKind(_) => "common.items.tag_examples.placeholder",
            ItemTag::Cultist => "common.items.tag_examples.cultist",
            ItemTag::Potion => "common.items.tag_examples.placeholder",
            ItemTag::Food => "common.items.tag_examples.placeholder",
            ItemTag::BaseMaterial => "common.items.tag_examples.placeholder",
            ItemTag::CraftingTool => "common.items.tag_examples.placeholder",
            ItemTag::Utility => "common.items.tag_examples.placeholder",
            ItemTag::Bag => "common.items.tag_examples.placeholder",
            ItemTag::SalvageInto(_) => "common.items.tag_examples.placeholder",
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
        kind: ConsumableKind,
        effects: Vec<Effect>,
    },
    Throwable {
        kind: Throwable,
    },
    Utility {
        kind: Utility,
    },
    Ingredient {
        kind: String,
        descriptor: String,
    },
    TagExamples {
        /// A list of item names to lookup the appearences of and animate
        /// through
        item_ids: Vec<String>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConsumableKind {
    Drink,
    Food,
    ComplexFood,
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
        serialize_with = "serialize_item_base",
        deserialize_with = "deserialize_item_base"
    )]
    item_base: ItemBase,
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
    hash: u64,
}

use std::hash::{Hash, Hasher};

// Used to find inventory item corresponding to hotbar slot
impl Hash for Item {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.item_definition_id().hash(state);
        self.components.iter().for_each(|comp| comp.hash(state));
    }
}

// Custom serialization for ItemDef, we only want to send the item_definition_id
// over the network, the client will use deserialize_item_def to fetch the
// ItemDef from assets.
fn serialize_item_base<S: Serializer>(field: &ItemBase, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(match field {
        ItemBase::Raw(item_def) => &item_def.item_definition_id,
        ItemBase::Modular(mod_base) => mod_base.pseudo_item_id(),
    })
}

// Custom de-serialization for ItemBase to retrieve the ItemBase from assets
// using its asset specifier (item_definition_id)
fn deserialize_item_base<'de, D>(deserializer: D) -> Result<ItemBase, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct ItemBaseStringVisitor;

    impl<'de> de::Visitor<'de> for ItemBaseStringVisitor {
        type Value = ItemBase;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("item def string")
        }

        fn visit_str<E>(self, serialized_item_base: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(
                if serialized_item_base.starts_with("veloren.core.pseudo_items.modular.") {
                    ItemBase::Modular(ModularBase::load_from_pseudo_id(serialized_item_base))
                } else {
                    ItemBase::Raw(Arc::<ItemDef>::load_expect_cloned(serialized_item_base))
                },
            )
        }
    }

    deserializer.deserialize_str(ItemBaseStringVisitor)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ItemName {
    Direct(String),
    Modular,
    Component(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ItemBase {
    Raw(Arc<ItemDef>),
    Modular(modular::ModularBase),
}

impl ItemBase {
    fn num_slots(&self) -> u16 {
        match self {
            ItemBase::Raw(item_def) => item_def.num_slots(),
            ItemBase::Modular(_) => 0,
        }
    }

    fn item_definition_id(&self) -> &str {
        match &self {
            ItemBase::Raw(item_def) => &item_def.item_definition_id,
            ItemBase::Modular(mod_base) => mod_base.pseudo_item_id(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemDef {
    #[serde(default)]
    /// The string that refers to the filepath to the asset, relative to the
    /// assets folder, which the ItemDef is loaded from. The name space
    /// prepended with `veloren.core` is reserved for veloren functions.
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
    pub abilities: AbilitySet<tool::AbilityItem>,
}

#[derive(Debug)]
pub enum ItemConfigError {
    BadItemKind,
}

impl TryFrom<(&Item, &AbilityMap, &MaterialStatManifest)> for ItemConfig {
    type Error = ItemConfigError;

    fn try_from(
        // TODO: Either remove msm or use it as argument in fn kind
        (item, ability_map, _msm): (&Item, &AbilityMap, &MaterialStatManifest),
    ) -> Result<Self, Self::Error> {
        if let ItemKind::Tool(tool) = &*item.kind() {
            // If no custom ability set is specified, fall back to abilityset of tool kind.
            let tool_default = |tool_kind| {
                let key = &AbilitySpec::Tool(tool_kind);
                ability_map.get_ability_set(key)
            };
            let abilities = if let Some(set_key) = item.ability_spec() {
                if let Some(set) = ability_map.get_ability_set(&*set_key) {
                    set.clone().modified_by_tool(tool)
                } else {
                    error!(
                        "Custom ability set: {:?} references non-existent set, falling back to \
                         default ability set.",
                        set_key
                    );
                    tool_default(tool.kind).cloned().unwrap_or_default()
                }
            } else if let Some(set) = tool_default(tool.kind) {
                set.clone().modified_by_tool(tool)
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

    #[cfg(test)]
    pub fn create_test_itemdef_from_kind(kind: ItemKind) -> Self {
        Self {
            item_definition_id: "test.item".to_string(),
            name: "test item name".to_owned(),
            description: "test item description".to_owned(),
            kind,
            quality: Quality::Common,
            tags: vec![],
            slots: 0,
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
        if let (ItemBase::Raw(self_def), ItemBase::Raw(other_def)) =
            (&self.item_base, &other.item_base)
        {
            self_def.item_definition_id == other_def.item_definition_id
                && self.components == other.components
        } else {
            false
        }
    }
}

impl assets::Compound for ItemDef {
    fn load<S: assets::source::Source + ?Sized>(
        cache: &assets::AssetCache<S>,
        specifier: &str,
    ) -> Result<Self, BoxedError> {
        if specifier.starts_with("veloren.core.") {
            return Err(format!(
                "Attempted to load an asset from a specifier reserved for core veloren functions. \
                 Specifier: {}",
                specifier
            )
            .into());
        }

        let RawItemDef {
            name,
            description,
            kind,
            quality,
            tags,
            slots,
            ability_spec,
        } = cache.load::<RawItemDef>(specifier)?.cloned();

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

impl Item {
    // TODO: consider alternatives such as default abilities that can be added to a
    // loadout when no weapon is present
    pub fn empty() -> Self { Item::new_from_asset_expect("common.items.weapons.empty.empty") }

    pub fn new_from_item_base(
        inner_item: ItemBase,
        components: Vec<Item>,
        ability_map: &AbilityMap,
        msm: &MaterialStatManifest,
    ) -> Self {
        let mut item = Item {
            item_id: Arc::new(AtomicCell::new(None)),
            amount: NonZeroU32::new(1).unwrap(),
            components,
            slots: vec![None; inner_item.num_slots() as usize],
            item_base: inner_item,
            // Updated immediately below
            item_config: None,
            hash: 0,
        };
        item.update_item_state(ability_map, msm);
        item
    }

    /// Creates a new instance of an `Item` from the provided asset identifier
    /// Panics if the asset does not exist.
    pub fn new_from_asset_expect(asset_specifier: &str) -> Self {
        Item::new_from_asset(asset_specifier).unwrap_or_else(|err| {
            panic!(
                "Expected asset to exist: {}, instead got error {:?}",
                asset_specifier, err
            );
        })
    }

    /// Creates a Vec containing one of each item that matches the provided
    /// asset glob pattern
    pub fn new_from_asset_glob(asset_glob: &str) -> Result<Vec<Self>, Error> {
        let specifier = asset_glob.strip_suffix(".*").unwrap_or(asset_glob);
        let defs = assets::load_dir::<RawItemDef>(specifier, true)?;
        defs.ids().map(Item::new_from_asset).collect()
    }

    /// Creates a new instance of an `Item from the provided asset identifier if
    /// it exists
    pub fn new_from_asset(asset: &str) -> Result<Self, Error> {
        let inner_item = if asset.starts_with("veloren.core.pseudo_items.modular") {
            ItemBase::Modular(ModularBase::load_from_pseudo_id(asset))
        } else {
            ItemBase::Raw(Arc::<ItemDef>::load_cloned(asset)?)
        };
        // TODO: Get msm and ability_map less hackily
        let msm = &MaterialStatManifest::load().read();
        let ability_map = &AbilityMap::load().read();
        Ok(Item::new_from_item_base(
            inner_item,
            Vec::new(),
            ability_map,
            msm,
        ))
    }

    /// Duplicates an item, creating an exact copy but with a new item ID
    #[must_use]
    pub fn duplicate(&self, ability_map: &AbilityMap, msm: &MaterialStatManifest) -> Self {
        let duplicated_components = self
            .components
            .iter()
            .map(|comp| comp.duplicate(ability_map, msm))
            .collect();
        let mut new_item = Item::new_from_item_base(
            match &self.item_base {
                ItemBase::Raw(item_def) => ItemBase::Raw(Arc::clone(item_def)),
                ItemBase::Modular(mod_base) => ItemBase::Modular(mod_base.clone()),
            },
            duplicated_components,
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
        // Reset item id for every component of an item too
        for component in self.components.iter_mut() {
            component.reset_item_id();
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

    pub fn persistence_access_add_component(&mut self, component: Item) {
        self.components.push(component);
    }

    pub fn persistence_access_mutable_component(&mut self, index: usize) -> Option<&mut Self> {
        self.components.get_mut(index)
    }

    /// Updates state of an item (important for creation of new items,
    /// persistence, and if components are ever added to items after initial
    /// creation)
    pub fn update_item_state(&mut self, ability_map: &AbilityMap, msm: &MaterialStatManifest) {
        // Updates item config of an item
        if let Ok(item_config) = ItemConfig::try_from((&*self, ability_map, msm)) {
            self.item_config = Some(Box::new(item_config));
        }
        // Updates hash of an item
        self.hash = {
            let mut s = DefaultHasher::new();
            self.hash(&mut s);
            s.finish()
        };
    }

    /// Returns an iterator that drains items contained within the item's slots
    pub fn drain(&mut self) -> impl Iterator<Item = Item> + '_ {
        self.slots.iter_mut().filter_map(mem::take)
    }

    pub fn item_definition_id(&self) -> &str { self.item_base.item_definition_id() }

    pub fn is_same_item_def(&self, item_def: &ItemDef) -> bool {
        if let ItemBase::Raw(self_def) = &self.item_base {
            self_def.item_definition_id == item_def.item_definition_id
        } else {
            false
        }
    }

    pub fn matches_recipe_input(&self, recipe_input: &RecipeInput, amount: u32) -> bool {
        match recipe_input {
            RecipeInput::Item(item_def) => self.is_same_item_def(item_def),
            RecipeInput::Tag(tag) => self.tags().contains(tag),
            RecipeInput::TagSameItem(tag) => {
                self.tags().contains(tag) && u32::from(self.amount) >= amount
            },
            RecipeInput::ListSameItem(item_defs) => item_defs.iter().any(|item_def| {
                self.is_same_item_def(item_def) && u32::from(self.amount) >= amount
            }),
        }
    }

    pub fn is_salvageable(&self) -> bool {
        self.tags()
            .iter()
            .any(|tag| matches!(tag, ItemTag::SalvageInto(_)))
    }

    pub fn salvage_output(&self) -> impl Iterator<Item = &str> {
        self.tags().into_iter().filter_map(|tag| {
            if let ItemTag::SalvageInto(material) = tag {
                material.asset_identifier()
            } else {
                None
            }
        })
    }

    pub fn name(&self) -> Cow<str> {
        match &self.item_base {
            ItemBase::Raw(item_def) => {
                if self.components.is_empty() {
                    Cow::Borrowed(&item_def.name)
                } else {
                    modular::modify_name(&item_def.name, self)
                }
            },
            ItemBase::Modular(mod_base) => mod_base.generate_name(self.components()),
        }
    }

    pub fn description(&self) -> &str {
        match &self.item_base {
            ItemBase::Raw(item_def) => &item_def.description,
            // TODO: See if James wanted to make description, else leave with none
            ItemBase::Modular(_) => "",
        }
    }

    pub fn kind(&self) -> Cow<ItemKind> {
        match &self.item_base {
            ItemBase::Raw(item_def) => Cow::Borrowed(&item_def.kind),
            ItemBase::Modular(mod_base) => {
                // TODO: Try to move further upward
                let msm = MaterialStatManifest::load().read();
                mod_base.kind(self.components(), &msm)
            },
        }
    }

    pub fn amount(&self) -> u32 { u32::from(self.amount) }

    pub fn is_stackable(&self) -> bool {
        match &self.item_base {
            ItemBase::Raw(item_def) => item_def.is_stackable(),
            // TODO: Let whoever implements stackable modular items deal with this
            ItemBase::Modular(_) => false,
        }
    }

    pub fn num_slots(&self) -> u16 { self.item_base.num_slots() }

    /// NOTE: invariant that amount() ≤ max_amount(), 1 ≤ max_amount(),
    /// and if !self.is_stackable(), self.max_amount() = 1.
    pub fn max_amount(&self) -> u32 { if self.is_stackable() { u32::MAX } else { 1 } }

    pub fn quality(&self) -> Quality {
        match &self.item_base {
            ItemBase::Raw(item_def) => item_def.quality,
            ItemBase::Modular(mod_base) => mod_base.compute_quality(self.components()),
        }
    }

    pub fn components(&self) -> &[Item] { &self.components }

    pub fn slots(&self) -> &[InvSlot] { &self.slots }

    pub fn slots_mut(&mut self) -> &mut [InvSlot] { &mut self.slots }

    pub fn item_config_expect(&self) -> &ItemConfig {
        self.item_config
            .as_ref()
            .expect("Item was expected to have an ItemConfig")
    }

    pub fn free_slots(&self) -> usize { self.slots.iter().filter(|x| x.is_none()).count() }

    pub fn populated_slots(&self) -> usize { self.slots().len().saturating_sub(self.free_slots()) }

    pub fn slot(&self, slot: usize) -> Option<&InvSlot> { self.slots.get(slot) }

    pub fn slot_mut(&mut self, slot: usize) -> Option<&mut InvSlot> { self.slots.get_mut(slot) }

    pub fn try_reclaim_from_block(block: Block) -> Option<Self> {
        block.get_sprite()?.collectible_id()?.to_item()
    }

    pub fn ability_spec(&self) -> Option<Cow<AbilitySpec>> {
        match &self.item_base {
            ItemBase::Raw(item_def) => item_def.ability_spec.as_ref().map(Cow::Borrowed).or({
                // If no custom ability set is specified, fall back to abilityset of tool kind.
                if let ItemKind::Tool(tool) = &item_def.kind {
                    Some(Cow::Owned(AbilitySpec::Tool(tool.kind)))
                } else {
                    None
                }
            }),
            ItemBase::Modular(mod_base) => mod_base.ability_spec(self.components()),
        }
    }

    // TODO: Maybe try to make slice again instead of vec?
    pub fn tags(&self) -> Vec<ItemTag> {
        match &self.item_base {
            ItemBase::Raw(item_def) => item_def.tags.to_vec(),
            // TODO: Do this properly. It'll probably be important at some point.
            ItemBase::Modular(mod_base) => mod_base.generate_tags(self.components()),
        }
    }

    pub fn is_modular(&self) -> bool {
        match &self.item_base {
            ItemBase::Raw(_) => false,
            ItemBase::Modular(_) => true,
        }
    }

    pub fn item_hash(&self) -> u64 { self.hash }

    #[cfg(test)]
    pub fn create_test_item_from_kind(kind: ItemKind) -> Self {
        let ability_map = &AbilityMap::load().read();
        let msm = &MaterialStatManifest::load().read();
        Self::new_from_item_base(
            ItemBase::Raw(Arc::new(ItemDef::create_test_itemdef_from_kind(kind))),
            Vec::new(),
            ability_map,
            msm,
        )
    }
}

/// Provides common methods providing details about an item definition
/// for either an `Item` containing the definition, or the actual `ItemDef`
pub trait ItemDesc {
    fn description(&self) -> &str;
    fn name(&self) -> Cow<str>;
    fn kind(&self) -> Cow<ItemKind>;
    fn quality(&self) -> Quality;
    fn num_slots(&self) -> u16;
    fn item_definition_id(&self) -> &str;
    fn tags(&self) -> Vec<ItemTag>;

    fn is_modular(&self) -> bool;

    fn components(&self) -> &[Item];

    fn tool_info(&self) -> Option<ToolKind> {
        if let ItemKind::Tool(tool) = &*self.kind() {
            Some(tool.kind)
        } else {
            None
        }
    }
}

impl ItemDesc for Item {
    fn description(&self) -> &str { self.description() }

    fn name(&self) -> Cow<str> { self.name() }

    fn kind(&self) -> Cow<ItemKind> { self.kind() }

    fn quality(&self) -> Quality { self.quality() }

    fn num_slots(&self) -> u16 { self.num_slots() }

    fn item_definition_id(&self) -> &str { self.item_definition_id() }

    fn tags(&self) -> Vec<ItemTag> { self.tags() }

    fn is_modular(&self) -> bool { self.is_modular() }

    fn components(&self) -> &[Item] { self.components() }
}

impl ItemDesc for ItemDef {
    fn description(&self) -> &str { &self.description }

    fn name(&self) -> Cow<str> { Cow::Borrowed(&self.name) }

    fn kind(&self) -> Cow<ItemKind> { Cow::Borrowed(&self.kind) }

    fn quality(&self) -> Quality { self.quality }

    fn num_slots(&self) -> u16 { self.slots }

    fn item_definition_id(&self) -> &str { &self.item_definition_id }

    fn tags(&self) -> Vec<ItemTag> { self.tags.to_vec() }

    fn is_modular(&self) -> bool { false }

    fn components(&self) -> &[Item] { &[] }
}

impl Component for Item {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemDrop(pub Item);

impl Component for ItemDrop {
    type Storage = IdvStorage<Self>;
}

impl<'a, T: ItemDesc + ?Sized> ItemDesc for &'a T {
    fn description(&self) -> &str { (*self).description() }

    fn name(&self) -> Cow<str> { (*self).name() }

    fn kind(&self) -> Cow<ItemKind> { (*self).kind() }

    fn quality(&self) -> Quality { (*self).quality() }

    fn num_slots(&self) -> u16 { (*self).num_slots() }

    fn item_definition_id(&self) -> &str { (*self).item_definition_id() }

    fn components(&self) -> &[Item] { (*self).components() }

    fn tags(&self) -> Vec<ItemTag> { (*self).tags() }

    fn is_modular(&self) -> bool { (*self).is_modular() }
}

/// Returns all item asset specifiers
///
/// Panics in case of filesystem errors
pub fn all_item_defs_expect() -> Vec<String> {
    try_all_item_defs().expect("Failed to access items directory")
}

/// Returns all item asset specifiers
pub fn try_all_item_defs() -> Result<Vec<String>, Error> {
    let defs = assets::load_dir::<RawItemDef>("common.items", true)?;
    Ok(defs.ids().map(|id| id.to_owned()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assets_items() {
        let ids = all_item_defs_expect();
        for item in ids.iter().map(|id| Item::new_from_asset_expect(id)) {
            std::mem::drop(item)
        }
    }
}
