pub mod armor;
pub mod item_key;
pub mod modular;
pub mod tool;

// Reexports
pub use modular::{MaterialStatManifest, ModularBase, ModularComponent};
pub use tool::{AbilityMap, AbilitySet, AbilitySpec, Hands, Tool, ToolKind};

use crate::{
    assets::{self, AssetExt, BoxedError, Error},
    comp::inventory::InvSlot,
    effect::Effect,
    recipe::RecipeInput,
    terrain::Block,
};
use common_i18n::Content;
use core::{
    convert::TryFrom,
    mem,
    num::{NonZeroU32, NonZeroU64},
};
use crossbeam_utils::atomic::AtomicCell;
use hashbrown::{Equivalent, HashMap};
use item_key::ItemKey;
use serde::{de, Deserialize, Serialize, Serializer};
use specs::{Component, DenseVecStorage, DerefFlaggedStorage};
use std::{borrow::Cow, collections::hash_map::DefaultHasher, fmt, sync::Arc};
use strum::{EnumIter, EnumString, IntoEnumIterator, IntoStaticStr};
use tracing::error;
use vek::Rgb;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Throwable {
    Bomb,
    Mine,
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
    Phoenix,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Utility {
    Coins,
    Collar,
    Key,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lantern {
    color: Rgb<u32>,
    strength_thousandths: u32,
    flicker_thousandths: u32,
}

impl Lantern {
    pub fn strength(&self) -> f32 { self.strength_thousandths as f32 / 1000_f32 }

    pub fn color(&self) -> Rgb<f32> { self.color.map(|c| c as f32 / 255.0) }

    pub fn flicker(&self) -> f32 { self.flicker_thousandths as f32 / 1000_f32 }
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
    fn exemplar_identifier(&self) -> Option<&str>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, IntoStaticStr)]
pub enum MaterialKind {
    Metal,
    Gem,
    Wood,
    Stone,
    Cloth,
    Hide,
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    IntoStaticStr,
    EnumString,
    EnumIter,
)]
#[strum(serialize_all = "snake_case")]
pub enum Material {
    Bronze,
    Iron,
    Steel,
    Cobalt,
    Bloodsteel,
    Silver,
    Gold,
    Orichalcum,
    Topaz,
    Emerald,
    Sapphire,
    Amethyst,
    Ruby,
    Diamond,
    Twig,
    PlantFiber,
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
    RedLinen,
    Wool,
    Silk,
    Lifecloth,
    Moonweave,
    Sunsilk,
    Rawhide,
    Leather,
    RigidLeather,
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
            | Material::Silver
            | Material::Gold
            | Material::Orichalcum => MaterialKind::Metal,
            Material::Topaz
            | Material::Emerald
            | Material::Sapphire
            | Material::Amethyst
            | Material::Ruby
            | Material::Diamond => MaterialKind::Gem,
            Material::Wood
            | Material::Twig
            | Material::PlantFiber
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
            | Material::RedLinen
            | Material::Wool
            | Material::Silk
            | Material::Lifecloth
            | Material::Moonweave
            | Material::Sunsilk => MaterialKind::Cloth,
            Material::Rawhide
            | Material::Leather
            | Material::RigidLeather
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
            Material::Silver => Some("common.items.mineral.ingot.silver"),
            Material::Gold => Some("common.items.mineral.ingot.gold"),
            Material::Orichalcum => Some("common.items.mineral.ingot.orichalcum"),
            Material::Topaz => Some("common.items.mineral.gem.topaz"),
            Material::Emerald => Some("common.items.mineral.gem.emerald"),
            Material::Sapphire => Some("common.items.mineral.gem.sapphire"),
            Material::Amethyst => Some("common.items.mineral.gem.amethyst"),
            Material::Ruby => Some("common.items.mineral.gem.ruby"),
            Material::Diamond => Some("common.items.mineral.gem.diamond"),
            Material::Twig => Some("common.items.crafting_ing.twigs"),
            Material::PlantFiber => Some("common.items.flowers.plant_fiber"),
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
            Material::RedLinen => Some("common.items.crafting_ing.cloth.linen_red"),
            Material::Wool => Some("common.items.crafting_ing.cloth.wool"),
            Material::Silk => Some("common.items.crafting_ing.cloth.silk"),
            Material::Lifecloth => Some("common.items.crafting_ing.cloth.lifecloth"),
            Material::Moonweave => Some("common.items.crafting_ing.cloth.moonweave"),
            Material::Sunsilk => Some("common.items.crafting_ing.cloth.sunsilk"),
            Material::Rawhide => Some("common.items.crafting_ing.leather.simple_leather"),
            Material::Leather => Some("common.items.crafting_ing.leather.thick_leather"),
            Material::RigidLeather => Some("common.items.crafting_ing.leather.rigid_leather"),
            Material::Scale => Some("common.items.crafting_ing.hide.scales"),
            Material::Carapace => Some("common.items.crafting_ing.hide.carapace"),
            Material::Plate => Some("common.items.crafting_ing.hide.plate"),
            Material::Dragonscale => Some("common.items.crafting_ing.hide.dragon_scale"),
        }
    }
}

