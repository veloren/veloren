use crate::{
    sim::SimChunk,
    site::Site,
    util::{map_array::GenericIndex, DHashMap},
};
use common::{
    store::Id,
    terrain::BiomeKind,
    trade::{Good, SitePrices},
};
use lazy_static::lazy_static;
use std::convert::TryFrom;

use Good::*;
mod map_types;
pub use map_types::{GoodIndex, GoodMap, Labor, LaborIndex, LaborMap, NaturalResources};

#[derive(Debug)]
pub struct TradeOrder {
    pub customer: Id<Site>,
    pub amount: GoodMap<f32>, // positive for orders, negative for exchange
}

#[derive(Debug)]
pub struct TradeDelivery {
    pub supplier: Id<Site>,
    pub amount: GoodMap<f32>, // positive for orders, negative for exchange
    pub prices: GoodMap<f32>, // at the time of interaction
    pub supply: GoodMap<f32>, // maximum amount available, at the time of interaction
}

#[derive(Debug, Default)]
pub struct TradeInformation {
    pub orders: DHashMap<Id<Site>, Vec<TradeOrder>>, // per provider
    pub deliveries: DHashMap<Id<Site>, Vec<TradeDelivery>>, // per receiver
}

#[derive(Debug)]
pub struct NeighborInformation {
    pub id: Id<Site>,
    pub travel_distance: usize,

    // remembered from last interaction
    pub last_values: GoodMap<f32>,
    pub last_supplies: GoodMap<f32>,
}

#[derive(Debug)]
pub struct Economy {
    // Population
    pub pop: f32,

    /// Total available amount of each good
    pub stocks: GoodMap<f32>,
    /// Surplus stock compared to demand orders
    pub surplus: GoodMap<f32>,
    /// change rate (derivative) of stock in the current situation
    pub marginal_surplus: GoodMap<f32>,
    /// amount of wares not needed by the economy (helps with trade planning)
    pub unconsumed_stock: GoodMap<f32>,
    // For some goods, such a goods without any supply, it doesn't make sense to talk about value
    pub values: GoodMap<Option<f32>>,
    pub last_exports: GoodMap<f32>,
    pub active_exports: GoodMap<f32>, // unfinished trade (amount unconfirmed)
    //pub export_targets: GoodMap<f32>,
    pub labor_values: GoodMap<Option<f32>>,
    pub material_costs: GoodMap<f32>,

    // Proportion of individuals dedicated to an industry
    pub labors: LaborMap<f32>,
    // Per worker, per year, of their output good
    pub yields: LaborMap<f32>,
    pub productivity: LaborMap<f32>,

    pub natural_resources: NaturalResources,
    // usize is distance
    pub neighbors: Vec<NeighborInformation>,
}

impl Default for Economy {
    fn default() -> Self {
        let coin_index: GoodIndex = GoodIndex::try_from(Coin).unwrap_or_default();
        Self {
            pop: 32.0,

            stocks: GoodMap::from_list(&[(coin_index, Economy::STARTING_COIN)], 100.0),
            surplus: Default::default(),
            marginal_surplus: Default::default(),
            values: GoodMap::from_list(&[(coin_index, Some(2.0))], None),
            last_exports: Default::default(),
            active_exports: Default::default(),

            labor_values: Default::default(),
            material_costs: Default::default(),

            labors: LaborMap::from_default(0.01),
            yields: LaborMap::from_default(1.0),
            productivity: LaborMap::from_default(1.0),

            natural_resources: Default::default(),
            neighbors: Default::default(),
            unconsumed_stock: Default::default(),
        }
    }
}

impl Economy {
    pub const MINIMUM_PRICE: f32 = 0.1;
    pub const STARTING_COIN: f32 = 1000.0;
    const _NATURAL_RESOURCE_SCALE: f32 = 1.0 / 9.0;

    pub fn cache_economy(&mut self) {
        for g in good_list() {
            let amount: f32 = self
                .natural_resources
                .per_area
                .iter()
                .map(|a| a.resource_sum[g])
                .sum();
            let chunks = self
                .natural_resources
                .per_area
                .iter()
                .map(|a| a.resource_chunks[g])
                .sum();
            if chunks > 0.001 {
                self.natural_resources.chunks_per_resource[g] = chunks;
                self.natural_resources.average_yield_per_chunk[g] = amount / chunks;
            }
        }
    }

    pub fn get_orders(&self) -> DHashMap<Option<LaborIndex>, Vec<(GoodIndex, f32)>> {
        Labor::list_full()
            .map(|l| (if l.is_everyone() { None } else { Some(l) }, l.orders()))
            .collect()
    }

    pub fn get_productivity(&self) -> LaborMap<(GoodIndex, f32)> {
        // cache the site independent part of production
        lazy_static! {
            static ref PRODUCTS: LaborMap<(GoodIndex, f32)> = LaborMap::from_iter(
                Labor::list().map(|p| { (p, p.products(),) }),
                (GoodIndex::default(), 0.0),
            );
        }
        PRODUCTS.map(|l, vec| {
            //dbg!((l,vec));
            let labor_ratio = self.labors[l];
            let total_workers = labor_ratio * self.pop;
            // apply economy of scale (workers get more productive in numbers)
            let relative_scale = 1.0 + labor_ratio;
            let absolute_scale = (1.0 + total_workers / 100.0).min(3.0);
            let scale = relative_scale * absolute_scale;
            (vec.0, vec.1 * scale)
        })
    }

