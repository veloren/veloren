use super::{
    good_list,
    map_types::{GoodIndex, GoodMap},
};
use crate::{
    assets::{self, AssetExt},
    util::DHashMap,
};
use common::{
    terrain::BiomeKind,
    trade::Good::{self, Terrain, Territory},
};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

const fn default_one() -> f32 { 1.0 }
const fn default_true() -> bool { true }

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RawGoodProperties {
    #[serde(default)]
    pub decay_rate: f32,
    #[serde(default = "default_one")]
    pub transport_effort: f32,
    #[serde(default = "default_true")]
    pub storable: bool,
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct RawGoodPropertiesList(DHashMap<Good, RawGoodProperties>);

impl assets::Asset for RawGoodPropertiesList {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

/// Contains caches used for economic simulation
pub struct EconomyCache {
    pub(crate) transport_effort: GoodMap<f32>,
    pub(crate) decay_rate: GoodMap<f32>,
    pub(crate) direct_use_goods: Vec<GoodIndex>,
}

lazy_static! {
    static ref CACHE: EconomyCache = load_cache();
}

pub fn cache() -> &'static EconomyCache { &CACHE }

fn load_cache() -> EconomyCache {
    let good_properties = RawGoodPropertiesList::load_expect("common.economy.trading_goods")
        .read()
        .0
        .clone();
    let mut decay_rate: GoodMap<f32> = GoodMap::from_default(0.0);
    let mut transport_effort: GoodMap<f32> = GoodMap::from_default(1.0);
    let mut direct_use_goods: Vec<GoodIndex> = Vec::new();

    for i in good_properties.iter() {
        if let Ok(rawgood) = (*i.0).try_into() {
            decay_rate[rawgood] = i.1.decay_rate;
            if !i.1.storable {
                direct_use_goods.push(rawgood);
            }
            transport_effort[rawgood] = i.1.transport_effort;
        } else {
            match *i.0 {
                Territory(BiomeKind::Void) => {
                    for j in good_list() {
                        if let Territory(_) = Good::from(j) {
                            decay_rate[j] = i.1.decay_rate;
                            transport_effort[j] = i.1.transport_effort;
                            if !i.1.storable {
                                direct_use_goods.push(j);
                            }
                        }
                    }
                },
                Terrain(BiomeKind::Void) => {
                    for j in good_list() {
                        if let Terrain(_) = Good::from(j) {
                            decay_rate[j] = i.1.decay_rate;
                            transport_effort[j] = i.1.transport_effort;
                            if !i.1.storable {
                                direct_use_goods.push(j);
                            }
                        }
                    }
                },
                _ => tracing::warn!("Raw good not in index: {:?}", i.0),
            }
        }
    }

    EconomyCache {
        transport_effort,
        decay_rate,
        direct_use_goods,
    }
}
