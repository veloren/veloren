use crate::{
    assets::{self, AssetExt, AssetHandle},
    comp::{item::ItemDef, Inventory, Item},
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recipe {
    pub output: (Arc<ItemDef>, u32),
    pub inputs: Vec<(Arc<ItemDef>, u32)>,
}

#[allow(clippy::type_complexity)]
impl Recipe {
    /// Perform a recipe, returning a list of missing items on failure
    pub fn perform(
        &self,
        inv: &mut Inventory,
    ) -> Result<Option<(Item, u32)>, Vec<(&ItemDef, u32)>> {
        // Get ingredient cells from inventory,
        inv.contains_ingredients(self)?
            .into_iter()
            .enumerate()
            .for_each(|(i, n)| {
                (0..n).for_each(|_| {
                    inv.take(i).expect("Expected item to exist in inventory");
                })
            });

        for i in 0..self.output.1 {
            let crafted_item = Item::new(Arc::clone(&self.output.0));
            if let Some(item) = inv.push(crafted_item) {
                return Ok(Some((item, self.output.1 - i)));
            }
        }

        Ok(None)
    }

    pub fn inputs(&self) -> impl ExactSizeIterator<Item = (&Arc<ItemDef>, u32)> {
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

#[derive(Deserialize)]
#[serde(transparent)]
struct RawRecipeBook(HashMap<String, ((String, u32), Vec<(String, u32)>)>);

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

        let raw = cache.load::<RawRecipeBook>(specifier)?.read();

        let recipes = raw
            .0
            .iter()
            .map(|(name, (output, inputs))| {
                let inputs = inputs.iter().map(load_item_def).collect::<Result<_, _>>()?;
                let output = load_item_def(output)?;
                Ok((name.clone(), Recipe { inputs, output }))
            })
            .collect::<Result<_, assets::Error>>()?;

        Ok(RecipeBook { recipes })
    }
}

pub fn default_recipe_book() -> AssetHandle<RecipeBook> {
    RecipeBook::load_expect("common.recipe_book")
}
