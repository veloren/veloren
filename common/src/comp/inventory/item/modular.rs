use super::{
    tool::{self, Hands},
    Item, ItemDesc, ItemKind, ItemName, RawItemDef, ToolKind,
};
use crate::{assets::AssetExt, lottery::Lottery, recipe};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

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

fn make_weapon_def(toolkind: ToolKind) -> (String, RawItemDef) {
    let identifier = format!("{}.{}", WEAPON_PREFIX, toolkind.identifier_name());
    let name = ItemName::Modular;
    let tool = tool::Tool {
        kind: toolkind,
        hands: tool::HandsKind::Modular,
        stats: tool::StatKind::Modular,
    };
    let kind = ItemKind::Tool(tool);
    let item = RawItemDef {
        name,
        description: "".to_string(),
        kind,
        quality: super::QualityKind::Modular,
        tags: Vec::new(),
        slots: 0,
        ability_spec: None,
    };
    (identifier, item)
}

fn initialize_modular_assets() -> HashMap<String, RawItemDef> {
    let mut itemdefs = HashMap::new();
    for &toolkind in &SUPPORTED_TOOLKINDS {
        let (identifier, item) = make_weapon_def(toolkind);
        itemdefs.insert(identifier.clone(), item);
    }
    itemdefs
}

lazy_static! {
    static ref ITEM_DEFS: HashMap<String, RawItemDef> = initialize_modular_assets();
}

/// Synthesize modular assets programmatically, to allow for the following:
/// - Allow the modular tag_examples to auto-update with the list of applicable
///   components
pub(super) fn synthesize_modular_asset(specifier: &str) -> Option<RawItemDef> {
    let ret = ITEM_DEFS.get(specifier).cloned();
    tracing::trace!("synthesize_modular_asset({:?}) -> {:?}", specifier, ret);
    ret
}

/// Modular weapons are named as "{Material} {Weapon}" where {Weapon} is from
/// the damage component used and {Material} is from the material the damage
/// component is created from.
pub(super) fn modular_name<'a>(item: &'a Item, arg1: &'a str) -> Cow<'a, str> {
    match item.kind() {
        ItemKind::Tool(tool) => {
            let main_components = item.components().iter().filter(|comp| {
                matches!(comp.kind(), ItemKind::ModularComponent(ModularComponent { modkind, .. })
                        if *modkind == ModularComponentKind::main_component(tool.kind)
                )
            });
            // Last fine as there should only ever be one damage component on a weapon
            let (material_name, weapon_name) = if let Some(component) = main_components.last() {
                let materials =
                    component
                        .components()
                        .iter()
                        .filter_map(|comp| match comp.kind() {
                            ItemKind::Ingredient { .. } => Some(comp.kind()),
                            _ => None,
                        });
                // TODO: Better handle multiple materials
                let material_name =
                    if let Some(ItemKind::Ingredient { descriptor, .. }) = materials.last() {
                        descriptor
                    } else {
                        "Modular"
                    };
                let weapon_name =
                    if let ItemKind::ModularComponent(ModularComponent { weapon_name, .. }) =
                        component.kind()
                    {
                        weapon_name
                    } else {
                        tool.kind.identifier_name()
                    };
                (material_name, weapon_name)
            } else {
                ("Modular", tool.kind.identifier_name())
            };

            Cow::Owned(format!("{} {}", material_name, weapon_name))
        },
        ItemKind::ModularComponent(comp) => {
            match comp.modkind {
                ModularComponentKind::Damage => {
                    let materials = item
                        .components()
                        .iter()
                        .filter_map(|comp| match comp.kind() {
                            ItemKind::Ingredient { .. } => Some(comp.kind()),
                            _ => None,
                        });
                    // TODO: Better handle multiple materials
                    let material_name =
                        if let Some(ItemKind::Ingredient { descriptor, .. }) = materials.last() {
                            descriptor
                        } else {
                            "Modular"
                        };
                    Cow::Owned(format!("{} {}", material_name, arg1))
                },
                ModularComponentKind::Held => Cow::Borrowed(arg1),
            }
        },
        _ => Cow::Borrowed("Modular Item"),
    }
}

