use crate::{
    assets::{self, AssetExt, AssetHandle},
    comp::{
        inventory::slot::InvSlotId,
        item::{
            modular,
            tool::{AbilityMap, ToolKind},
            ItemBase, ItemDef, ItemDefinitionIdOwned, ItemKind, ItemTag, MaterialStatManifest,
        },
        Inventory, Item,
    },
    terrain::SpriteKind,
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, sync::Arc};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RecipeInput {
    /// Only an item with a matching ItemDef can be used to satisfy this input
    Item(Arc<ItemDef>),
    /// Any items with this tag can be used to satisfy this input
    Tag(ItemTag),
    /// Similar to RecipeInput::Tag(_), but all items must be the same.
    /// Specifically this means that a mix of different items with the tag
    /// cannot be used.
    /// TODO: Currently requires that all items must be in the same slot.
    /// Eventually should be reworked so that items can be spread over multiple
    /// slots.
    TagSameItem(ItemTag),
    /// List is similar to tag, but has items defined in centralized file
    /// Similar to RecipeInput::TagSameItem(_), all items must be the same, they
    /// cannot be a mix of different items defined in the list.
    // Intent of using List over Tag is to make it harder for tag to be innocuously added to an
    // item breaking a recipe
    /// TODO: Currently requires that all items must be in the same slot.
    /// Eventually should be reworked so that items can be spread over multiple
    /// slots.
    ListSameItem(Vec<Arc<ItemDef>>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recipe {
    pub output: (Arc<ItemDef>, u32),
    /// Input required for recipe, amount of input needed, whether input should
    /// be tracked as a modular component
    pub inputs: Vec<(RecipeInput, u32, bool)>,
    pub craft_sprite: Option<SpriteKind>,
}

impl Recipe {
    /// Perform a recipe, returning a list of missing items on failure
    pub fn craft_simple(
        &self,
        inv: &mut Inventory,
        // Vec tying an input to a slot
        slots: Vec<(u32, InvSlotId)>,
        ability_map: &AbilityMap,
        msm: &MaterialStatManifest,
    ) -> Result<Vec<Item>, Vec<(&RecipeInput, u32)>> {
        let mut slot_claims = HashMap::new();
        let mut unsatisfied_requirements = Vec::new();
        let mut component_slots = Vec::new();

        // Checks each input against slots in the inventory. If the slots contain an
        // item that fulfills the need of the input, marks some of the item as claimed
        // up to quantity needed for the crafting input. If the item either
        // cannot be used, or there is insufficient quantity, adds input and
        // number of materials needed to unsatisfied requirements.
        self.inputs
            .iter()
            .enumerate()
            .for_each(|(i, (input, amount, mut is_component))| {
                let mut required = *amount;
                // Check used for recipes that have an input that is not consumed, e.g.
                // craftsman hammer
                let mut contains_any = false;
                // Gets all slots provided for this input by the frontend
                let input_slots = slots
                    .iter()
                    .filter_map(|(j, slot)| if i as u32 == *j { Some(slot) } else { None });
                // Goes through each slot and marks some amount from each slot as claimed
                for slot in input_slots {
                    // Checks that the item in the slot can be used for the input
                    if let Some(item) = inv
                        .get(*slot)
                        .filter(|item| item.matches_recipe_input(input, *amount))
                    {
                        // Gets the number of items claimed from the slot, or sets to 0 if slot has
                        // not been claimed by another input yet
                        let claimed = slot_claims.entry(*slot).or_insert(0);
                        let available = item.amount().saturating_sub(*claimed);
                        let provided = available.min(required);
                        required -= provided;
                        *claimed += provided;
                        // If input is a component and provided amount from this slot at least 1,
                        // mark 1 piece as coming from that slot and set is_component to false to
                        // indicate it has been claimed.
                        if provided > 0 && is_component {
                            component_slots.push(*slot);
                            is_component = false;
                        }
                        contains_any = true;
                    }
                }
                // If there were not sufficient items to cover requirement between all provided
                // slots, or if non-consumed item was not present, mark input as not satisfied
                if required > 0 || !contains_any {
                    unsatisfied_requirements.push((input, required));
                }
            });

        // If there are no unsatisfied requirements, create the items produced by the
        // recipe in the necessary quantity and remove the items that the recipe
        // consumes
        if unsatisfied_requirements.is_empty() {
            let mut components = Vec::new();
            for slot in component_slots.iter() {
                let component = inv
                    .take(*slot, ability_map, msm)
                    .expect("Expected item to exist in the inventory");
                components.push(component);
                let to_remove = slot_claims
                    .get_mut(slot)
                    .expect("If marked in component slots, should be in slot claims");
                *to_remove -= 1;
            }
            for (slot, to_remove) in slot_claims.iter() {
                for _ in 0..*to_remove {
                    let _ = inv
                        .take(*slot, ability_map, msm)
                        .expect("Expected item to exist in the inventory");
                }
            }
            let (item_def, quantity) = &self.output;

            let crafted_item = Item::new_from_item_base(
                ItemBase::Simple(Arc::clone(item_def)),
                components,
                ability_map,
                msm,
            );
            let mut crafted_items = Vec::with_capacity(*quantity as usize);
            for _ in 0..*quantity {
                crafted_items.push(crafted_item.duplicate(ability_map, msm));
            }
            Ok(crafted_items)
        } else {
            Err(unsatisfied_requirements)
        }
    }

    pub fn inputs(&self) -> impl ExactSizeIterator<Item = (&RecipeInput, u32, bool)> {
        self.inputs
            .iter()
            .map(|(item_def, amount, is_mod_comp)| (item_def, *amount, *is_mod_comp))
    }

    /// Determine whether the inventory contains the ingredients for a recipe.
    /// If it does, return a vec of inventory slots that contain the
    /// ingredients needed, whose positions correspond to particular recipe
    /// inputs. If items are missing, return the missing items, and how many
    /// are missing.
    pub fn inventory_contains_ingredients(
        &self,
        inv: &Inventory,
        recipe_amount: u32,
    ) -> Result<Vec<(u32, InvSlotId)>, Vec<(&RecipeInput, u32)>> {
        inventory_contains_ingredients(
            self.inputs()
                .map(|(input, amount, _is_modular)| (input, amount)),
            inv,
            recipe_amount,
        )
    }

    /// Calculates the maximum number of items craftable given the current
    /// inventory state.
    pub fn max_from_ingredients(&self, inv: &Inventory) -> u32 {
        let mut max_recipes = None;

        for (input, amount) in self
            .inputs()
            .map(|(input, amount, _is_modular)| (input, amount))
        {
            let needed = amount as f32;
            let mut input_max = HashMap::<InvSlotId, u32>::new();

            // Checks through every slot, filtering to only those that contain items that
            // can satisfy the input.
            for (inv_slot_id, slot) in inv.slots_with_id() {
                if let Some(item) = slot
                    .as_ref()
                    .filter(|item| item.matches_recipe_input(input, amount))
                {
                    *input_max.entry(inv_slot_id).or_insert(0) += item.amount();
                }
            }

            // Updates maximum craftable amount based on least recipe-proportional
            // availability.
            let max_item_proportion =
                ((input_max.values().sum::<u32>() as f32) / needed).floor() as u32;
            max_recipes = Some(match max_recipes {
                None => max_item_proportion,
                Some(max_recipes) if (max_item_proportion < max_recipes) => max_item_proportion,
                Some(n) => n,
            });
        }

        max_recipes.unwrap_or(0)
    }
}

/// Determine whether the inventory contains the ingredients for a recipe.
/// If it does, return a vec of inventory slots that contain the
/// ingredients needed, whose positions correspond to particular recipe
/// inputs. If items are missing, return the missing items, and how many
/// are missing.
// Note: Doc comment duplicated on two public functions that call this function
#[allow(clippy::type_complexity)]
fn inventory_contains_ingredients<'a, I: Iterator<Item = (&'a RecipeInput, u32)>>(
    ingredients: I,
    inv: &Inventory,
    recipe_amount: u32,
) -> Result<Vec<(u32, InvSlotId)>, Vec<(&'a RecipeInput, u32)>> {
    // Hashmap tracking the quantity that needs to be removed from each slot (so
    // that it doesn't think a slot can provide more items than it contains)
    let mut slot_claims = HashMap::<InvSlotId, u32>::new();
    // Important to be a vec and to remain separate from slot_claims as it must
    // remain ordered, unlike the hashmap
    let mut slots = Vec::<(u32, InvSlotId)>::new();
    // The inputs to a recipe that have missing items, and the amount missing
    let mut missing = Vec::<(&RecipeInput, u32)>::new();

    for (i, (input, amount)) in ingredients.enumerate() {
        let mut needed = amount * recipe_amount;
        let mut contains_any = false;
        // Checks through every slot, filtering to only those that contain items that
        // can satisfy the input
        for (inv_slot_id, slot) in inv.slots_with_id() {
            if let Some(item) = slot
                .as_ref()
                .filter(|item| item.matches_recipe_input(input, amount))
            {
                let claim = slot_claims.entry(inv_slot_id).or_insert(0);
                slots.push((i as u32, inv_slot_id));
                let can_claim = (item.amount().saturating_sub(*claim)).min(needed);
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
        let salvage_item = inv.get(slot).expect("Expected item to exist in inventory");
        let salvage_output: Vec<_> = salvage_item
            .salvage_output()
            .map(Item::new_from_asset_expect)
            .collect();
        if salvage_output.is_empty() {
            // If no output items, assume salvaging was a failure
            // TODO: If we ever change salvaging to have a percent chance, remove the check
            // of outputs being empty (requires assets to exist for rock and wood materials
            // so that salvaging doesn't silently fail)
            Err(SalvageError::NotSalvageable)
        } else {
            // Remove item that is being salvaged
            let _ = inv
                .take(slot, ability_map, msm)
                .expect("Expected item to exist in inventory");
            // Return the salvaging output
            Ok(salvage_output)
        }
    } else {
        Err(SalvageError::NotSalvageable)
    }
}

pub enum ModularWeaponError {
    InvalidSlot,
    ComponentMismatch,
    DifferentTools,
    DifferentHands,
}

pub fn modular_weapon(
    inv: &mut Inventory,
    primary_component: InvSlotId,
    secondary_component: InvSlotId,
    ability_map: &AbilityMap,
    msm: &MaterialStatManifest,
) -> Result<Item, ModularWeaponError> {
    use modular::ModularComponent;
    // Closure to get inner modular component info from item in a given slot
    fn unwrap_modular(inv: &Inventory, slot: InvSlotId) -> Option<Cow<ModularComponent>> {
        inv.get(slot).and_then(|item| match item.kind() {
            Cow::Owned(ItemKind::ModularComponent(mod_comp)) => Some(Cow::Owned(mod_comp)),
            Cow::Borrowed(ItemKind::ModularComponent(mod_comp)) => Some(Cow::Borrowed(mod_comp)),
            _ => None,
        })
    }

    // Checks if both components are compatible, and if so returns the toolkind to
    // make weapon of
    let compatiblity = if let (Some(primary_component), Some(secondary_component)) = (
        unwrap_modular(inv, primary_component),
        unwrap_modular(inv, secondary_component),
    ) {
        // Checks that damage and held component slots each contain a damage and held
        // modular component respectively
        if let (
            ModularComponent::ToolPrimaryComponent {
                toolkind: tool_a,
                hand_restriction: hands_a,
                ..
            },
            ModularComponent::ToolSecondaryComponent {
                toolkind: tool_b,
                hand_restriction: hands_b,
                ..
            },
        ) = (&*primary_component, &*secondary_component)
        {
            // Checks that both components are of the same tool kind
            if tool_a == tool_b {
                // Checks that if both components have a hand restriction, they are the same
                let hands_check = hands_a.zip(*hands_b).map_or(true, |(a, b)| a == b);
                if hands_check {
                    Ok(())
                } else {
                    Err(ModularWeaponError::DifferentHands)
                }
            } else {
                Err(ModularWeaponError::DifferentTools)
            }
        } else {
            Err(ModularWeaponError::ComponentMismatch)
        }
    } else {
        Err(ModularWeaponError::InvalidSlot)
    };

    match compatiblity {
        Ok(()) => {
            // Remove components from inventory
            let primary_component = inv
                .take(primary_component, ability_map, msm)
                .expect("Expected component to exist");
            let secondary_component = inv
                .take(secondary_component, ability_map, msm)
                .expect("Expected component to exist");

            // Create modular weapon
            Ok(Item::new_from_item_base(
                ItemBase::Modular(modular::ModularBase::Tool),
                vec![primary_component, secondary_component],
                ability_map,
                msm,
            ))
        },
        Err(err) => Err(err),
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
            .filter(|(_, recipe)| recipe.inventory_contains_ingredients(inv, 1).is_ok())
            .map(|(name, recipe)| (name.clone(), recipe.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_recipe_valid_key_check() {
        let recipe_book = default_recipe_book().read();
        let is_invalid_key =
            |input: &str| input.chars().any(|c| c.is_uppercase() || c.is_whitespace());
        assert!(!recipe_book.iter().any(|(k, _)| is_invalid_key(k)));
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RawRecipeInput {
    Item(String),
    Tag(ItemTag),
    TagSameItem(ItemTag),
    ListSameItem(String),
}

impl RawRecipeInput {
    fn load_recipe_input(&self) -> Result<RecipeInput, assets::Error> {
        let input = match self {
            RawRecipeInput::Item(name) => RecipeInput::Item(Arc::<ItemDef>::load_cloned(name)?),
            RawRecipeInput::Tag(tag) => RecipeInput::Tag(*tag),
            RawRecipeInput::TagSameItem(tag) => RecipeInput::TagSameItem(*tag),
            RawRecipeInput::ListSameItem(list) => {
                let assets = &ItemList::load_expect(list).read().0;
                let items = assets
                    .iter()
                    .map(|asset| Arc::<ItemDef>::load_expect_cloned(asset))
                    .collect();
                RecipeInput::ListSameItem(items)
            },
        };
        Ok(input)
    }
}

#[derive(Clone, Deserialize)]
pub(crate) struct RawRecipe {
    pub(crate) output: (String, u32),
    /// Input required for recipe, amount of input needed, whether input should
    /// be tracked as a modular component
    pub(crate) inputs: Vec<(RawRecipeInput, u32, bool)>,
    pub(crate) craft_sprite: Option<SpriteKind>,
}

#[derive(Clone, Deserialize)]
#[serde(transparent)]
pub(crate) struct RawRecipeBook(pub(crate) HashMap<String, RawRecipe>);

impl assets::Asset for RawRecipeBook {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

#[derive(Deserialize, Clone)]
struct ItemList(Vec<String>);

impl assets::Asset for ItemList {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

impl assets::Compound for RecipeBook {
    fn load(
        cache: assets::AnyCache,
        specifier: &assets::SharedString,
    ) -> Result<Self, assets::BoxedError> {
        #[inline]
        fn load_item_def(spec: &(String, u32)) -> Result<(Arc<ItemDef>, u32), assets::Error> {
            let def = Arc::<ItemDef>::load_cloned(&spec.0)?;
            Ok((def, spec.1))
        }

        #[inline]
        fn load_recipe_input(
            (input, amount, is_mod_comp): &(RawRecipeInput, u32, bool),
        ) -> Result<(RecipeInput, u32, bool), assets::Error> {
            let def = input.load_recipe_input()?;
            Ok((def, *amount, *is_mod_comp))
        }

        let raw = cache.load::<RawRecipeBook>(specifier)?.cloned();

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComponentRecipeBook {
    recipes: HashMap<ComponentKey, ComponentRecipe>,
}

#[derive(Clone, Debug)]
pub struct ReverseComponentRecipeBook {
    recipes: HashMap<ItemDefinitionIdOwned, ComponentRecipe>,
}

impl ComponentRecipeBook {
    pub fn get(&self, key: &ComponentKey) -> Option<&ComponentRecipe> { self.recipes.get(key) }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&ComponentKey, &ComponentRecipe)> {
        self.recipes.iter()
    }
}

impl ReverseComponentRecipeBook {
    pub fn get(&self, key: &ItemDefinitionIdOwned) -> Option<&ComponentRecipe> {
        self.recipes.get(key)
    }
}

#[derive(Clone, Deserialize)]
#[serde(transparent)]
struct RawComponentRecipeBook(Vec<RawComponentRecipe>);

impl assets::Asset for RawComponentRecipeBook {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct ComponentKey {
    // Can't use ItemDef here because hash needed, item definition id used instead
    // TODO: Make more general for other things that have component inputs that should be tracked
    // after item creation
    pub toolkind: ToolKind,
    /// Refers to the item definition id of the material
    pub material: String,
    /// Refers to the item definition id of the modifier
    pub modifier: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComponentRecipe {
    output: ComponentOutput,
    material: (RecipeInput, u32),
    modifier: Option<(RecipeInput, u32)>,
    additional_inputs: Vec<(RecipeInput, u32)>,
    pub craft_sprite: Option<SpriteKind>,
}

impl ComponentRecipe {
    /// Craft an item that has components, returning a list of missing items on
    /// failure
    pub fn craft_component(
        &self,
        inv: &mut Inventory,
        material_slot: InvSlotId,
        modifier_slot: Option<InvSlotId>,
        // Vec tying an input to a slot
        slots: Vec<(u32, InvSlotId)>,
        ability_map: &AbilityMap,
        msm: &MaterialStatManifest,
    ) -> Result<Vec<Item>, Vec<(&RecipeInput, u32)>> {
        let mut slot_claims = HashMap::new();
        let mut unsatisfied_requirements = Vec::new();

        fn handle_requirement<'a, I: Iterator<Item = InvSlotId>>(
            slot_claims: &mut HashMap<InvSlotId, u32>,
            unsatisfied_requirements: &mut Vec<(&'a RecipeInput, u32)>,
            inv: &Inventory,
            input: &'a RecipeInput,
            amount: u32,
            input_slots: I,
        ) {
            let mut required = amount;
            // contains_any check used for recipes that have an input that is not consumed,
            // e.g. craftsman hammer
            // Goes through each slot and marks some amount from each slot as claimed
            let contains_any = input_slots.into_iter().all(|slot| {
                // Checks that the item in the slot can be used for the input
                if let Some(item) = inv
                    .get(slot)
                    .filter(|item| item.matches_recipe_input(input, amount))
                {
                    // Gets the number of items claimed from the slot, or sets to 0 if slot has
                    // not been claimed by another input yet
                    let claimed = slot_claims.entry(slot).or_insert(0);
                    let available = item.amount().saturating_sub(*claimed);
                    let provided = available.min(required);
                    required -= provided;
                    *claimed += provided;
                    true
                } else {
                    false
                }
            });
            // If there were not sufficient items to cover requirement between all provided
            // slots, or if non-consumed item was not present, mark input as not satisfied
            if required > 0 || !contains_any {
                unsatisfied_requirements.push((input, required));
            }
        }

        // Checks each input against slots in the inventory. If the slots contain an
        // item that fulfills the need of the input, marks some of the item as claimed
        // up to quantity needed for the crafting input. If the item either
        // cannot be used, or there is insufficient quantity, adds input and
        // number of materials needed to unsatisfied requirements.
        handle_requirement(
            &mut slot_claims,
            &mut unsatisfied_requirements,
            inv,
            &self.material.0,
            self.material.1,
            core::iter::once(material_slot),
        );
        if let Some((modifier_input, modifier_amount)) = &self.modifier {
            // TODO: Better way to get slot to use that ensures this requirement fails if no
            // slot provided?
            handle_requirement(
                &mut slot_claims,
                &mut unsatisfied_requirements,
                inv,
                modifier_input,
                *modifier_amount,
                core::iter::once(modifier_slot.unwrap_or(InvSlotId::new(0, 0))),
            );
        }
        self.additional_inputs
            .iter()
            .enumerate()
            .for_each(|(i, (input, amount))| {
                // Gets all slots provided for this input by the frontend
                let input_slots = slots
                    .iter()
                    .filter_map(|(j, slot)| if i as u32 == *j { Some(slot) } else { None })
                    .copied();
                // Checks if requirement is met, and if not marks it as unsatisfied
                handle_requirement(
                    &mut slot_claims,
                    &mut unsatisfied_requirements,
                    inv,
                    input,
                    *amount,
                    input_slots,
                );
            });

        // If there are no unsatisfied requirements, create the items produced by the
        // recipe in the necessary quantity and remove the items that the recipe
        // consumes
        if unsatisfied_requirements.is_empty() {
            for (slot, to_remove) in slot_claims.iter() {
                for _ in 0..*to_remove {
                    let _ = inv
                        .take(*slot, ability_map, msm)
                        .expect("Expected item to exist in the inventory");
                }
            }

            let crafted_item = self.item_output(ability_map, msm);

            Ok(vec![crafted_item])
        } else {
            Err(unsatisfied_requirements)
        }
    }

    #[allow(clippy::type_complexity)]
    /// Determine whether the inventory contains the additional ingredients for
    /// a component recipe. If it does, return a vec of inventory slots that
    /// contain the ingredients needed, whose positions correspond to particular
    /// recipe are missing.
    pub fn inventory_contains_additional_ingredients(
        &self,
        inv: &Inventory,
    ) -> Result<Vec<(u32, InvSlotId)>, Vec<(&RecipeInput, u32)>> {
        inventory_contains_ingredients(
            self.additional_inputs
                .iter()
                .map(|(input, amount)| (input, *amount)),
            inv,
            1,
        )
    }

    pub fn itemdef_output(&self) -> ItemDefinitionIdOwned {
        match &self.output {
            ComponentOutput::ItemComponents {
                item: item_def,
                components,
            } => {
                let components = components
                    .iter()
                    .map(|item_def| ItemDefinitionIdOwned::Simple(item_def.id().to_owned()))
                    .collect::<Vec<_>>();
                ItemDefinitionIdOwned::Compound {
                    simple_base: item_def.id().to_owned(),
                    components,
                }
            },
        }
    }

    pub fn item_output(&self, ability_map: &AbilityMap, msm: &MaterialStatManifest) -> Item {
        match &self.output {
            ComponentOutput::ItemComponents {
                item: item_def,
                components,
            } => {
                let components = components
                    .iter()
                    .map(|item_def| {
                        Item::new_from_item_base(
                            ItemBase::Simple(Arc::clone(item_def)),
                            Vec::new(),
                            ability_map,
                            msm,
                        )
                    })
                    .collect::<Vec<_>>();
                Item::new_from_item_base(
                    ItemBase::Simple(Arc::clone(item_def)),
                    components,
                    ability_map,
                    msm,
                )
            },
        }
    }

    pub fn inputs(&self) -> impl ExactSizeIterator<Item = (&RecipeInput, u32)> {
        pub struct ComponentRecipeInputsIterator<'a> {
            material: Option<&'a (RecipeInput, u32)>,
            modifier: Option<&'a (RecipeInput, u32)>,
            additional_inputs: std::slice::Iter<'a, (RecipeInput, u32)>,
        }

        impl<'a> Iterator for ComponentRecipeInputsIterator<'a> {
            type Item = &'a (RecipeInput, u32);

            fn next(&mut self) -> Option<&'a (RecipeInput, u32)> {
                self.material
                    .take()
                    .or_else(|| self.modifier.take())
                    .or_else(|| self.additional_inputs.next())
            }
        }

        impl<'a> IntoIterator for &'a ComponentRecipe {
            type IntoIter = ComponentRecipeInputsIterator<'a>;
            type Item = &'a (RecipeInput, u32);

            fn into_iter(self) -> Self::IntoIter {
                ComponentRecipeInputsIterator {
                    material: Some(&self.material),
                    modifier: self.modifier.as_ref(),
                    additional_inputs: self.additional_inputs.as_slice().iter(),
                }
            }
        }

        impl<'a> ExactSizeIterator for ComponentRecipeInputsIterator<'a> {
            fn len(&self) -> usize {
                self.material.is_some() as usize
                    + self.modifier.is_some() as usize
                    + self.additional_inputs.len()
            }
        }

        self.into_iter().map(|(recipe, amount)| (recipe, *amount))
    }
}

#[derive(Clone, Deserialize)]
struct RawComponentRecipe {
    output: RawComponentOutput,
    /// String refers to an item definition id
    material: (String, u32),
    /// String refers to an item definition id
    modifier: Option<(String, u32)>,
    additional_inputs: Vec<(RawRecipeInput, u32)>,
    craft_sprite: Option<SpriteKind>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum ComponentOutput {
    // TODO: Don't store list of components here in case we ever want components in future to have
    // state to them (e.g. a component having sub-components)
    ItemComponents {
        item: Arc<ItemDef>,
        components: Vec<Arc<ItemDef>>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum RawComponentOutput {
    /// Creates the primary component of a modular tool. Assumes that the
    /// material used is the only component in the item.
    ToolPrimaryComponent { toolkind: ToolKind, item: String },
}

impl assets::Compound for ComponentRecipeBook {
    fn load(
        cache: assets::AnyCache,
        specifier: &assets::SharedString,
    ) -> Result<Self, assets::BoxedError> {
        #[inline]
        fn create_recipe_key(raw_recipe: &RawComponentRecipe) -> ComponentKey {
            match &raw_recipe.output {
                RawComponentOutput::ToolPrimaryComponent { toolkind, item: _ } => {
                    let material = String::from(&raw_recipe.material.0);
                    let modifier = raw_recipe
                        .modifier
                        .as_ref()
                        .map(|(modifier, _amount)| String::from(modifier));
                    ComponentKey {
                        toolkind: *toolkind,
                        material,
                        modifier,
                    }
                },
            }
        }

        #[inline]
        fn load_recipe(raw_recipe: &RawComponentRecipe) -> Result<ComponentRecipe, assets::Error> {
            let output = match &raw_recipe.output {
                RawComponentOutput::ToolPrimaryComponent { toolkind: _, item } => {
                    let item = Arc::<ItemDef>::load_cloned(item)?;
                    let components = vec![Arc::<ItemDef>::load_cloned(&raw_recipe.material.0)?];
                    ComponentOutput::ItemComponents { item, components }
                },
            };
            let material = (
                RecipeInput::Item(Arc::<ItemDef>::load_cloned(&raw_recipe.material.0)?),
                raw_recipe.material.1,
            );
            let modifier = if let Some((modifier, amount)) = &raw_recipe.modifier {
                let modifier = Arc::<ItemDef>::load_cloned(modifier)?;
                Some((RecipeInput::Item(modifier), *amount))
            } else {
                None
            };
            let additional_inputs = raw_recipe
                .additional_inputs
                .iter()
                .map(|(input, amount)| input.load_recipe_input().map(|input| (input, *amount)))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(ComponentRecipe {
                output,
                material,
                modifier,
                additional_inputs,
                craft_sprite: raw_recipe.craft_sprite,
            })
        }

        let raw = cache.load::<RawComponentRecipeBook>(specifier)?.cloned();

        let recipes = raw
            .0
            .iter()
            .map(|raw_recipe| {
                load_recipe(raw_recipe).map(|recipe| (create_recipe_key(raw_recipe), recipe))
            })
            .collect::<Result<_, assets::Error>>()?;

        Ok(ComponentRecipeBook { recipes })
    }
}

pub fn default_recipe_book() -> AssetHandle<RecipeBook> {
    RecipeBook::load_expect("common.recipe_book")
}

pub fn default_component_recipe_book() -> AssetHandle<ComponentRecipeBook> {
    ComponentRecipeBook::load_expect("common.component_recipe_book")
}

impl assets::Compound for ReverseComponentRecipeBook {
    fn load(
        cache: assets::AnyCache,
        specifier: &assets::SharedString,
    ) -> Result<Self, assets::BoxedError> {
        let forward = cache.load::<ComponentRecipeBook>(specifier)?.cloned();
        let mut recipes = HashMap::new();
        for (_, recipe) in forward.iter() {
            recipes.insert(recipe.itemdef_output(), recipe.clone());
        }
        Ok(ReverseComponentRecipeBook { recipes })
    }
}
