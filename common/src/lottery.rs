use crate::assets::{self, Asset};
use rand::prelude::*;
use serde::{de::DeserializeOwned, Deserialize};
use std::{fs::File, io::BufReader};

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Lottery<T> {
    items: Vec<(f32, T)>,
    total: f32,
}

impl<T: DeserializeOwned + Send + Sync> Asset for Lottery<T> {
    const ENDINGS: &'static [&'static str] = &["ron"];

    fn parse(buf_reader: BufReader<File>, _specifier: &str) -> Result<Self, assets::Error> {
        ron::de::from_reader::<BufReader<File>, Vec<(f32, T)>>(buf_reader)
            .map(|items| Lottery::from_rates(items.into_iter()))
            .map_err(assets::Error::parse_error)
    }
}

impl<T> Lottery<T> {
    pub fn from_rates(items: impl Iterator<Item = (f32, T)>) -> Self {
        let mut total = 0.0;
        let items = items
            .map(|(rate, item)| {
                total += rate;
                (total - rate, item)
            })
            .collect();
        Self { items, total }
    }

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
    use crate::{assets::Asset, comp::Item};

    #[test]
    fn test_loot_table() {
        let test = Lottery::<String>::load_expect("common.loot_tables.loot_table");

        for (_, item_asset_specifier) in test.iter() {
            assert!(
                Item::new_from_asset(item_asset_specifier).is_ok(),
                "Invalid loot table item '{}'",
                item_asset_specifier
            );
        }
    }
}
