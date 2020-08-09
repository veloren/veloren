use crate::site::Site;
use common::store::{Id, Store};
use noise::{NoiseFn, Seedable, SuperSimplex};

pub struct Index {
    pub seed: u32,
    pub time: f32,
    pub noise: Noise,
    pub sites: Store<Site>,
}

impl Index {
    pub fn new(seed: u32) -> Self {
        Self {
            seed,
            time: 0.0,
            noise: Noise::new(seed),
            sites: Store::default(),
        }
    }
}

pub struct Noise {
    pub cave_nz: SuperSimplex,
    pub scatter_nz: SuperSimplex,
}

impl Noise {
    fn new(seed: u32) -> Self {
        Self {
            cave_nz: SuperSimplex::new().set_seed(seed + 0),
            scatter_nz: SuperSimplex::new().set_seed(seed + 1),
        }
    }
}