pub(super) fn resolve_quality(item: &Item) -> super::Quality {
    item.components
        .iter()
        .fold(super::Quality::Common, |a, b| a.max(b.quality()))
}

/// Returns directory that contains components for a particular combination of
/// toolkind and modular component kind
fn make_mod_comp_dir_def(tool: ToolKind, mod_kind: ModularComponentKind) -> String {
    const MOD_COMP_DIR_PREFIX: &str = "common.items.crafting_ing.modular";
    format!(
        "{}.{}.{}",
        MOD_COMP_DIR_PREFIX,
        mod_kind.identifier_name(),
        tool.identifier_name()
    )
}

/// Creates initial item for a modular weapon
pub fn initialize_modular_weapon(toolkind: ToolKind) -> Item {
    Item::new_from_asset_expect(&make_weapon_def(toolkind).0)
}

/// Creates a random modular weapon when provided with a toolkind, material, and
/// optionally the handedness
pub fn random_weapon(tool: ToolKind, material: super::Material, hands: Option<Hands>) -> Item {
    // Returns inner modular component of an item if it has one
    fn unwrap_modular_component(item: &Item) -> Option<&ModularComponent> {
        if let ItemKind::ModularComponent(mod_comp) = item.kind() {
            Some(mod_comp)
        } else {
            None
        }
    }

    // Loads default ability map and material stat manifest for later use
    let ability_map = Default::default();
    let msm = Default::default();

    // Initialize modular weapon
    let mut modular_weapon = initialize_modular_weapon(tool);

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
            // Check that item is composed of material, uses heuristic that assumes all modular components use the TagSameItem recipe input
            .any(|recipe| {
                recipe
                    .inputs
                    .iter()
                    .any(|input| {
                        matches!(input.0, recipe::RawRecipeInput::TagSameItem(item_tag) if item_tag == super::ItemTag::MaterialKind(material.material_kind()))
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
                matches!(unwrap_modular_component(item), Some(ModularComponent { modkind, .. }) if *modkind != material_comp)
                || is_composed_of(item.item_definition_id())
            })
            .map(|item| (1.0, item))
            .collect::<Vec<_>>();

        // Create lottery and choose item
        Lottery::<Item>::from(components).choose_owned()
    };

    // Creates components of modular weapon
    let damage_comp_dir = make_mod_comp_dir_def(tool, ModularComponentKind::Damage);
    let mut damage_component = create_component(&damage_comp_dir, hands);
    // Takes whichever is more restrictive of hand restriction passed in and hand
    // restriction from damage component e.g. if None is passed to function, and
    // damage component chooses piece with two handed restriction, then makes held
    // component have two handed restriction as well
    let damage_hands = unwrap_modular_component(&damage_component)
        .and_then(|mc| mc.hand_restriction)
        .map_or(hands, Some);
    let held_comp_dir = make_mod_comp_dir_def(tool, ModularComponentKind::Held);
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

    // Insert components onto modular weapon
    modular_weapon.add_component(damage_component, &ability_map, &msm);
    modular_weapon.add_component(held_component, &ability_map, &msm);

    // Returns fully created modular weapon
    modular_weapon
}

// (Main component, material, hands)
pub type ModularWeaponKey = (String, String, Hands);

pub fn weapon_to_key(mod_weap: &dyn ItemDesc) -> ModularWeaponKey {
    let hands = if let ItemKind::Tool(tool) = mod_weap.kind() {
        tool.hands.resolve_hands(mod_weap.components())
    } else {
        Hands::One
    };

    let (main_comp, material) = if let Some(main_comp) = mod_weap.components().iter().find(|comp| {
        matches!(comp.kind(), ItemKind::ModularComponent(mod_comp) if ModularComponentKind::main_component(mod_comp.toolkind) == mod_comp.modkind)
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
