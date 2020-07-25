use crate::site::Site;
use common::store::{Id, Store};

#[derive(Default)]
pub struct Index {
    pub seed: u32,
    pub time: f32,
    pub sites: Store<Site>,
}

impl Index {
    pub fn new(seed: u32) -> Self {
        Self {
            seed,
            ..Self::default()
        }
    }
}