impl TagExampleInfo for Material {
    fn name(&self) -> &str { self.into() }

    fn exemplar_identifier(&self) -> Option<&str> { self.asset_identifier() }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemTag {
    /// Used to indicate that an item is composed of this material
    Material(Material),
    /// Used to indicate that an item is composed of this material kind
    MaterialKind(MaterialKind),
    Cultist,
    Gnarling,
    Potion,
    Food,
    BaseMaterial, // Cloth-scraps, Leather...
    CraftingTool, // Pickaxe, Craftsman-Hammer, Sewing-Set
    Utility,
    Bag,
    SalvageInto(Material, u32),
}

impl TagExampleInfo for ItemTag {
    fn name(&self) -> &str {
        match self {
            ItemTag::Material(material) => material.name(),
            ItemTag::MaterialKind(material_kind) => material_kind.into(),
            ItemTag::Cultist => "cultist",
            ItemTag::Gnarling => "gnarling",
            ItemTag::Potion => "potion",
            ItemTag::Food => "food",
            ItemTag::BaseMaterial => "basemat",
            ItemTag::CraftingTool => "tool",
            ItemTag::Utility => "utility",
            ItemTag::Bag => "bag",
            ItemTag::SalvageInto(_, _) => "salvage",
        }
    }

    // TODO: Autogenerate these?
    fn exemplar_identifier(&self) -> Option<&str> {
        match self {
            ItemTag::Material(material) => material.exemplar_identifier(),
            ItemTag::MaterialKind(_) => None,
            ItemTag::Cultist => Some("common.items.tag_examples.cultist"),
            ItemTag::Gnarling => Some("common.items.tag_examples.gnarling"),
            ItemTag::Potion => None,
            ItemTag::Food => None,
            ItemTag::BaseMaterial => None,
            ItemTag::CraftingTool => None,
            ItemTag::Utility => None,
            ItemTag::Bag => None,
            ItemTag::SalvageInto(_, _) => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Effects {
    Any(Vec<Effect>),
    All(Vec<Effect>),
    One(Effect),
}

impl Effects {
    pub fn effects(&self) -> &[Effect] {
        match self {
            Effects::Any(effects) => effects,
            Effects::All(effects) => effects,
            Effects::One(effect) => std::slice::from_ref(effect),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ItemKind {
    /// Something wieldable
    Tool(Tool),
    ModularComponent(ModularComponent),
    Lantern(Lantern),
    Armor(armor::Armor),
    Glider,
    Consumable {
        kind: ConsumableKind,
        effects: Effects,
    },
    Throwable {
        kind: Throwable,
    },
    Utility {
        kind: Utility,
    },
    Ingredient {
        /// Used to generate names for modular items composed of this ingredient
        // I think we can actually remove it now?
        #[deprecated = "part of non-localized name generation"]
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
    Charm,
}

impl ItemKind {
    pub fn is_equippable(&self) -> bool {
        matches!(
            self,
            ItemKind::Tool(_) | ItemKind::Armor { .. } | ItemKind::Glider | ItemKind::Lantern(_)
        )
    }

    // Used for inventory sorting, what comes before the first colon (:) is used as
    // a broader category
    pub fn get_itemkind_string(&self) -> String {
        match self {
            // Using tool and toolkind to sort tools by kind
            ItemKind::Tool(tool) => format!("Tool: {:?}", tool.kind),
            ItemKind::ModularComponent(modular_component) => {
                format!("ModularComponent: {:?}", modular_component.toolkind())
            },
            ItemKind::Lantern(lantern) => format!("Lantern: {:?}", lantern),
            ItemKind::Armor(armor) => format!("Armor: {:?}", armor.stats),
            ItemKind::Glider => "Glider:".to_string(),
            ItemKind::Consumable { kind, .. } => {
                format!("Consumable: {:?}", kind)
            },
            ItemKind::Throwable { kind } => format!("Throwable: {:?}", kind),
            ItemKind::Utility { kind } => format!("Utility: {:?}", kind),
            #[allow(deprecated)]
            ItemKind::Ingredient { descriptor } => format!("Ingredient: {}", descriptor),
            ItemKind::TagExamples { item_ids } => format!("TagExamples: {:?}", item_ids),
        }
    }

    pub fn has_durability(&self) -> bool {
        match self {
            ItemKind::Tool(_) => true,
            ItemKind::Armor(armor) => armor.kind.has_durability(),
            ItemKind::ModularComponent(_)
            | ItemKind::Lantern(_)
            | ItemKind::Glider
            | ItemKind::Consumable { .. }
            | ItemKind::Throwable { .. }
            | ItemKind::Utility { .. }
            | ItemKind::Ingredient { .. }
            | ItemKind::TagExamples { .. } => false,
        }
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
    /// Tracks how many deaths occurred while item was equipped, which is
    /// converted into the items durability. Only tracked for tools and armor
    /// currently.
    durability_lost: Option<u32>,
}

use std::hash::{Hash, Hasher};

// Used to find inventory item corresponding to hotbar slot
impl Hash for Item {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.item_definition_id().hash(state);
        self.components.iter().for_each(|comp| comp.hash(state));
    }
}

// at the time of writing, we use Fluent, which supports attributes
// and we can get both name and description using them
type I18nId = String;

#[derive(Clone, Debug, Serialize, Deserialize)]
// TODO: probably make a Resource if used outside of voxygen
// TODO: add hot-reloading similar to how ItemImgs does it?
// TODO: make it work with plugins (via Concatenate?)
/// To be used with ItemDesc::i18n
///
/// NOTE: there is a limitation to this manifest, as it uses ItemKey and
/// ItemKey isn't uniquely identifies Item, when it comes to modular items.
///
/// If modular weapon has the same primary component and the same hand-ness,
/// we use the same model EVEN IF it has different secondary components, like
/// Staff with Heavy core or Light core.
///
/// Translations currently do the same, but *maybe* they shouldn't in which case
/// we should either extend ItemKey or use new identifier. We could use
/// ItemDefinitionId, but it's very generic and cumbersome.
pub struct ItemI18n {
    /// maps ItemKey to i18n identifier
    map: HashMap<ItemKey, I18nId>,
}

impl assets::Asset for ItemI18n {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

impl ItemI18n {
    pub fn new_expect() -> Self {
        ItemI18n::load_expect("common.item_i18n_manifest")
            .read()
            .clone()
    }

    /// Returns (name, description) in Content form.
    // TODO: after we remove legacy text from ItemDef, consider making this
    // function non-fallible?
    fn item_text_opt(&self, mut item_key: ItemKey) -> Option<(Content, Content)> {
        // we don't put TagExamples into manifest
        if let ItemKey::TagExamples(_, id) = item_key {
            item_key = ItemKey::Simple(id.to_string());
        }

        let key = self.map.get(&item_key);
        key.map(|key| {
            (
                Content::Key(key.to_owned()),
                Content::Attr(key.to_owned(), "desc".to_owned()),
            )
        })
    }
}

#[derive(Clone, Debug)]
pub enum ItemBase {
    Simple(Arc<ItemDef>),
    Modular(ModularBase),
}

impl Serialize for ItemBase {
    // Custom serialization for ItemDef, we only want to send the item_definition_id
    // over the network, the client will use deserialize_item_def to fetch the
    // ItemDef from assets.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            ItemBase::Simple(item_def) => &item_def.item_definition_id,
            ItemBase::Modular(mod_base) => mod_base.pseudo_item_id(),
        })
    }
}

impl<'de> Deserialize<'de> for ItemBase {
    // Custom de-serialization for ItemBase to retrieve the ItemBase from assets
    // using its asset specifier (item_definition_id)
    fn deserialize<D>(deserializer: D) -> Result<ItemBase, D::Error>
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
                    if serialized_item_base.starts_with(crate::modular_item_id_prefix!()) {
                        ItemBase::Modular(ModularBase::load_from_pseudo_id(serialized_item_base))
                    } else {
                        ItemBase::Simple(Arc::<ItemDef>::load_expect_cloned(serialized_item_base))
                    },
                )
            }
        }

        deserializer.deserialize_str(ItemBaseStringVisitor)
    }
}

impl ItemBase {
    fn num_slots(&self) -> u16 {
        match self {
            ItemBase::Simple(item_def) => item_def.num_slots(),
            ItemBase::Modular(_) => 0,
        }
    }
}

// TODO: could this theorectically hold a ref to the actual components and
// lazily get their IDs for hash/partialeq/debug/to_owned/etc? (i.e. eliminating
// `Vec`s)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ItemDefinitionId<'a> {
    Simple(&'a str),
    Modular {
        pseudo_base: &'a str,
        components: Vec<ItemDefinitionId<'a>>,
    },
    Compound {
        simple_base: &'a str,
        components: Vec<ItemDefinitionId<'a>>,
    },
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum ItemDefinitionIdOwned {
    Simple(String),
    Modular {
        pseudo_base: String,
        components: Vec<ItemDefinitionIdOwned>,
    },
    Compound {
        simple_base: String,
        components: Vec<ItemDefinitionIdOwned>,
    },
}

impl ItemDefinitionIdOwned {
    pub fn as_ref(&self) -> ItemDefinitionId<'_> {
        match *self {
            Self::Simple(ref id) => ItemDefinitionId::Simple(id),
            Self::Modular {
                ref pseudo_base,
                ref components,
            } => ItemDefinitionId::Modular {
                pseudo_base,
                components: components.iter().map(|comp| comp.as_ref()).collect(),
            },
            Self::Compound {
                ref simple_base,
                ref components,
            } => ItemDefinitionId::Compound {
                simple_base,
                components: components.iter().map(|comp| comp.as_ref()).collect(),
            },
        }
    }
}

impl<'a> ItemDefinitionId<'a> {
    pub fn itemdef_id(&self) -> Option<&str> {
        match self {
            Self::Simple(id) => Some(id),
            Self::Modular { .. } => None,
            Self::Compound { simple_base, .. } => Some(simple_base),
        }
    }

