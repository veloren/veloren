use crate::{
    assets::{self, AssetExt, AssetHandle},
    comp::{
        item::{modular, ItemDef, ItemTag, MaterialStatManifest},
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
    pub fn perform(
        &self,
        inv: &mut Inventory,
        msm: &MaterialStatManifest,
    ) -> Result<Option<(Item, u32)>, Vec<(&RecipeInput, u32)>> {
        // Get ingredient cells from inventory,
        let mut components = Vec::new();

        inv.contains_ingredients(self)?
            .into_iter()
            .for_each(|(pos, n)| {
                (0..n).for_each(|_| {
                    let component = inv
                        .take(pos, msm)
                        .expect("Expected item to exist in inventory");
                    components.push(component);
                })
            });

        for i in 0..self.output.1 {
            let crafted_item =
                Item::new_from_item_def(Arc::clone(&self.output.0), &components, msm);
            if let Err(item) = inv.push(crafted_item) {
                return Ok(Some((item, self.output.1 - i)));
            }
        }

        Ok(None)
    }

    pub fn inputs(&self) -> impl ExactSizeIterator<Item = (&RecipeInput, u32)> {
        self.inputs
            .iter()
            .map(|(item_def, amount)| (item_def, *amount))
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
            .filter(|(_, recipe)| inv.contains_ingredients(recipe).is_ok())
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
    #[serde(default)]
    pub(crate) craft_sprite: Option<SpriteKind>,
}

#[derive(Clone, Deserialize)]
#[serde(transparent)]
#[allow(clippy::type_complexity)]
pub(crate) struct RawRecipeBook(
    pub(crate) HashMap<String, RawRecipe>,
);

impl assets::Asset for RawRecipeBook {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

impl assets::Compound for RecipeBook {
    fn load<S: assets_manager::source::Source>(
        cache: &assets_manager::AssetCache<S>,
        specifier: &str,
    ) -> Result<Self, assets_manager::Error> {
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
                RawRecipeInput::Item(name) => {
                    RecipeInput::Item(Arc::<ItemDef>::load_cloned(&name)?)
                },
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
            .map(|(name, RawRecipe { output, inputs, craft_sprite })| {
                let inputs = inputs
                    .iter()
                    .map(load_recipe_input)
                    .collect::<Result<Vec<_>, _>>()?;
                let output = load_item_def(output)?;
                Ok((name.clone(), Recipe { output, inputs, craft_sprite: *craft_sprite }))
            })
            .collect::<Result<_, assets::Error>>()?;

        Ok(RecipeBook { recipes })
    }
}

pub fn default_recipe_book() -> AssetHandle<RecipeBook> {
    RecipeBook::load_expect("common.recipe_book")
}
