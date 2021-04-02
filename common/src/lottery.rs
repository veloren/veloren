// Example for calculating a drop rate:
//
// On every roll an f32 between 0 and 1 is created.
// For every loot table a total range is created by the sum of the individual
// ranges per item.
//
// This range is the sum of all single ranges defined per item in a table.
//                                                   // Individual Range
// (3, "common.items.food.cheese"),                  // 0.0..3.0
// (3, "common.items.food.apple"),                   // 3.0..6.0
// (3, "common.items.food.mushroom"),                // 6.0..9.0
// (1, "common.items.food.coconut"),                 // 9.0..10.0
// (0.05, "common.items.food.apple_mushroom_curry"), // 10.0..10.05
// (0.10, "common.items.food.apple_stick"),          // 10.05..10.15
// (0.10, "common.items.food.mushroom_stick"),       // 10.15..10.25
//
// The f32 is multiplied by the max. value needed to drop an item in this
// particular table. X = max. value needed = 10.15
//
// Example roll
// [Random Value 0..1] * X = Number inside the table's total range
// 0.45777 * X = 4.65
// 4.65 is in the range of 3.0..6.0 => Apple drops
//
// Example drop chance calculation
// Cheese drop rate = 3/X = 29.6%
// Coconut drop rate = 1/X = 9.85%

use crate::{
    assets::{self, AssetExt},
    comp::{Body, Item},
};
use rand::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Lottery<T> {
    items: Vec<(f32, T)>,
    total: f32,
}

impl<T: DeserializeOwned + Send + Sync + 'static> assets::Asset for Lottery<T> {
    type Loader = assets::LoadFrom<Vec<(f32, T)>, assets::RonLoader>;

    const EXTENSION: &'static str = "ron";
}

impl<T> From<Vec<(f32, T)>> for Lottery<T> {
    fn from(mut items: Vec<(f32, T)>) -> Lottery<T> {
        let mut total = 0.0;

        for (rate, _) in &mut items {
            total += *rate;
            *rate = total - *rate;
        }

        Self { items, total }
    }
}

impl<T> Lottery<T> {
    pub fn choose_seeded(&self, seed: u32) -> &T {
        let x = ((seed % 65536) as f32 / 65536.0) * self.total;
        &self.items[self
            .items
            .binary_search_by(|(y, _)| y.partial_cmp(&x).unwrap())
            .unwrap_or_else(|i| i.saturating_sub(1))]
        .1
    }

    pub fn choose(&self) -> &T { self.choose_seeded(thread_rng().gen()) }

    pub fn iter(&self) -> impl Iterator<Item = &(f32, T)> { self.items.iter() }

    pub fn total(&self) -> f32 { self.total }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum LootSpec {
    /// Asset specifier
    Item(String),
    /// Asset specifier, lower range, upper range
    ItemQuantity(String, u32, u32),
    /// Loot table
    LootTable(String),
    /// Matches on species to provide a crafting material
    CreatureMaterial,
}

impl LootSpec {
    #[allow(unused_must_use)]
    pub fn to_item(&self, body: Option<Body>) -> Item {
        match self {
            Self::Item(item) => Item::new_from_asset_expect(&item),
            Self::ItemQuantity(item, lower, upper) => {
                let range = *lower..=*upper;
                let quantity = thread_rng().gen_range(range);
                let mut item = Item::new_from_asset_expect(&item);
                item.set_amount(quantity);
                item
            },
            Self::LootTable(table) => Lottery::<LootSpec>::load_expect(&table)
                .read()
                .choose()
                .to_item(body),
            Self::CreatureMaterial => body.map_or(
                Item::new_from_asset_expect("common.items.food.cheese"),
                |b| b.get_material(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assets::{AssetExt, Error}, comp::Item};
    
    #[test]
    fn test_loot_tables() {
        #[derive(Clone)]
        struct LootTableList(Vec<Lottery<LootSpec>>);
        impl assets::Compound for LootTableList {
            fn load<S: assets::source::Source>(
                cache: &assets::AssetCache<S>,
                specifier: &str,
            ) -> Result<Self, Error> {
                let list = cache
                    .load::<assets::Directory>(specifier)?
                    .read()
                    .iter()
                    .map(|spec| Lottery::<LootSpec>::load_cloned(spec))
                    .collect::<Result<_, Error>>()?;
    
                Ok(LootTableList(list))
            }
        }

        fn validate_table_contents(table: Lottery<LootSpec>) {
            for (_, item) in table.iter() {
                match item {
                    LootSpec::Item(item) => {
                        Item::new_from_asset_expect(&item);
                    },
                    LootSpec::ItemQuantity(item, lower, upper) => {
                        assert!(*lower > 0, "Lower quantity must be more than 0. It is {}.", lower);
                        assert!(upper >= lower, "Upper quantity must be at least the value of lower quantity. Upper value: {}, low value: {}.", upper, lower);
                        Item::new_from_asset_expect(&item);
                    },
                    LootSpec::LootTable(loot_table) => {
                        let loot_table = Lottery::<LootSpec>::load_expect_cloned(&loot_table);
                        validate_table_contents(loot_table);
                    },
                    LootSpec::CreatureMaterial => {
                        item.to_item(None);
                    },
                }
            }
        }

        let loot_tables = LootTableList::load_expect_cloned("common.loot_tables.*").0;
        for loot_table in loot_tables {
            validate_table_contents(loot_table);
        }
    }
}
