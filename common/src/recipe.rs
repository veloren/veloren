use crate::{
    assets::{self, AssetExt, AssetHandle},
    comp::{
        inventory::slot::InvSlotId,
        item::{modular, tool::AbilityMap, ItemDef, ItemTag, MaterialStatManifest},
        Inventory, Item,
    },
    terrain::SpriteKind,
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RecipeInput {
    Item(Arc<ItemDef>),
    Tag(ItemTag),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recipe {
    pub output: (Arc<ItemDef>, u32),
    pub inputs: Vec<(RecipeInput, u32)>,
    pub craft_sprite: Option<SpriteKind>,
}

#[allow(clippy::type_complexity)]
impl Recipe {
    /// Perform a recipe, returning a list of missing items on failure
    pub fn craft_simple(
        &self,
        inv: &mut Inventory,
        slots: Vec<InvSlotId>,
        ability_map: &AbilityMap,
        msm: &MaterialStatManifest,
    ) -> Result<Vec<Item>, Vec<(&RecipeInput, u32)>> {
        let mut recipe_inputs = Vec::new();
        let mut unsatisfied_requirements = Vec::new();

        // Checks each input against a slot in the inventory. If the slot contains an
        // item that fulfills the need of the input, takes from the inventory up to the
        // quantity needed for the crafting input. If the item either cannot be used, or
        // there is insufficient quantity, adds input and number of materials needed to
        // unsatisfied requirements.
        self.inputs
            .iter()
            .enumerate()
            .for_each(|(i, (input, amount))| {
                let valid_input = if let Some(item) = slots.get(i).and_then(|slot| inv.get(*slot)) {
                    item.matches_recipe_input(input)
                } else {
                    false
                };

                if let Some(slot) = slots.get(i) {
                    if !valid_input {
                        unsatisfied_requirements.push((input, *amount));
                    } else {
                        for taken in 0..*amount {
                            if let Some(item) = inv.take(*slot, ability_map, msm) {
                                recipe_inputs.push(item);
                            } else {
                                unsatisfied_requirements.push((input, *amount - taken));
                                break;
                            }
                        }
                    }
                } else {
                    unsatisfied_requirements.push((input, *amount));
                }
            });

        // If there are no unsatisfied requirements, create the items produced by the
        // recipe in the necessary quantity, else insert the ingredients back into the
        // inventory
        if unsatisfied_requirements.is_empty() {
            let (item_def, quantity) = &self.output;
            let crafted_item = Item::new_from_item_def(Arc::clone(item_def), &[], ability_map, msm);
            let mut crafted_items = Vec::with_capacity(*quantity as usize);
            for _ in 0..*quantity {
                crafted_items.push(crafted_item.duplicate(ability_map, msm));
            }
            Ok(crafted_items)
        } else {
            for item in recipe_inputs {
                inv.push(item)
                    .expect("Item was in inventory before craft attempt");
            }
            Err(unsatisfied_requirements)
        }
    }

    pub fn inputs(&self) -> impl ExactSizeIterator<Item = (&RecipeInput, u32)> {
        self.inputs
            .iter()
            .map(|(item_def, amount)| (item_def, *amount))
    }

    /// Determine whether the inventory contains the ingredients for a recipe.
    /// If it does, return a vec of  inventory slots that contain the
    /// ingredients needed, whose positions correspond to particular recipe
    /// inputs. If items are missing, return the missing items, and how many
    /// are missing.
    pub fn inventory_contains_ingredients<'a>(
        &self,
        inv: &'a Inventory,
    ) -> Result<Vec<InvSlotId>, Vec<(&RecipeInput, u32)>> {
        let mut slot_claims = HashMap::<InvSlotId, u32>::new();
        // Important to be a vec and to remain separate from slot_claims as it must
        // remain ordered, unlike the hashmap
        let mut slots = Vec::<InvSlotId>::new();
        let mut missing = Vec::<(&RecipeInput, u32)>::new();

        for (input, mut needed) in self.inputs() {
            let mut contains_any = false;

            for (inv_slot_id, slot) in inv.slots_with_id() {
                if let Some(item) = slot
                    .as_ref()
                    .filter(|item| item.matches_recipe_input(&*input))
                {
                    let claim = slot_claims.entry(inv_slot_id).or_insert(0);
                    slots.push(inv_slot_id);
                    // FIXME: Fishy, looks like it can underflow before min which can trigger an
                    // overflow check.
                    let can_claim = (item.amount() - *claim).min(needed);
                    *claim += can_claim;
                    needed -= can_claim;
                    contains_any = true;
                }
            }

            if needed > 0 || !contains_any {
                missing.push((input, needed));
            }
        }

        if missing.is_empty() {
            Ok(slots)
        } else {
            Err(missing)
        }
    }
}

pub enum SalvageError {
    NotSalvageable,
}

pub fn try_salvage(
    inv: &mut Inventory,
    slot: InvSlotId,
    ability_map: &AbilityMap,
    msm: &MaterialStatManifest,
) -> Result<Vec<Item>, SalvageError> {
    if inv.get(slot).map_or(false, |item| item.is_salvageable()) {
        let salvage_item = inv
            .take(slot, ability_map, msm)
            .expect("Expected item to exist in inventory");
        match salvage_item.try_salvage() {
            Ok(items) => Ok(items),
            Err(item) => {
                inv.push(item)
                    .expect("Item taken from inventory just before");
                Err(SalvageError::NotSalvageable)
            },
        }
    } else {
        Err(SalvageError::NotSalvageable)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecipeBook {
    recipes: HashMap<String, Recipe>,
}

impl RecipeBook {
    pub fn get(&self, recipe: &str) -> Option<&Recipe> { self.recipes.get(recipe) }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&String, &Recipe)> { self.recipes.iter() }

    pub fn get_available(&self, inv: &Inventory) -> Vec<(String, Recipe)> {
        self.recipes
            .iter()
            .filter(|(_, recipe)| recipe.inventory_contains_ingredients(inv).is_ok())
            .map(|(name, recipe)| (name.clone(), recipe.clone()))
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RawRecipeInput {
    Item(String),
    Tag(ItemTag),
}

#[derive(Clone, Deserialize)]
pub(crate) struct RawRecipe {
    pub(crate) output: (String, u32),
    pub(crate) inputs: Vec<(RawRecipeInput, u32)>,
    pub(crate) craft_sprite: Option<SpriteKind>,
}

#[derive(Clone, Deserialize)]
#[serde(transparent)]
pub(crate) struct RawRecipeBook(pub(crate) HashMap<String, RawRecipe>);

impl assets::Asset for RawRecipeBook {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

impl assets::Compound for RecipeBook {
    fn load<S: assets::source::Source>(
        cache: &assets::AssetCache<S>,
        specifier: &str,
    ) -> Result<Self, assets::Error> {
        #[inline]
        fn load_item_def(spec: &(String, u32)) -> Result<(Arc<ItemDef>, u32), assets::Error> {
            let def = Arc::<ItemDef>::load_cloned(&spec.0)?;
            Ok((def, spec.1))
        }

        #[inline]
        fn load_recipe_input(
            spec: &(RawRecipeInput, u32),
        ) -> Result<(RecipeInput, u32), assets::Error> {
            let def = match &spec.0 {
                RawRecipeInput::Item(name) => RecipeInput::Item(Arc::<ItemDef>::load_cloned(name)?),
                RawRecipeInput::Tag(tag) => RecipeInput::Tag(*tag),
            };
            Ok((def, spec.1))
        }

        let mut raw = cache.load::<RawRecipeBook>(specifier)?.read().clone();

        // Avoid showing purple-question-box recipes until the assets are added
        // (the `if false` is needed because commenting out the call will add a warning
        // that there are no other uses of append_modular_recipes)
        if false {
            modular::append_modular_recipes(&mut raw);
        }

        let recipes = raw
            .0
            .iter()
            .map(
                |(
                    name,
                    RawRecipe {
                        output,
                        inputs,
                        craft_sprite,
                    },
                )| {
                    let inputs = inputs
                        .iter()
                        .map(load_recipe_input)
                        .collect::<Result<Vec<_>, _>>()?;
                    let output = load_item_def(output)?;
                    Ok((name.clone(), Recipe {
                        output,
                        inputs,
                        craft_sprite: *craft_sprite,
                    }))
                },
            )
            .collect::<Result<_, assets::Error>>()?;

        Ok(RecipeBook { recipes })
    }
}

pub fn default_recipe_book() -> AssetHandle<RecipeBook> {
    RecipeBook::load_expect("common.recipe_book")
}