    pub fn replenish(&mut self, _time: f32) {
        for (good, &ch) in self.natural_resources.chunks_per_resource.iter() {
            let per_year = self.natural_resources.average_yield_per_chunk[good] * ch;
            self.stocks[good] = self.stocks[good].max(per_year);
        }
        // info!("resources {:?}", self.stocks);
    }

    pub fn add_chunk(&mut self, ch: &SimChunk, distance_squared: i64) {
        // let biome = ch.get_biome();
        // we don't scale by pi, although that would be correct
        let distance_bin = (distance_squared >> 16).min(64) as usize;
        if self.natural_resources.per_area.len() <= distance_bin {
            self.natural_resources
                .per_area
                .resize_with(distance_bin + 1, Default::default);
        }
        self.natural_resources.per_area[distance_bin].chunks += 1;

        let mut add_biome = |biome, amount| {
            if let Ok(idx) = GoodIndex::try_from(Terrain(biome)) {
                self.natural_resources.per_area[distance_bin].resource_sum[idx] += amount;
                self.natural_resources.per_area[distance_bin].resource_chunks[idx] += amount;
            }
        };
        if ch.river.is_ocean() {
            add_biome(BiomeKind::Ocean, 1.0);
        } else if ch.river.is_lake() {
            add_biome(BiomeKind::Lake, 1.0);
        } else {
            add_biome(BiomeKind::Forest, 0.5 + ch.tree_density);
            add_biome(BiomeKind::Grassland, 0.5 + ch.humidity);
            add_biome(BiomeKind::Jungle, 0.5 + ch.humidity * ch.temp.max(0.0));
            add_biome(BiomeKind::Mountain, 0.5 + (ch.alt / 4000.0).max(0.0));
            add_biome(
                BiomeKind::Desert,
                0.5 + (1.0 - ch.humidity) * ch.temp.max(0.0),
            );
            add_biome(BiomeKind::Snowland, 0.5 + (-ch.temp).max(0.0));
        }
    }

    pub fn add_neighbor(&mut self, id: Id<Site>, distance: usize) {
        self.neighbors.push(NeighborInformation {
            id,
            travel_distance: distance,

            last_values: GoodMap::from_default(Economy::MINIMUM_PRICE),
            last_supplies: Default::default(),
        });
    }

    pub fn get_site_prices(&self) -> SitePrices {
        let normalize = |xs: GoodMap<Option<f32>>| {
            let sum = xs
                .iter()
                .map(|(_, x)| (*x).unwrap_or(0.0))
                .sum::<f32>()
                .max(0.001);
            xs.map(|_, x| Some(x? / sum))
        };

        SitePrices {
            values: {
                let labor_values = normalize(self.labor_values);
                // Use labor values as prices. Not correct (doesn't care about exchange value)
                let prices = normalize(self.values).map(|good, value| {
                    (labor_values[good].unwrap_or(Economy::MINIMUM_PRICE)
                        + value.unwrap_or(Economy::MINIMUM_PRICE))
                        * 0.5
                });
                prices.iter().map(|(g, v)| (Good::from(g), *v)).collect()
            },
        }
    }
}

pub fn good_list() -> impl Iterator<Item = GoodIndex> {
    (0..GoodIndex::LENGTH).map(GoodIndex::from_usize)
}

// cache in GoodMap ?
pub fn transportation_effort(g: GoodIndex) -> f32 {
    match Good::from(g) {
        Terrain(_) | Territory(_) | RoadSecurity => 0.0,
        Coin => 0.01,
        Potions => 0.1,

        Armor => 2.0,
        Stone => 4.0,

        _ => 1.0,
    }
}

pub fn decay_rate(g: GoodIndex) -> f32 {
    match Good::from(g) {
        Food => 0.2,
        Flour => 0.1,
        Meat => 0.25,
        Ingredients => 0.1,
        _ => 0.0,
    }
}

/** you can't accumulate or save these options/resources for later */
pub fn direct_use_goods() -> &'static [GoodIndex] {
    lazy_static! {
        static ref DIRECT_USE: [GoodIndex; 13] = [
            GoodIndex::try_from(Transportation).unwrap_or_default(),
            GoodIndex::try_from(Territory(BiomeKind::Grassland)).unwrap_or_default(),
            GoodIndex::try_from(Territory(BiomeKind::Forest)).unwrap_or_default(),
            GoodIndex::try_from(Territory(BiomeKind::Lake)).unwrap_or_default(),
            GoodIndex::try_from(Territory(BiomeKind::Ocean)).unwrap_or_default(),
            GoodIndex::try_from(Territory(BiomeKind::Mountain)).unwrap_or_default(),
            GoodIndex::try_from(RoadSecurity).unwrap_or_default(),
            GoodIndex::try_from(Terrain(BiomeKind::Grassland)).unwrap_or_default(),
            GoodIndex::try_from(Terrain(BiomeKind::Forest)).unwrap_or_default(),
            GoodIndex::try_from(Terrain(BiomeKind::Lake)).unwrap_or_default(),
            GoodIndex::try_from(Terrain(BiomeKind::Ocean)).unwrap_or_default(),
            GoodIndex::try_from(Terrain(BiomeKind::Mountain)).unwrap_or_default(),
            GoodIndex::try_from(Terrain(BiomeKind::Desert)).unwrap_or_default(),
        ];
    }
    &*DIRECT_USE
}
