use super::{
    tool::{self, AbilityMap, AbilitySpec, Hands, MaterialStatManifest},
    Item, ItemBase, ItemDef, ItemDesc, ItemKind, Material, Quality, ToolKind,
};
use crate::{assets::AssetExt, recipe};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use rand::{prelude::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, sync::Arc};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ModularBase {
    Tool,
}

impl ModularBase {
    // DO NOT CHANGE. THIS IS A PERSISTENCE RELATED FUNCTION. MUST MATCH THE
    // FUNCTION BELOW.
    pub fn pseudo_item_id(&self) -> &str {
        match self {
            ModularBase::Tool => "veloren.core.pseudo_items.modular.tool",
        }
    }

    // DO NOT CHANGE. THIS IS A PERSISTENCE RELATED FUNCTION. MUST MATCH THE
    // FUNCTION ABOVE.
    pub fn load_from_pseudo_id(id: &str) -> Self {
        match id {
            "veloren.core.pseudo_items.modular.tool" => ModularBase::Tool,
            _ => panic!("Attempted to load a non existent pseudo item: {}", id),
        }
    }

    pub fn kind(&self, components: &[Item], msm: &MaterialStatManifest) -> Cow<ItemKind> {
        fn resolve_hands(components: &[Item]) -> Hands {
            // Checks if weapon has components that restrict hands to two. Restrictions to
            // one hand or no restrictions default to one-handed weapon.
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

        pub fn resolve_stats(components: &[Item], msm: &MaterialStatManifest) -> tool::Stats {
            components
                .iter()
                .filter_map(|comp| {
                    if let ItemKind::ModularComponent(mod_comp) = &*comp.kind() {
                        mod_comp.tool_stats(comp.components(), msm)
                    } else {
                        None
                    }
                })
                .fold(tool::Stats::one(), |a, b| a * b)
        }

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

        match self {
            ModularBase::Tool => Cow::Owned(ItemKind::Tool(tool::Tool {
                kind: toolkind,
                hands: resolve_hands(components),
                stats: resolve_stats(components, msm),
            })),
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
                                    Cow::Owned(ItemKind::Ingredient { descriptor, .. }) => {
                                        Some(Cow::Owned(descriptor))
                                    },
                                    Cow::Borrowed(ItemKind::Ingredient { descriptor, .. }) => {
                                        Some(Cow::Borrowed(descriptor.as_str()))
                                    },
                                    _ => None,
                                })
                                .unwrap_or_else(|| "Modular".into());
                            Some(format!("{} {}", material_name, weapon_name))
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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ModularComponent {
    ToolPrimaryComponent {
        toolkind: ToolKind,
        stats: tool::Stats,
        hand_restriction: Option<Hands>,
        weapon_name: String,
    },
    ToolSecondaryComponent {
        toolkind: ToolKind,
        stats: tool::Stats,
        hand_restriction: Option<Hands>,
    },
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
                    .filter_map(|comp| msm.0.get(comp.item_definition_id()).copied().zip(Some(1)))
                    .reduce(|(stats_a, count_a), (stats_b, count_b)| {
                        (stats_a + stats_b, count_a + count_b)
                    })
                    .map_or_else(tool::Stats::one, |(stats_sum, count)| stats_sum / count);

                Some(*stats * average_material_mult)
            },
            Self::ToolSecondaryComponent { stats, .. } => Some(*stats),
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

type PrimaryComponentPool = HashMap<(ToolKind, String), Vec<(Arc<ItemDef>, Option<Hands>)>>;
type SecondaryComponentPool = HashMap<ToolKind, Vec<(Arc<ItemDef>, Option<Hands>)>>;

lazy_static! {
    static ref PRIMARY_COMPONENT_POOL: PrimaryComponentPool = {
        let mut component_pool = HashMap::new();

        // Load recipe book (done to check that material is valid for a particular component)
        let recipe_book = recipe::RawRecipeBook::load_expect("common.recipe_book");
        let recipes = &recipe_book.read().0;

        const ASSET_PREFIX: &str = "common.items.crafting_ing.modular.primary";

        // Closure to check that an Item has a recipe that uses the provided material
        let valid_materials = |item: &str| {
            // Iterate over all recipes in the raw recipe book
            recipes
                .values()
                // Filter by recipes that have an output of the item of interest
                .filter(|recipe| recipe.output.0.eq(item))
                // Check that item is composed of material, uses heuristic that assumes all modular components use the ListSameItem recipe input
                .find_map(|recipe| {
                    recipe
                        .inputs
                        .iter()
                        .find_map(|input| {
                            match &input.0 {
                                recipe::RawRecipeInput::ListSameItem(items) => {
                                    Some(recipe::ItemList::load_expect_cloned(items).0)
                                },
                                _ => None,
                            }
                        })
                })
        };

        for toolkind in SUPPORTED_TOOLKINDS {
            let directory = format!("{}.{}", ASSET_PREFIX, toolkind.identifier_name());
            if let Ok(items) = Item::new_from_asset_glob(&directory) {
                items
                    .into_iter()
                    .map(|comp| comp.item_definition_id().to_owned())
                    .filter_map(|id| Arc::<ItemDef>::load_cloned(&id).ok())
                    .for_each(|comp_def| {
                        if let ItemKind::ModularComponent(ModularComponent::ToolPrimaryComponent { hand_restriction, .. }) = comp_def.kind {
                            if let Some(material_ids) = valid_materials(comp_def.id()) {
                                for material in material_ids {
                                        let entry = component_pool.entry((toolkind, material)).or_insert(Vec::new());
                                        entry.push((Arc::clone(&comp_def), hand_restriction));
                                    }
                                }
                            }
                        }
                    );
            }
        }

        component_pool
    };

    static ref SECONDARY_COMPONENT_POOL: SecondaryComponentPool = {
        let mut component_pool = HashMap::new();

        const ASSET_PREFIX: &str = "common.items.crafting_ing.modular.secondary";

        for toolkind in SUPPORTED_TOOLKINDS {
            let directory = format!("{}.{}", ASSET_PREFIX, toolkind.identifier_name());
            if let Ok(items) = Item::new_from_asset_glob(&directory) {
                items
                    .into_iter()
                    .map(|comp| comp.item_definition_id().to_owned())
                    .filter_map(|id| Arc::<ItemDef>::load_cloned(&id).ok())
                    .for_each(|comp_def| {
                        if let ItemKind::ModularComponent(ModularComponent::ToolSecondaryComponent { hand_restriction, .. }) = comp_def.kind {
                            let entry = component_pool.entry(toolkind).or_insert(Vec::new());
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

/// Creates a random modular weapon when provided with a toolkind, material, and
/// optionally the handedness
pub fn random_weapon(
    tool: ToolKind,
    material: Material,
    hand_restriction: Option<Hands>,
) -> Result<Item, ModularWeaponCreationError> {
    if let Some(material_id) = material.asset_identifier() {
        // Loads default ability map and material stat manifest for later use
        let ability_map = AbilityMap::load().read();
        let msm = MaterialStatManifest::load().read();

        let mut rng = thread_rng();

        let material = Item::new_from_asset_expect(material_id);
        let primary_components = PRIMARY_COMPONENT_POOL
            .get(&(tool, material_id.to_owned()))
            .into_iter()
            .flatten()
            .filter(|(_def, hand)| match (hand_restriction, hand) {
                (Some(restriction), Some(hand)) => restriction == *hand,
                (None, _) | (_, None) => true,
            })
            .collect::<Vec<_>>();

        let (primary_component, hand_restriction) = {
            let (def, hand) = primary_components
                .choose(&mut rng)
                .ok_or(ModularWeaponCreationError::PrimaryComponentNotFound)?;
            let comp = Item::new_from_item_base(
                ItemBase::Raw(Arc::clone(def)),
                vec![material],
                &ability_map,
                &msm,
            );
            (comp, hand_restriction.or(*hand))
        };

        let secondary_components = SECONDARY_COMPONENT_POOL
            .get(&tool)
            .into_iter()
            .flatten()
            .filter(|(_def, hand)| match (hand_restriction, hand) {
                (Some(restriction), Some(hand)) => restriction == *hand,
                (None, _) | (_, None) => true,
            })
            .collect::<Vec<_>>();

        let secondary_component = {
            let def = &secondary_components
                .choose(&mut rng)
                .ok_or(ModularWeaponCreationError::SecondaryComponentNotFound)?
                .0;
            Item::new_from_item_base(
                ItemBase::Raw(Arc::clone(def)),
                Vec::new(),
                &ability_map,
                &msm,
            )
        };

        // Create modular weapon
        Ok(Item::new_from_item_base(
            ItemBase::Modular(ModularBase::Tool),
            vec![primary_component, secondary_component],
            &ability_map,
            &msm,
        ))
    } else {
        Err(ModularWeaponCreationError::MaterialNotFound)
    }
}

pub fn modify_name<'a>(item_name: &'a str, item: &'a Item) -> Cow<'a, str> {
    if let ItemKind::ModularComponent(_) = &*item.kind() {
        if let Some(material_name) = item
            .components()
            .iter()
            .find_map(|comp| match &*comp.kind() {
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

pub fn weapon_to_key(mod_weap: &dyn ItemDesc) -> ModularWeaponKey {
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
                let component_id = comp.item_definition_id().to_owned();
                let material_id = comp.components().iter().find_map(|mat| match &*mat.kind() {
                    ItemKind::Ingredient { .. } => Some(mat.item_definition_id().to_owned()),
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
        ItemKind::Ingredient { .. } => Some(mat.item_definition_id().to_owned()),
        _ => None,
    }) {
        Some(material_id) => Ok((item_def_id.to_owned(), material_id)),
        None => Err(ModularWeaponComponentKeyError::MaterialNotFound),
    }
}
