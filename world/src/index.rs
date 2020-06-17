use crate::site::Site;
use common::store::{Id, Store};

#[derive(Default)]
pub struct Index {
    pub time: f32,
    pub sites: Store<Site>,
}
