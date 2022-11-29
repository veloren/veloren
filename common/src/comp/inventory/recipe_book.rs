use crate::{
    comp::item::{Item, ItemKind},
    recipe::{Recipe, RecipeBookManifest},
};
use hashbrown::HashSet;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RecipeBook {
    recipe_groups: Vec<Item>,
    recipes: HashSet<String>,
}

impl RecipeBook {
    pub(super) fn get<'a>(
        &'a self,
        recipe_key: &str,
        rbm: &'a RecipeBookManifest,
    ) -> Option<&Recipe> {
        if self.recipes.iter().any(|r| r == recipe_key) {
            rbm.get(recipe_key)
        } else {
            None
        }
    }

    pub(super) fn len(&self) -> usize { self.recipes.len() }

    pub(super) fn iter(&self) -> impl ExactSizeIterator<Item = &String> { self.recipes.iter() }

    pub(super) fn get_available_iter<'a>(
        &'a self,
        rbm: &'a RecipeBookManifest,
    ) -> impl Iterator<Item = (&String, &Recipe)> + '_ {
        self.recipes
            .iter()
            .filter_map(|recipe: &String| rbm.get(recipe).map(|rbm_recipe| (recipe, rbm_recipe)))
    }

    pub(super) fn reset(&mut self) {
        self.recipe_groups.clear();
        self.recipes.clear();
    }

    /// Pushes a group of recipes to the recipe book. If group already exists
    /// return the recipe group.
    pub(super) fn push_group(&mut self, group: Item) -> Result<(), Item> {
        if self
            .recipe_groups
            .iter()
            .any(|rg| rg.item_definition_id() == group.item_definition_id())
        {
            Err(group)
        } else {
            self.recipe_groups.push(group);
            self.update();
            Ok(())
        }
    }

    /// Syncs recipes hashset with recipe_groups vec
    pub(super) fn update(&mut self) {
        self.recipe_groups.iter().for_each(|group| {
            if let ItemKind::RecipeGroup { recipes } = &*group.kind() {
                self.recipes.extend(recipes.iter().map(String::from))
            }
        })
    }

    pub fn recipe_book_from_persistence(recipe_groups: Vec<Item>) -> Self {
        let mut book = Self {
            recipe_groups,
            recipes: HashSet::new(),
        };
        book.update();
        book
    }

    pub fn persistence_recipes_iter_with_index(&self) -> impl Iterator<Item = (usize, &Item)> {
        self.recipe_groups.iter().enumerate()
    }

    pub(super) fn is_known(&self, recipe_key: &str) -> bool { self.recipes.contains(recipe_key) }
}

#[cfg(test)]
mod tests {
    use crate::{
        comp::item::{Item, ItemKind},
        recipe::{complete_recipe_book, default_component_recipe_book},
    };

    fn valid_recipe(recipe: &str) -> bool {
        let recipe_book = complete_recipe_book();
        let component_recipe_book = default_component_recipe_book();

        recipe_book.read().keys().any(|key| key == recipe)
            || component_recipe_book
                .read()
                .iter()
                .any(|(_, cr)| cr.recipe_book_key == recipe)
    }

    /// Verify that all recipes in recipe items point to a valid recipe
    #[test]
    fn validate_recipes() {
        let groups = Item::new_from_asset_glob("common.items.recipes.*")
            .expect("The directory should exist");
        for group in groups {
            let ItemKind::RecipeGroup { recipes } = &*group.kind() else {
                panic!("Expected item to be of kind RecipeGroup")
            };
            assert!(recipes.iter().all(|r| valid_recipe(r)));
        }
    }
}
