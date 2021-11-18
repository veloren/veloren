use super::{
    tool::{self, AbilitySpec, Hands, MaterialStatManifest, Stats},
    Item, ItemBase, ItemDesc, ItemKind, Quality, ToolKind,
};
use crate::{assets::AssetExt, lottery::Lottery, recipe};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ModularBase {
    Tool(ToolKind),
}

impl ModularBase {
    pub(super) fn duplicate(&self) -> Self {
        match self {
            ModularBase::Tool(toolkind) => ModularBase::Tool(*toolkind),
        }
    }

    pub fn kind(&self, components: &[Item], msm: &MaterialStatManifest) -> Cow<ItemKind> {
        fn resolve_hands(components: &[Item]) -> Hands {
            // Checks if weapon has components that restrict hands to two. Restrictions to
            // one hand or no restrictions default to one-handed weapon.
            let is_two_handed = components.iter().any(|item| matches!(&*item.kind(), ItemKind::ModularComponent(mc) if matches!(mc.hand_restriction, Some(Hands::Two))));
            // If weapon is two handed, make it two handed
            if is_two_handed {
                Hands::Two
            } else {
                Hands::One
            }
        }

        pub fn resolve_stats(components: &[Item], msm: &MaterialStatManifest) -> Stats {
            let mut stats = Stats::one();
            let mut material_multipliers: Vec<Stats> = Vec::new();
            for item in components.iter() {
                match &*item.kind() {
                    // Modular components directly multiply against the base stats
                    ItemKind::ModularComponent(mc) => {
                        let inner_stats = mc.stats * resolve_stats(item.components(), msm);
                        stats *= inner_stats;
                    },
                    // Ingredients push multiplier to vec as the ingredient multipliers are averaged
                    ItemKind::Ingredient { .. } => {
                        if let Some(mult_stats) = msm.0.get(item.item_definition_id()) {
                            material_multipliers.push(*mult_stats);
                        }
                    },
                    _ => (),
                }
            }

            // Take the average of the material multipliers
            if !material_multipliers.is_empty() {
                let mut average_mult = Stats::zero();
                for stat in material_multipliers.iter() {
                    average_mult += *stat;
                }
                average_mult /= material_multipliers.len();
                stats *= average_mult;
            }
            stats
        }

        match self {
            ModularBase::Tool(toolkind) => Cow::Owned(ItemKind::Tool(tool::Tool {
                kind: *toolkind,
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
            ModularBase::Tool(toolkind) => {
                // Closure to get material name from an item
                fn material_name(component: &Item) -> String {
                    component
                        .components()
                        .iter()
                        .filter_map(|comp| match &*comp.kind() {
                            ItemKind::Ingredient { descriptor, .. } => Some(descriptor.to_owned()),
                            _ => None,
                        })
                        .next()
                        .unwrap_or_else(|| "Modular".to_owned())
                }

                let main_component = components.iter().find(|comp| {
                    matches!(&*comp.kind(), ItemKind::ModularComponent(ModularComponent { modkind, .. })
                            if *modkind == ModularComponentKind::main_component(*toolkind)
                    )
                });
                let (material_name, weapon_name) = if let Some(component) = main_component {
                    let material_name = material_name(component);
                    let weapon_name = if let ItemKind::ModularComponent(ModularComponent {
                        weapon_name,
                        ..
                    }) = &*component.kind()
                    {
                        weapon_name.to_owned()
                    } else {
                        toolkind.identifier_name().to_owned()
                    };
                    (material_name, weapon_name)
                } else {
                    ("Modular".to_owned(), toolkind.identifier_name().to_owned())
                };

                Cow::Owned(format!("{} {}", material_name, weapon_name))
            },
        }
    }

    pub fn compute_quality(&self, components: &[Item]) -> Quality {
        components
            .iter()
            .fold(Quality::Low, |a, b| a.max(b.quality()))
    }

    pub fn ability_spec(&self, _components: &[Item]) -> Option<Cow<AbilitySpec>> {
        match self {
            ModularBase::Tool(toolkind) => Some(Cow::Owned(AbilitySpec::Tool(*toolkind))),
        }
    }
}

// TODO: Look into changing to: Primary, Secondary
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModularComponentKind {
    Damage,
    Held,
}

impl ModularComponentKind {
    fn identifier_name(&self) -> &'static str {
        match self {
            ModularComponentKind::Damage => "damage",
            ModularComponentKind::Held => "held",
        }
    }

    /// Returns the main component of a weapon, i.e. which component has a
    /// material component
    fn main_component(tool: ToolKind) -> Self {
        match tool {
            ToolKind::Sword | ToolKind::Axe | ToolKind::Hammer | ToolKind::Bow => Self::Damage,
            ToolKind::Staff | ToolKind::Sceptre => Self::Held,
            _ => unimplemented!(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModularComponent {
    pub toolkind: ToolKind,
    pub modkind: ModularComponentKind,
    pub stats: tool::Stats,
    pub hand_restriction: Option<Hands>,
    pub weapon_name: String,
}

const SUPPORTED_TOOLKINDS: [ToolKind; 6] = [
    ToolKind::Sword,
    ToolKind::Axe,
    ToolKind::Hammer,
    ToolKind::Bow,
    ToolKind::Staff,
    ToolKind::Sceptre,
];

const WEAPON_PREFIX: &str = "common.items.weapons.modular";

fn make_weapon_id(toolkind: ToolKind) -> String {
    format!("{}.{}", WEAPON_PREFIX, toolkind.identifier_name())
}

/// Returns directory that contains components for a particular combination of
/// toolkind and modular component kind
fn make_mod_comp_dir_spec(tool: ToolKind, mod_kind: ModularComponentKind) -> String {
    const MOD_COMP_DIR_PREFIX: &str = "common.items.crafting_ing.modular";
    format!(
        "{}.{}.{}",
        MOD_COMP_DIR_PREFIX,
        mod_kind.identifier_name(),
        tool.identifier_name()
    )
}

/// Creates a random modular weapon when provided with a toolkind, material, and
/// optionally the handedness
pub fn random_weapon(tool: ToolKind, material: super::Material, hands: Option<Hands>) -> Item {
    // Returns inner modular component of an item if it has one
    fn unwrap_modular_component(item: &Item) -> Option<ModularComponent> {
        if let ItemKind::ModularComponent(mod_comp) = &*item.kind() {
            // TODO: Maybe get rid of clone?
            Some(mod_comp.clone())
        } else {
            None
        }
    }

    // Loads default ability map and material stat manifest for later use
    let ability_map = Default::default();
    let msm = Default::default();

    // Load recipe book (done to check that material is valid for a particular
    // component)
    let recipe::RawRecipeBook(recipes) =
        recipe::RawRecipeBook::load_expect_cloned("common.recipe_book");

    // Closure to check that an Item has a recipe that uses the provided material
    let is_composed_of = |item: &str| {
        // Iterate over all recipes in the raw recipe book
        recipes
            .values()
            // Filter by recipes that have an output of the item of interest
            .filter(|recipe| recipe.output.0.eq(item))
            // Check that item is composed of material, uses heuristic that assumes all modular components use the ListSameItem recipe input
            .any(|recipe| {
                recipe
                    .inputs
                    .iter()
                    .any(|input| {
                        match &input.0 {
                            recipe::RawRecipeInput::ListSameItem(items) => {
                                let assets = recipe::ItemList::load_expect_cloned(items).0;
                                assets.iter().any(|asset| Some(asset.as_str()) == material.asset_identifier())
                            },
                            _ => false,
                        }
                    })
            })
    };

    // Finds which component has a material as a subcomponent
    let material_comp = ModularComponentKind::main_component(tool);

    // Closure to return vec of components that are eligible to be used in the
    // modular weapon
    let create_component = |directory, hands| {
        // Load directory of components
        let components = Item::new_from_asset_glob(directory)
            .expect("Asset directory did not properly load")
            .into_iter()
            // Filter by handedness requirement
            .filter(|item| {
                matches!(unwrap_modular_component(item), Some(ModularComponent { hand_restriction, .. }) if hand_restriction.zip(hands).map_or(true, |(hr1, hr2)| hr1 == hr2))
            })
            // Filter by if component does not have a material, or if material can be used in the modular component
            .filter(|item| {
                matches!(unwrap_modular_component(item), Some(ModularComponent { modkind, .. }) if modkind != material_comp)
                || is_composed_of(item.item_definition_id())
            })
            .map(|item| (1.0, item))
            .collect::<Vec<_>>();

        // Create lottery and choose item
        Lottery::<Item>::from(components).choose_owned()
    };

    // Creates components of modular weapon
    let damage_comp_dir = make_mod_comp_dir_spec(tool, ModularComponentKind::Damage);
    let mut damage_component = create_component(&damage_comp_dir, hands);
    // Takes whichever is more restrictive of hand restriction passed in and hand
    // restriction from damage component e.g. if None is passed to function, and
    // damage component chooses piece with two handed restriction, then makes held
    // component have two handed restriction as well
    let damage_hands = unwrap_modular_component(&damage_component)
        .and_then(|mc| mc.hand_restriction)
        .or(hands);
    let held_comp_dir = make_mod_comp_dir_spec(tool, ModularComponentKind::Held);
    let mut held_component = create_component(&held_comp_dir, damage_hands);
    let material_component = Item::new_from_asset_expect(material.asset_identifier().expect(
        "Code reviewers: open comment here if I forget about this, I got lazy during a rebase",
    ));

    // Insert material item into modular component of appropriate kind
    match material_comp {
        ModularComponentKind::Damage => {
            damage_component.add_component(material_component, &ability_map, &msm);
        },
        ModularComponentKind::Held => {
            held_component.add_component(material_component, &ability_map, &msm);
        },
    }

    // Create modular weapon
    let components = vec![damage_component, held_component];
    Item::new_from_item_base(
        ItemBase::Modular(ModularBase::Tool(tool)),
        &components,
        &ability_map,
        &msm,
    )
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

    let (main_comp, material) = if let Some(main_comp) = mod_weap.components().iter().find(|comp| {
        matches!(&*comp.kind(), ItemKind::ModularComponent(mod_comp) if ModularComponentKind::main_component(mod_comp.toolkind) == mod_comp.modkind)
    }) {
        let material = if let Some(material) = main_comp.components().iter().filter_map(|mat| {
            if let Some(super::ItemTag::Material(material)) = mat.tags().iter().find(|tag| matches!(tag, super::ItemTag::Material(_))) {
                Some(material)
            } else {
                None
            }
        }).next() {
            material.into()
        } else {
            ""
        };

        (main_comp.item_definition_id(), material)
    } else {
        ("", "")
    };

    (main_comp.to_owned(), material.to_owned(), hands)
}

/// This is used as a key to uniquely identify the modular weapon in asset
/// manifests in voxygen (Main component, material)
pub type ModularWeaponComponentKey = (String, String);

pub enum ModularWeaponComponentKeyError {
    NotModularComponent,
    NotMainComponent,
}

pub fn weapon_component_to_key(
    mod_weap_comp: &dyn ItemDesc,
) -> Result<ModularWeaponComponentKey, ModularWeaponComponentKeyError> {
    if let ItemKind::ModularComponent(mod_comp) = &*mod_weap_comp.kind() {
        if ModularComponentKind::main_component(mod_comp.toolkind) == mod_comp.modkind {
            let material = if let Some(material) = mod_weap_comp
                .components()
                .iter()
                .filter_map(|mat| {
                    if let Some(super::ItemTag::Material(material)) = mat
                        .tags()
                        .iter()
                        .find(|tag| matches!(tag, super::ItemTag::Material(_)))
                    {
                        Some(material)
                    } else {
                        None
                    }
                })
                .next()
            {
                material.into()
            } else {
                ""
            };

            Ok((
                mod_weap_comp.item_definition_id().to_owned(),
                material.to_owned(),
            ))
        } else {
            Err(ModularWeaponComponentKeyError::NotMainComponent)
        }
    } else {
        Err(ModularWeaponComponentKeyError::NotModularComponent)
    }
}
