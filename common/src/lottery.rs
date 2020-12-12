use crate::assets;
use rand::prelude::*;
use serde::{de::DeserializeOwned, Deserialize};

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Lottery<T> {
    items: Vec<(f32, T)>,
    total: f32,
}

impl<T: DeserializeOwned + Send + Sync + 'static> assets::Asset for Lottery<T> {
    const EXTENSION: &'static str = "ron";
    type Loader = assets::LoadFrom<Vec<(f32, T)>, assets::RonLoader>;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assets::AssetExt, comp::Item};

    #[test]
    fn test_loot_table() {
        let test = Lottery::<String>::load_expect("common.loot_tables.loot_table");

        for (_, item_asset_specifier) in test.read().iter() {
            assert!(
                Item::new_from_asset(item_asset_specifier).is_ok(),
                "Invalid loot table item '{}'",
                item_asset_specifier
            );
        }
    }
}
