use crate::{
    assets::{self, Asset},
    comp::{Inventory, Item},
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::{fs::File, io::BufReader, sync::Arc};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recipe {
    pub output: (Item, usize),
    pub inputs: Vec<(Item, usize)>,
}

#[allow(clippy::type_complexity)]
impl Recipe {
    /// Perform a recipe, returning a list of missing items on failure
    pub fn perform(
        &self,
        inv: &mut Inventory,
    ) -> Result<Option<(Item, usize)>, Vec<(&Item, usize)>> {
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
            if let Some(item) = inv.push(self.output.0.clone()) {
                return Ok(Some((item, self.output.1 - i)));
            }
        }

        Ok(None)
    }

    pub fn inputs(&self) -> impl ExactSizeIterator<Item = (&Item, usize)> {
        self.inputs.iter().map(|(item, amount)| (item, *amount))
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

impl Asset for RecipeBook {
    const ENDINGS: &'static [&'static str] = &["ron"];

    fn parse(buf_reader: BufReader<File>) -> Result<Self, assets::Error> {
        ron::de::from_reader::<
            BufReader<File>,
            HashMap<String, ((String, usize), Vec<(String, usize)>)>,
        >(buf_reader)
        .map_err(assets::Error::parse_error)
        .and_then(|recipes| {
            Ok(RecipeBook {
                recipes: recipes
                    .into_iter()
                    .map::<Result<(String, Recipe), assets::Error>, _>(
                        |(name, ((output, amount), inputs))| {
                            Ok((name, Recipe {
                                output: ((&*assets::load::<Item>(&output)?).clone(), amount),
                                inputs: inputs
                                    .into_iter()
                                    .map::<Result<(Item, usize), assets::Error>, _>(
                                        |(name, amount)| {
                                            Ok(((&*assets::load::<Item>(&name)?).clone(), amount))
                                        },
                                    )
                                    .collect::<Result<_, _>>()?,
                            }))
                        },
                    )
                    .collect::<Result<_, _>>()?,
            })
        })
    }
}

pub fn default_recipe_book() -> Arc<RecipeBook> { assets::load_expect("common.recipe_book") }
