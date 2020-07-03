use crate::{
    assets::{self, Asset},
};
use rand::prelude::*;
use std::{fs::File, io::BufReader};

// Generate a random float between 0 and 1
pub fn rand() -> f32 {
    let mut rng = rand::thread_rng();
    rng.gen()
}

pub struct Lottery<T> {
    items: Vec<f32, T>,
    total: f32,
}

impl Asset for Lottery<&str> {
    const ENDINGS: &'static [&'static str] = &["ron"];

    fn parse(buf_reader: BufReader<File>) -> Result<Self, assets::Error> {
        ron::de::from_reader(buf_reader).map_err(assets::Error::parse_error)
    }
}

impl<T> Lottery<T> {
    pub fn from_rates(items: impl Iterator<Item=(f32, T)>) -> Self {
        let mut total = 0.0;
        let items = items
            .map(|(rate, item| {
                total += rate;
                (total - rate, item)
            })
            .collect();
        Self { items, total }
    }

    pub fn choose(&self) -> &T {
        let x = rand() * self.total;
        &self.items[self.items
            .binary_search_by(|(y, _)|, y.partial_cmp(&x).unwrap())
            .unwrap_or_else(|i| i.saturating_sub(1))].1
    }
}