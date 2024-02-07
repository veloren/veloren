use super::{
    armor,
    tool::{self, AbilityMap, AbilitySpec, Hands, Tool},
    DurabilityMultiplier, Item, ItemBase, ItemDef, ItemDesc, ItemKind, ItemTag, Material, Quality,
    ToolKind,
};
use crate::{
    assets::{self, Asset, AssetExt, AssetHandle},
    recipe,
};
use common_base::dev_panic;
use hashbrown::HashMap;
use lazy_static::lazy_static;
use rand::{prelude::SliceRandom, Rng};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, sync::Arc};

// Macro instead of constant to work with concat! macro.
// DO NOT CHANGE. THIS PREFIX AFFECTS PERSISTENCE AND IF CHANGED A MIGRATION
// MUST BE PERFORMED.
#[macro_export]
macro_rules! modular_item_id_prefix {
    () => {
        "veloren.core.pseudo_items.modular."
    };
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaterialStatManifest {
    tool_stats: HashMap<String, tool::Stats>,
    armor_stats: HashMap<String, armor::Stats>,
}

impl MaterialStatManifest {
    pub fn load() -> AssetHandle<Self> { Self::load_expect("common.material_stats_manifest") }

    pub fn armor_stats(&self, key: &str) -> Option<armor::Stats> {
        self.armor_stats.get(key).copied()
    }

    #[doc(hidden)]
    /// needed for tests to load it without actual assets
    pub fn with_empty() -> Self {
        Self {
            tool_stats: HashMap::default(),
            armor_stats: HashMap::default(),
        }
    }
}

// This could be a Compound that also loads the keys, but the RecipeBook
// Compound impl already does that, so checking for existence here is redundant.
impl Asset for MaterialStatManifest {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ModularBase {
    Tool,
}

impl ModularBase {
    // DO NOT CHANGE. THIS IS A PERSISTENCE RELATED FUNCTION. MUST MATCH THE
    // FUNCTION BELOW.
    pub fn pseudo_item_id(&self) -> &str {
        match self {
            ModularBase::Tool => concat!(modular_item_id_prefix!(), "tool"),
        }
    }

    // DO NOT CHANGE. THIS IS A PERSISTENCE RELATED FUNCTION. MUST MATCH THE
    // FUNCTION ABOVE.
    pub fn load_from_pseudo_id(id: &str) -> Self {
        match id {
            concat!(modular_item_id_prefix!(), "tool") => ModularBase::Tool,
            _ => panic!("Attempted to load a non existent pseudo item: {}", id),
        }
    }

    fn resolve_hands(components: &[Item]) -> Hands {
        // Checks if weapon has components that restrict hands to two. Restrictions to
        // one hand or no restrictions default to one-handed weapon.
        // Note: Hand restriction here will never conflict on components
        // TODO: Maybe look into adding an assert at some point?
        let hand_restriction = components.iter().find_map(|comp| match &*comp.kind() {
            ItemKind::ModularComponent(mc) => match mc {
                ModularComponent::ToolPrimaryComponent {
                    hand_restriction, ..
                }
                | ModularComponent::ToolSecondaryComponent {
                    hand_restriction, ..
                } => *hand_restriction,
            },
            _ => None,
        });
        // In the event of no hand restrictions on the components, default to one handed
        hand_restriction.unwrap_or(Hands::One)
    }

    #[inline(never)]
    pub(super) fn kind(
        &self,
        components: &[Item],
        msm: &MaterialStatManifest,
        durability_multiplier: DurabilityMultiplier,
    ) -> Cow<ItemKind> {
        let toolkind = components
            .iter()
            .find_map(|comp| match &*comp.kind() {
                ItemKind::ModularComponent(ModularComponent::ToolPrimaryComponent {
                    toolkind,
                    ..
                }) => Some(*toolkind),
                _ => None,
            })
            .unwrap_or(ToolKind::Empty);

        let stats: tool::Stats = components
            .iter()
            .filter_map(|comp| {
                if let ItemKind::ModularComponent(mod_comp) = &*comp.kind() {
                    mod_comp.tool_stats(comp.components(), msm)
                } else {
                    None
                }
            })
            .fold(tool::Stats::one(), |a, b| a * b)
            * durability_multiplier;

        match self {
            ModularBase::Tool => Cow::Owned(ItemKind::Tool(Tool::new(
                toolkind,
                Self::resolve_hands(components),
                stats,
            ))),
        }
    }

    /// Modular weapons are named as "{Material} {Weapon}" where {Weapon} is
    /// from the damage component used and {Material} is from the material
    /// the damage component is created from.
    pub fn generate_name(&self, components: &[Item]) -> Cow<str> {
        match self {
            ModularBase::Tool => {
                let name = components
                    .iter()
                    .find_map(|comp| match &*comp.kind() {
                        ItemKind::ModularComponent(ModularComponent::ToolPrimaryComponent {
                            weapon_name,
                            ..
                        }) => {
                            let material_name = comp
                                .components()
                                .iter()
                                .find_map(|mat| match mat.kind() {
                                    #[allow(deprecated)]
                                    Cow::Owned(ItemKind::Ingredient { descriptor, .. }) => {
                                        Some(Cow::Owned(descriptor))
                                    },
                                    #[allow(deprecated)]
                                    Cow::Borrowed(ItemKind::Ingredient { descriptor, .. }) => {
                                        Some(Cow::Borrowed(descriptor.as_str()))
                                    },
                                    _ => None,
                                })
                                .unwrap_or_else(|| "Modular".into());
                            Some(format!(
                                "{} {}",
                                material_name,
                                weapon_name.resolve_name(Self::resolve_hands(components))
                            ))
                        },
                        _ => None,
                    })
                    .unwrap_or_else(|| "Modular Weapon".to_owned());
                Cow::Owned(name)
            },
        }
    }

    pub fn compute_quality(&self, components: &[Item]) -> Quality {
        components
            .iter()
            .fold(Quality::MIN, |a, b| a.max(b.quality()))
    }

    pub fn ability_spec(&self, components: &[Item]) -> Option<Cow<AbilitySpec>> {
        match self {
            ModularBase::Tool => components.iter().find_map(|comp| match &*comp.kind() {
                ItemKind::ModularComponent(ModularComponent::ToolPrimaryComponent {
                    toolkind,
                    ..
                }) => Some(Cow::Owned(AbilitySpec::Tool(*toolkind))),
                _ => None,
            }),
        }
    }

    pub fn generate_tags(&self, components: &[Item]) -> Vec<ItemTag> {
        match self {
            ModularBase::Tool => {
                if let Some(comp) = components.iter().find(|comp| {
                    matches!(
                        &*comp.kind(),
                        ItemKind::ModularComponent(ModularComponent::ToolPrimaryComponent { .. })
                    )
                }) {
                    if let Some(material) =
                        comp.components()
                            .iter()
                            .find_map(|comp| match &*comp.kind() {
                                ItemKind::Ingredient { .. } => {
                                    comp.tags().into_iter().find_map(|tag| match tag {
                                        ItemTag::Material(material) => Some(material),
                                        _ => None,
                                    })
                                },
                                _ => None,
                            })
                    {
                        vec![
                            ItemTag::Material(material),
                            ItemTag::SalvageInto(material, 1),
                        ]
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ModularComponent {
    ToolPrimaryComponent {
        toolkind: ToolKind,
        stats: tool::Stats,
        hand_restriction: Option<Hands>,
        weapon_name: WeaponName,
    },
    ToolSecondaryComponent {
        toolkind: ToolKind,
        stats: tool::Stats,
        hand_restriction: Option<Hands>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeaponName {
    Universal(String),
    HandednessDependent {
        one_handed: String,
        two_handed: String,
    },
}

impl WeaponName {
    fn resolve_name(&self, handedness: Hands) -> &str {
        match self {
            Self::Universal(name) => name,
            Self::HandednessDependent {
                one_handed: name1,
                two_handed: name2,
            } => match handedness {
                Hands::One => name1,
                Hands::Two => name2,
            },
        }
    }
}

impl ModularComponent {
    pub fn tool_stats(
        &self,
        components: &[Item],
        msm: &MaterialStatManifest,
    ) -> Option<tool::Stats> {
        match self {
            Self::ToolPrimaryComponent { stats, .. } => {
                let average_material_mult = components
                    .iter()
                    .filter_map(|comp| {
                        comp.item_definition_id()
                            .itemdef_id()
                            .and_then(|id| msm.tool_stats.get(id))
                            .copied()
                            .zip(Some(1))
                    })
                    .reduce(|(stats_a, count_a), (stats_b, count_b)| {
                        (stats_a + stats_b, count_a + count_b)
                    })
                    .map_or_else(tool::Stats::one, |(stats_sum, count)| {
                        stats_sum / (count as f32)
                    });

                Some(*stats * average_material_mult)
            },
            Self::ToolSecondaryComponent { stats, .. } => Some(*stats),
        }
    }

    pub fn toolkind(&self) -> Option<ToolKind> {
        match self {
            Self::ToolPrimaryComponent { toolkind, .. }
            | Self::ToolSecondaryComponent { toolkind, .. } => Some(*toolkind),
        }
    }
}

const SUPPORTED_TOOLKINDS: [ToolKind; 6] = [
    ToolKind::Sword,
    ToolKind::Axe,
    ToolKind::Hammer,
    ToolKind::Bow,
    ToolKind::Staff,
    ToolKind::Sceptre,
];

type PrimaryComponentPool = HashMap<(ToolKind, String), Vec<(Item, Option<Hands>)>>;
type SecondaryComponentPool = HashMap<ToolKind, Vec<(Arc<ItemDef>, Option<Hands>)>>;

lazy_static! {
    pub static ref PRIMARY_COMPONENT_POOL: PrimaryComponentPool = {
        let mut component_pool = HashMap::new();

        // Load recipe book
        // (done to check that material is valid for a particular component)
        use crate::recipe::ComponentKey;
        let recipes = recipe::default_component_recipe_book().read();
        let ability_map = &AbilityMap::load().read();
        let msm = &MaterialStatManifest::load().read();

        recipes.iter().for_each(
            |(
                ComponentKey {
                    toolkind, material, ..
                },
                recipe,
            )| {
                let component = recipe.item_output(ability_map, msm);
                let hand_restriction =
                    if let ItemKind::ModularComponent(ModularComponent::ToolPrimaryComponent {
                        hand_restriction,
                        ..
                    }) = &*component.kind()
                    {
                        *hand_restriction
                    } else {
                        return;
                    };
                let entry: &mut Vec<_> = component_pool
                    .entry((*toolkind, String::from(material)))
                    .or_default();
                entry.push((component, hand_restriction));
            },
        );

        component_pool
    };

    static ref SECONDARY_COMPONENT_POOL: SecondaryComponentPool = {
        let mut component_pool = HashMap::new();

        const ASSET_PREFIX: &str = "common.items.modular.weapon.secondary";

        for toolkind in SUPPORTED_TOOLKINDS {
            let directory = format!("{}.{}", ASSET_PREFIX, toolkind.identifier_name());
            if let Ok(items) = Item::new_from_asset_glob(&directory) {
                items
                    .into_iter()
                    .filter_map(|comp| Some(comp.item_definition_id().itemdef_id()?.to_owned()))
                    .filter_map(|id| Arc::<ItemDef>::load_cloned(&id).ok())
                    .for_each(|comp_def| {
                        if let ItemKind::ModularComponent(
                            ModularComponent::ToolSecondaryComponent {
                                hand_restriction, ..
                            },
                        ) = comp_def.kind
                        {
                            let entry: &mut Vec<_> = component_pool.entry(toolkind).or_default();
                            entry.push((Arc::clone(&comp_def), hand_restriction));
                        }
                    });
            }
        }

        component_pool
    };
}

#[derive(Debug)]
pub enum ModularWeaponCreationError {
    MaterialNotFound,
    PrimaryComponentNotFound,
    SecondaryComponentNotFound,
}

/// Check if hand restrictions are compatible.
///
/// If at least on of them is omitted, check is passed.
pub fn compatible_handedness(a: Option<Hands>, b: Option<Hands>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a == b,
        _ => true,
    }
}

/// Generate all primary components for specific tool and material.
///
/// Read [random_weapon_primary_component] for more.
pub fn generate_weapon_primary_components(
    tool: ToolKind,
    material: Material,
    hand_restriction: Option<Hands>,
) -> Result<Vec<(Item, Option<Hands>)>, ModularWeaponCreationError> {
    if let Some(material_id) = material.asset_identifier() {
        // Loads default ability map and material stat manifest for later use
        let ability_map = &AbilityMap::load().read();
        let msm = &MaterialStatManifest::load().read();

        Ok(PRIMARY_COMPONENT_POOL
            .get(&(tool, material_id.to_owned()))
            .into_iter()
            .flatten()
            .filter(|(_comp, hand)| compatible_handedness(hand_restriction, *hand))
            .map(|(c, h)| (c.duplicate(ability_map, msm), hand_restriction.or(*h)))
            .collect())
    } else {
        Err(ModularWeaponCreationError::MaterialNotFound)
    }
}

/// Creates a random modular weapon primary component when provided with a
/// toolkind, material, and optionally the handedness
///
/// NOTE: The component produced is not necessarily restricted to that
/// handedness, but rather is able to produce a weapon of that handedness
/// depending on what secondary component is used
///
/// Returns the compatible handednesses that can be used with provided
/// restriction and generated component (useful for cases where no restriction
/// was passed in, but generated component has a restriction)
pub fn random_weapon_primary_component(
    tool: ToolKind,
    material: Material,
    hand_restriction: Option<Hands>,
    mut rng: &mut impl Rng,
) -> Result<(Item, Option<Hands>), ModularWeaponCreationError> {
    let result = {
        if let Some(material_id) = material.asset_identifier() {
            // Loads default ability map and material stat manifest for later use
            let ability_map = &AbilityMap::load().read();
            let msm = &MaterialStatManifest::load().read();

            let primary_components = PRIMARY_COMPONENT_POOL
                .get(&(tool, material_id.to_owned()))
                .into_iter()
                .flatten()
                .filter(|(_comp, hand)| compatible_handedness(hand_restriction, *hand))
                .collect::<Vec<_>>();

            let (comp, hand) = primary_components
                .choose(&mut rng)
                .ok_or(ModularWeaponCreationError::PrimaryComponentNotFound)?;
            let comp = comp.duplicate(ability_map, msm);
            Ok((comp, hand_restriction.or(*hand)))
        } else {
            Err(ModularWeaponCreationError::MaterialNotFound)
        }
    };

    if let Err(err) = &result {
        let error_str = format!(
            "Failed to synthesize a primary component for a modular {tool:?} made of {material:?} \
             that had a hand restriction of {hand_restriction:?}. Error: {err:?}"
        );
        dev_panic!(error_str)
    }
    result
}

pub fn generate_weapons(
    tool: ToolKind,
    material: Material,
    hand_restriction: Option<Hands>,
) -> Result<Vec<Item>, ModularWeaponCreationError> {
    // Loads default ability map and material stat manifest for later use
    let ability_map = &AbilityMap::load().read();
    let msm = &MaterialStatManifest::load().read();

    let primaries = generate_weapon_primary_components(tool, material, hand_restriction)?;
    let mut weapons = Vec::new();

    for (comp, comp_hand) in primaries {
        let secondaries = SECONDARY_COMPONENT_POOL
            .get(&tool)
            .into_iter()
            .flatten()
            .filter(|(_def, hand)| compatible_handedness(hand_restriction, *hand))
            .filter(|(_def, hand)| compatible_handedness(comp_hand, *hand));

        for (def, _hand) in secondaries {
            let secondary = Item::new_from_item_base(
                ItemBase::Simple(Arc::clone(def)),
                Vec::new(),
                ability_map,
                msm,
            );
            let it = Item::new_from_item_base(
                ItemBase::Modular(ModularBase::Tool),
                vec![comp.duplicate(ability_map, msm), secondary],
                ability_map,
                msm,
            );
            weapons.push(it);
        }
    }

    Ok(weapons)
}

/// Creates a random modular weapon when provided with a toolkind, material, and
/// optionally the handedness
pub fn random_weapon(
    tool: ToolKind,
    material: Material,
    hand_restriction: Option<Hands>,
    mut rng: &mut impl Rng,
) -> Result<Item, ModularWeaponCreationError> {
    let result = {
        // Loads default ability map and material stat manifest for later use
        let ability_map = &AbilityMap::load().read();
        let msm = &MaterialStatManifest::load().read();

        let (primary_component, hand_restriction) =
            random_weapon_primary_component(tool, material, hand_restriction, rng)?;

        let secondary_components = SECONDARY_COMPONENT_POOL
            .get(&tool)
            .into_iter()
            .flatten()
            .filter(|(_def, hand)| compatible_handedness(hand_restriction, *hand))
            .collect::<Vec<_>>();

        let secondary_component = {
            let def = &secondary_components
                .choose(&mut rng)
                .ok_or(ModularWeaponCreationError::SecondaryComponentNotFound)?
                .0;

            Item::new_from_item_base(
                ItemBase::Simple(Arc::clone(def)),
                Vec::new(),
                ability_map,
                msm,
            )
        };

        // Create modular weapon
        Ok(Item::new_from_item_base(
            ItemBase::Modular(ModularBase::Tool),
            vec![primary_component, secondary_component],
            ability_map,
            msm,
        ))
    };
    if let Err(err) = &result {
        let error_str = format!(
            "Failed to synthesize a modular {tool:?} made of {material:?} that had a hand \
             restriction of {hand_restriction:?}. Error: {err:?}"
        );
        dev_panic!(error_str)
    }
    result
}

pub fn modify_name<'a>(item_name: &'a str, item: &'a Item) -> Cow<'a, str> {
    if let ItemKind::ModularComponent(_) = &*item.kind() {
        if let Some(material_name) = item
            .components()
            .iter()
            .find_map(|comp| match &*comp.kind() {
                #[allow(deprecated)]
                ItemKind::Ingredient { descriptor, .. } => Some(descriptor.to_owned()),
                _ => None,
            })
        {
            Cow::Owned(format!("{} {}", material_name, item_name))
        } else {
            Cow::Borrowed(item_name)
        }
    } else {
        Cow::Borrowed(item_name)
    }
}

/// This is used as a key to uniquely identify the modular weapon in asset
/// manifests in voxygen (Main component, material, hands)
pub type ModularWeaponKey = (String, String, Hands);

pub fn weapon_to_key(mod_weap: impl ItemDesc) -> ModularWeaponKey {
    let hands = if let ItemKind::Tool(tool) = &*mod_weap.kind() {
        tool.hands
    } else {
        Hands::One
    };

    match mod_weap
        .components()
        .iter()
        .find_map(|comp| match &*comp.kind() {
            ItemKind::ModularComponent(ModularComponent::ToolPrimaryComponent { .. }) => {
                let component_id = comp.item_definition_id().itemdef_id()?.to_owned();
                let material_id = comp.components().iter().find_map(|mat| match &*mat.kind() {
                    ItemKind::Ingredient { .. } => {
                        Some(mat.item_definition_id().itemdef_id()?.to_owned())
                    },
                    _ => None,
                });
                Some((component_id, material_id))
            },
            _ => None,
        }) {
        Some((component_id, Some(material_id))) => (component_id, material_id, hands),
        Some((component_id, None)) => (component_id, String::new(), hands),
        None => (String::new(), String::new(), hands),
    }
}

/// This is used as a key to uniquely identify the modular weapon in asset
/// manifests in voxygen (Main component, material)
pub type ModularWeaponComponentKey = (String, String);

pub enum ModularWeaponComponentKeyError {
    MaterialNotFound,
}

pub fn weapon_component_to_key(
    item_def_id: &str,
    components: &[Item],
) -> Result<ModularWeaponComponentKey, ModularWeaponComponentKeyError> {
    match components.iter().find_map(|mat| match &*mat.kind() {
        ItemKind::Ingredient { .. } => Some(mat.item_definition_id().itemdef_id()?.to_owned()),
        _ => None,
    }) {
        Some(material_id) => Ok((item_def_id.to_owned(), material_id)),
        None => Err(ModularWeaponComponentKeyError::MaterialNotFound),
    }
}