    pub fn to_owned(&self) -> ItemDefinitionIdOwned {
        match self {
            Self::Simple(id) => ItemDefinitionIdOwned::Simple(String::from(*id)),
            Self::Modular {
                pseudo_base,
                components,
            } => ItemDefinitionIdOwned::Modular {
                pseudo_base: String::from(*pseudo_base),
                components: components.iter().map(|comp| comp.to_owned()).collect(),
            },
            Self::Compound {
                simple_base,
                components,
            } => ItemDefinitionIdOwned::Compound {
                simple_base: String::from(*simple_base),
                components: components.iter().map(|comp| comp.to_owned()).collect(),
            },
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
    #[deprecated = "since item i18n"]
    name: String,
    #[deprecated = "since item i18n"]
    description: String,
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
                if let Some(set) = ability_map.get_ability_set(&set_key) {
                    set.clone()
                        .modified_by_tool(tool, item.stats_durability_multiplier())
                } else {
                    error!(
                        "Custom ability set: {:?} references non-existent set, falling back to \
                         default ability set.",
                        set_key
                    );
                    tool_default(tool.kind).cloned().unwrap_or_default()
                }
            } else if let Some(set) = tool_default(tool.kind) {
                set.clone()
                    .modified_by_tool(tool, item.stats_durability_multiplier())
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
        #[allow(deprecated)]
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
        #[allow(deprecated)]
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
        if let (ItemBase::Simple(self_def), ItemBase::Simple(other_def)) =
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
    fn load(cache: assets::AnyCache, specifier: &assets::SharedString) -> Result<Self, BoxedError> {
        if specifier.starts_with("veloren.core.") {
            return Err(format!(
                "Attempted to load an asset from a specifier reserved for core veloren functions. \
                 Specifier: {}",
                specifier
            )
            .into());
        }

        let RawItemDef {
            legacy_name,
            legacy_description,
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

        #[allow(deprecated)]
        Ok(ItemDef {
            item_definition_id,
            name: legacy_name,
            description: legacy_description,
            kind,
            quality,
            tags,
            slots,
            ability_spec,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename = "ItemDef", deny_unknown_fields)]
struct RawItemDef {
    legacy_name: String,
    legacy_description: String,
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
    pub const MAX_DURABILITY: u32 = 12;

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
            // These fields are updated immediately below
            item_config: None,
            hash: 0,
            durability_lost: None,
        };
        item.durability_lost = item.has_durability().then_some(0);
        item.update_item_state(ability_map, msm);
        item
    }

    pub fn new_from_item_definition_id(
        item_definition_id: ItemDefinitionId<'_>,
        ability_map: &AbilityMap,
        msm: &MaterialStatManifest,
    ) -> Result<Self, Error> {
        let (base, components) = match item_definition_id {
            ItemDefinitionId::Simple(spec) => {
                let base = ItemBase::Simple(Arc::<ItemDef>::load_cloned(spec)?);
                (base, Vec::new())
            },
            ItemDefinitionId::Modular {
                pseudo_base,
                components,
            } => {
                let base = ItemBase::Modular(ModularBase::load_from_pseudo_id(pseudo_base));
                let components = components
                    .into_iter()
                    .map(|id| Item::new_from_item_definition_id(id, ability_map, msm))
                    .collect::<Result<Vec<_>, _>>()?;
                (base, components)
            },
            ItemDefinitionId::Compound {
                simple_base,
                components,
            } => {
                let base = ItemBase::Simple(Arc::<ItemDef>::load_cloned(simple_base)?);
                let components = components
                    .into_iter()
                    .map(|id| Item::new_from_item_definition_id(id, ability_map, msm))
                    .collect::<Result<Vec<_>, _>>()?;
                (base, components)
            },
        };
        Ok(Item::new_from_item_base(base, components, ability_map, msm))
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
        let defs = assets::load_rec_dir::<RawItemDef>(specifier)?;
        defs.read()
            .ids()
            .map(|id| Item::new_from_asset(id))
            .collect()
    }

    /// Creates a new instance of an `Item from the provided asset identifier if
    /// it exists
    pub fn new_from_asset(asset: &str) -> Result<Self, Error> {
        let inner_item = if asset.starts_with("veloren.core.pseudo_items.modular") {
            ItemBase::Modular(ModularBase::load_from_pseudo_id(asset))
        } else {
            ItemBase::Simple(Arc::<ItemDef>::load_cloned(asset)?)
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
                ItemBase::Simple(item_def) => ItemBase::Simple(Arc::clone(item_def)),
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

    pub fn stacked_duplicates<'a>(
        &'a self,
        ability_map: &'a AbilityMap,
        msm: &'a MaterialStatManifest,
        count: u32,
    ) -> impl Iterator<Item = Self> + 'a {
        let max_stack_count = count / self.max_amount();
        let rest = count % self.max_amount();

        (0..max_stack_count)
            .map(|_| {
                let mut item = self.duplicate(ability_map, msm);

                item.set_amount(item.max_amount())
                    .expect("max_amount() is always a valid amount.");

                item
            })
            .chain((rest > 0).then(move || {
                let mut item = self.duplicate(ability_map, msm);

                item.set_amount(rest)
                    .expect("anything less than max_amount() is always a valid amount.");

                item
            }))
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

    pub fn item_definition_id(&self) -> ItemDefinitionId<'_> {
        match &self.item_base {
            ItemBase::Simple(item_def) => {
                if self.components.is_empty() {
                    ItemDefinitionId::Simple(&item_def.item_definition_id)
                } else {
                    ItemDefinitionId::Compound {
                        simple_base: &item_def.item_definition_id,
                        components: self
                            .components
                            .iter()
                            .map(|item| item.item_definition_id())
                            .collect(),
                    }
                }
            },
            ItemBase::Modular(mod_base) => ItemDefinitionId::Modular {
                pseudo_base: mod_base.pseudo_item_id(),
                components: self
                    .components
                    .iter()
                    .map(|item| item.item_definition_id())
                    .collect(),
            },
        }
    }

    pub fn is_same_item_def(&self, item_def: &ItemDef) -> bool {
        if let ItemBase::Simple(self_def) = &self.item_base {
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
            .any(|tag| matches!(tag, ItemTag::SalvageInto(_, _)))
    }

    pub fn salvage_output(&self) -> impl Iterator<Item = (&str, u32)> {
        self.tags().into_iter().filter_map(|tag| {
            if let ItemTag::SalvageInto(material, quantity) = tag {
                material
                    .asset_identifier()
                    .map(|material_id| (material_id, quantity))
            } else {
                None
            }
        })
    }

    #[deprecated = "since item i18n"]
    pub fn name(&self) -> Cow<str> {
        match &self.item_base {
            ItemBase::Simple(item_def) => {
                if self.components.is_empty() {
                    #[allow(deprecated)]
                    Cow::Borrowed(&item_def.name)
                } else {
                    #[allow(deprecated)]
                    modular::modify_name(&item_def.name, self)
                }
            },
            ItemBase::Modular(mod_base) => mod_base.generate_name(self.components()),
        }
    }

    #[deprecated = "since item i18n"]
    pub fn description(&self) -> &str {
        match &self.item_base {
            #[allow(deprecated)]
            ItemBase::Simple(item_def) => &item_def.description,
            // TODO: See if James wanted to make description, else leave with none
            ItemBase::Modular(_) => "",
        }
    }

    pub fn kind(&self) -> Cow<ItemKind> {
        match &self.item_base {
            ItemBase::Simple(item_def) => Cow::Borrowed(&item_def.kind),
            ItemBase::Modular(mod_base) => {
                // TODO: Try to move further upward
                let msm = MaterialStatManifest::load().read();
                mod_base.kind(self.components(), &msm, self.stats_durability_multiplier())
            },
        }
    }

    pub fn amount(&self) -> u32 { u32::from(self.amount) }

    pub fn is_stackable(&self) -> bool {
        match &self.item_base {
            ItemBase::Simple(item_def) => item_def.is_stackable(),
            // TODO: Let whoever implements stackable modular items deal with this
            ItemBase::Modular(_) => false,
        }
    }

    /// Return `true` if `other` can be merged into this item. This is generally
    /// only possible if the item has a compatible item ID and is stackable,
    /// along with any other similarity checks.
    pub fn can_merge(&self, other: &Item) -> bool {
        if self.is_stackable()
            && let ItemBase::Simple(other_item_def) = &other.item_base
            && self.is_same_item_def(other_item_def)
            && u32::from(self.amount)
                .checked_add(other.amount())
                .filter(|&amount| amount <= self.max_amount())
                .is_some()
        {
            true
        } else {
            false
        }
    }

    /// Try to merge `other` into this item. This is generally only possible if
    /// the item has a compatible item ID and is stackable, along with any
    /// other similarity checks.
    pub fn try_merge(&mut self, other: Item) -> Result<(), Item> {
        if self.can_merge(&other) {
            self.increase_amount(other.amount())
                .expect("`can_merge` succeeded but `increase_amount` did not");
            Ok(())
        } else {
            Err(other)
        }
    }

    pub fn num_slots(&self) -> u16 { self.item_base.num_slots() }

    /// NOTE: invariant that amount() ≤ max_amount(), 1 ≤ max_amount(),
    /// and if !self.is_stackable(), self.max_amount() = 1.
    pub fn max_amount(&self) -> u32 { if self.is_stackable() { u32::MAX } else { 1 } }

    pub fn quality(&self) -> Quality {
        match &self.item_base {
            ItemBase::Simple(item_def) => item_def.quality.max(
                self.components
                    .iter()
                    .fold(Quality::MIN, |a, b| a.max(b.quality())),
            ),
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

    pub fn try_reclaim_from_block(block: Block) -> Option<Vec<(u32, Self)>> {
        block.get_sprite()?.collectible_id()??.to_items()
    }

    pub fn ability_spec(&self) -> Option<Cow<AbilitySpec>> {
        match &self.item_base {
            ItemBase::Simple(item_def) => {
                item_def.ability_spec.as_ref().map(Cow::Borrowed).or({
                    // If no custom ability set is specified, fall back to abilityset of tool
                    // kind.
                    if let ItemKind::Tool(tool) = &item_def.kind {
                        Some(Cow::Owned(AbilitySpec::Tool(tool.kind)))
                    } else {
                        None
                    }
                })
            },
            ItemBase::Modular(mod_base) => mod_base.ability_spec(self.components()),
        }
    }

    // TODO: Maybe try to make slice again instead of vec? Could also try to make an
    // iterator?
    pub fn tags(&self) -> Vec<ItemTag> {
        match &self.item_base {
            ItemBase::Simple(item_def) => item_def.tags.to_vec(),
            // TODO: Do this properly. It'll probably be important at some point.
            ItemBase::Modular(mod_base) => mod_base.generate_tags(self.components()),
        }
    }

    pub fn is_modular(&self) -> bool {
        match &self.item_base {
            ItemBase::Simple(_) => false,
            ItemBase::Modular(_) => true,
        }
    }

    pub fn item_hash(&self) -> u64 { self.hash }

    pub fn persistence_item_id(&self) -> &str {
        match &self.item_base {
            ItemBase::Simple(item_def) => &item_def.item_definition_id,
            ItemBase::Modular(mod_base) => mod_base.pseudo_item_id(),
        }
    }

    pub fn durability_lost(&self) -> Option<u32> {
        self.durability_lost.map(|x| x.min(Self::MAX_DURABILITY))
    }

    pub fn stats_durability_multiplier(&self) -> DurabilityMultiplier {
        let durability_lost = self.durability_lost.unwrap_or(0);
        debug_assert!(durability_lost <= Self::MAX_DURABILITY);
        // How much durability must be lost before stats start to decay
        const DURABILITY_THRESHOLD: u32 = 9;
        const MIN_FRAC: f32 = 0.25;
        let mult = (1.0
            - durability_lost.saturating_sub(DURABILITY_THRESHOLD) as f32
                / (Self::MAX_DURABILITY - DURABILITY_THRESHOLD) as f32)
            * (1.0 - MIN_FRAC)
            + MIN_FRAC;
        DurabilityMultiplier(mult)
    }

    pub fn has_durability(&self) -> bool {
        self.kind().has_durability() && self.quality() != Quality::Debug
    }

    pub fn increment_damage(&mut self, ability_map: &AbilityMap, msm: &MaterialStatManifest) {
        if let Some(durability_lost) = &mut self.durability_lost {
            if *durability_lost < Self::MAX_DURABILITY {
                *durability_lost += 1;
            }
        }
        // Update item state after applying durability because stats have potential to
        // change from different durability
        self.update_item_state(ability_map, msm);
    }

    pub fn persistence_durability(&self) -> Option<NonZeroU32> {
        self.durability_lost.and_then(NonZeroU32::new)
    }

    pub fn persistence_set_durability(&mut self, value: Option<NonZeroU32>) {
        // If changes have been made so that item no longer needs to track durability,
        // set to None
        if !self.has_durability() {
            self.durability_lost = None;
        } else {
            // Set durability to persisted value, and if item previously had no durability,
            // set to Some(0) so that durability will be tracked
            self.durability_lost = Some(value.map_or(0, NonZeroU32::get));
        }
    }

    pub fn reset_durability(&mut self, ability_map: &AbilityMap, msm: &MaterialStatManifest) {
        self.durability_lost = self.has_durability().then_some(0);
        // Update item state after applying durability because stats have potential to
        // change from different durability
        self.update_item_state(ability_map, msm);
    }

    /// If an item is stackable and has an amount greater than 1, creates a new
    /// item with half the amount (rounded down), and decreases the amount of
    /// the original item by the same quantity.
    #[must_use = "Returned items will be lost if not used"]
    pub fn take_half(
        &mut self,
        ability_map: &AbilityMap,
        msm: &MaterialStatManifest,
    ) -> Option<Item> {
        if self.is_stackable() && self.amount() > 1 {
            let mut return_item = self.duplicate(ability_map, msm);
            let returning_amount = self.amount() / 2;
            self.decrease_amount(returning_amount).ok()?;
            return_item.set_amount(returning_amount).expect(
                "return_item.amount() = self.amount() / 2 < self.amount() (since self.amount() ≥ \
                 1) ≤ self.max_amount() = return_item.max_amount(), since return_item is a \
                 duplicate of item",
            );
            Some(return_item)
        } else {
            None
        }
    }

    #[cfg(test)]
    pub fn create_test_item_from_kind(kind: ItemKind) -> Self {
        let ability_map = &AbilityMap::load().read();
        let msm = &MaterialStatManifest::load().read();
        Self::new_from_item_base(
            ItemBase::Simple(Arc::new(ItemDef::create_test_itemdef_from_kind(kind))),
            Vec::new(),
            ability_map,
            msm,
        )
    }
}

pub fn flatten_counted_items<'a>(
    items: &'a [(u32, Item)],
    ability_map: &'a AbilityMap,
    msm: &'a MaterialStatManifest,
) -> impl Iterator<Item = Item> + 'a {
    items
        .iter()
        .flat_map(|(count, item)| item.stacked_duplicates(ability_map, msm, *count))
}

/// Provides common methods providing details about an item definition
/// for either an `Item` containing the definition, or the actual `ItemDef`
pub trait ItemDesc {
    #[deprecated = "since item i18n"]
    fn description(&self) -> &str;
    #[deprecated = "since item i18n"]
    fn name(&self) -> Cow<str>;
    fn kind(&self) -> Cow<ItemKind>;
    fn amount(&self) -> NonZeroU32;
    fn quality(&self) -> Quality;
    fn num_slots(&self) -> u16;
    fn item_definition_id(&self) -> ItemDefinitionId<'_>;
    fn tags(&self) -> Vec<ItemTag>;
    fn is_modular(&self) -> bool;
    fn components(&self) -> &[Item];
    fn has_durability(&self) -> bool;
    fn durability_lost(&self) -> Option<u32>;
    fn stats_durability_multiplier(&self) -> DurabilityMultiplier;

    fn tool_info(&self) -> Option<ToolKind> {
        if let ItemKind::Tool(tool) = &*self.kind() {
            Some(tool.kind)
        } else {
            None
        }
    }

    /// Return name's and description's localization descriptors
    fn i18n(&self, i18n: &ItemI18n) -> (Content, Content) {
        let item_key: ItemKey = self.into();

        #[allow(deprecated)]
        i18n.item_text_opt(item_key).unwrap_or_else(|| {
            (
                Content::Plain(self.name().to_string()),
                Content::Plain(self.description().to_string()),
            )
        })
    }
}

impl ItemDesc for Item {
    fn description(&self) -> &str {
        #[allow(deprecated)]
        self.description()
    }

    fn name(&self) -> Cow<str> {
        #[allow(deprecated)]
        self.name()
    }

    fn kind(&self) -> Cow<ItemKind> { self.kind() }

    fn amount(&self) -> NonZeroU32 { self.amount }

    fn quality(&self) -> Quality { self.quality() }

    fn num_slots(&self) -> u16 { self.num_slots() }

    fn item_definition_id(&self) -> ItemDefinitionId<'_> { self.item_definition_id() }

    fn tags(&self) -> Vec<ItemTag> { self.tags() }

    fn is_modular(&self) -> bool { self.is_modular() }

    fn components(&self) -> &[Item] { self.components() }

    fn has_durability(&self) -> bool { self.has_durability() }

    fn durability_lost(&self) -> Option<u32> { self.durability_lost() }

    fn stats_durability_multiplier(&self) -> DurabilityMultiplier {
        self.stats_durability_multiplier()
    }
}

impl ItemDesc for ItemDef {
    fn description(&self) -> &str {
        #[allow(deprecated)]
        &self.description
    }

    fn name(&self) -> Cow<str> {
        #[allow(deprecated)]
        Cow::Borrowed(&self.name)
    }

    fn kind(&self) -> Cow<ItemKind> { Cow::Borrowed(&self.kind) }

    fn amount(&self) -> NonZeroU32 { NonZeroU32::new(1).unwrap() }

    fn quality(&self) -> Quality { self.quality }

    fn num_slots(&self) -> u16 { self.slots }

    fn item_definition_id(&self) -> ItemDefinitionId<'_> {
        ItemDefinitionId::Simple(&self.item_definition_id)
    }

    fn tags(&self) -> Vec<ItemTag> { self.tags.to_vec() }

    fn is_modular(&self) -> bool { false }

    fn components(&self) -> &[Item] { &[] }

    fn has_durability(&self) -> bool {
        self.kind().has_durability() && self.quality != Quality::Debug
    }

    fn durability_lost(&self) -> Option<u32> { None }

    fn stats_durability_multiplier(&self) -> DurabilityMultiplier { DurabilityMultiplier(1.0) }
}

impl Component for Item {
    type Storage = DerefFlaggedStorage<Self, DenseVecStorage<Self>>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemDrops(pub Vec<(u32, Item)>);

impl Component for ItemDrops {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Copy, Clone, Debug)]
pub struct DurabilityMultiplier(pub f32);

impl<'a, T: ItemDesc + ?Sized> ItemDesc for &'a T {
    fn description(&self) -> &str {
        #[allow(deprecated)]
        (*self).description()
    }

    fn name(&self) -> Cow<str> {
        #[allow(deprecated)]
        (*self).name()
    }

    fn kind(&self) -> Cow<ItemKind> { (*self).kind() }

    fn amount(&self) -> NonZeroU32 { (*self).amount() }

    fn quality(&self) -> Quality { (*self).quality() }

    fn num_slots(&self) -> u16 { (*self).num_slots() }

    fn item_definition_id(&self) -> ItemDefinitionId<'_> { (*self).item_definition_id() }

    fn tags(&self) -> Vec<ItemTag> { (*self).tags() }

    fn is_modular(&self) -> bool { (*self).is_modular() }

    fn components(&self) -> &[Item] { (*self).components() }

    fn has_durability(&self) -> bool { (*self).has_durability() }

    fn durability_lost(&self) -> Option<u32> { (*self).durability_lost() }

    fn stats_durability_multiplier(&self) -> DurabilityMultiplier {
        (*self).stats_durability_multiplier()
    }
}

/// Returns all item asset specifiers
///
/// Panics in case of filesystem errors
pub fn all_item_defs_expect() -> Vec<String> {
    try_all_item_defs().expect("Failed to access items directory")
}

/// Returns all item asset specifiers
pub fn try_all_item_defs() -> Result<Vec<String>, Error> {
    let defs = assets::load_rec_dir::<RawItemDef>("common.items")?;
    Ok(defs.read().ids().map(|id| id.to_string()).collect())
}

/// Designed to return all possible items, including modulars.
/// And some impossible too, like ItemKind::TagExamples.
pub fn all_items_expect() -> Vec<Item> {
    let defs = assets::load_rec_dir::<RawItemDef>("common.items")
        .expect("failed to load item asset directory");

    // Grab all items from assets
    let mut asset_items: Vec<Item> = defs
        .read()
        .ids()
        .map(|id| Item::new_from_asset_expect(id))
        .collect();

    let mut material_parse_table = HashMap::new();
    for mat in Material::iter() {
        if let Some(id) = mat.asset_identifier() {
            material_parse_table.insert(id.to_owned(), mat);
        }
    }

    let primary_comp_pool = modular::PRIMARY_COMPONENT_POOL.clone();

    // Grab weapon primary components
    let mut primary_comps: Vec<Item> = primary_comp_pool
        .values()
        .flatten()
        .map(|(item, _hand_rules)| item.clone())
        .collect();

    // Grab modular weapons
    let mut modular_items: Vec<Item> = primary_comp_pool
        .keys()
        .flat_map(|(tool, mat_id)| {
            let mat = material_parse_table
                .get(mat_id)
                .expect("unexpected material ident");

            // get all weapons without imposing additional hand restrictions
            modular::generate_weapons(*tool, *mat, None)
                .expect("failure during modular weapon generation")
        })
        .collect();

    // 1. Append asset items, that should include pretty much everything,
    // except modular items
    // 2. Append primary weapon components, which are modular as well.
    // 3. Finally append modular weapons that are made from (1) and (2)
    // extend when we get some new exotic stuff
    //
    // P. s. I still can't wrap my head around the idea that you can put
    // tag example into your inventory.
    let mut all = Vec::new();
    all.append(&mut asset_items);
    all.append(&mut primary_comps);
    all.append(&mut modular_items);

    all
}

impl PartialEq<ItemDefinitionId<'_>> for ItemDefinitionIdOwned {
    fn eq(&self, other: &ItemDefinitionId<'_>) -> bool {
        use ItemDefinitionId as DefId;
        match self {
            Self::Simple(simple) => {
                matches!(other, DefId::Simple(other_simple) if simple == other_simple)
            },
            Self::Modular {
                pseudo_base,
                components,
            } => matches!(
                other,
                DefId::Modular { pseudo_base: other_base, components: other_comps }
                if pseudo_base == other_base && components == other_comps
            ),
            Self::Compound {
                simple_base,
                components,
            } => matches!(
                other,
                DefId::Compound { simple_base: other_base, components: other_comps }
                if simple_base == other_base && components == other_comps
            ),
        }
    }
}

impl PartialEq<ItemDefinitionIdOwned> for ItemDefinitionId<'_> {
    #[inline]
    fn eq(&self, other: &ItemDefinitionIdOwned) -> bool { other == self }
}

impl Equivalent<ItemDefinitionIdOwned> for ItemDefinitionId<'_> {
    fn equivalent(&self, key: &ItemDefinitionIdOwned) -> bool { self == key }
}

impl From<&ItemDefinitionId<'_>> for ItemDefinitionIdOwned {
    fn from(value: &ItemDefinitionId<'_>) -> Self { value.to_owned() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assets_items() {
        let ids = all_item_defs_expect();
        for item in ids.iter().map(|id| Item::new_from_asset_expect(id)) {
            drop(item)
        }
    }

    #[test]
    fn test_item_i18n() { let _ = ItemI18n::new_expect(); }

    #[test]
    // Probably can't fail, but better safe than crashing production server
    fn test_all_items() { let _ = all_items_expect(); }

    #[test]
    // All items in Veloren should have localization.
    // If no, add some common dummy i18n id.
    fn ensure_item_localization() {
        let manifest = ItemI18n::new_expect();
        let items = all_items_expect();
        for item in items {
            let item_key: ItemKey = (&item).into();
            let _ = manifest.item_text_opt(item_key).unwrap();
        }
    }
}
