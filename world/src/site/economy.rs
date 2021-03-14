use crate::{
    assets::{self, AssetExt, AssetHandle},
    sim::SimChunk,
    site::Site,
    util::{DHashMap, MapVec},
};
use common::{
    store::Id,
    terrain::BiomeKind,
    trade::{Good, SitePrices},
};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{fmt, marker::PhantomData, sync::Once};

use Good::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct Profession {
    pub name: String,
    pub orders: Vec<(Good, f32)>,
    pub products: Vec<(Good, f32)>,
}

// reference to profession
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Labor(u8, PhantomData<Profession>);

#[derive(Debug)]
pub struct AreaResources {
    pub resource_sum: MapVec<Good, f32>,
    pub resource_chunks: MapVec<Good, u32>,
    pub chunks: u32,
}

impl Default for AreaResources {
    fn default() -> Self {
        Self {
            resource_sum: MapVec::default(),
            resource_chunks: MapVec::default(),
            chunks: 0,
        }
    }
}

#[derive(Debug)]
pub struct NaturalResources {
    // resources per distance, we should increase labor cost for far resources
    pub per_area: Vec<AreaResources>,

    // computation simplifying cached values
    pub chunks_per_resource: MapVec<Good, u32>,
    pub average_yield_per_chunk: MapVec<Good, f32>,
}

impl Default for NaturalResources {
    fn default() -> Self {
        Self {
            per_area: Vec::new(),
            chunks_per_resource: MapVec::default(),
            average_yield_per_chunk: MapVec::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RawProfessions(Vec<Profession>);

impl assets::Asset for RawProfessions {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

pub fn default_professions() -> AssetHandle<RawProfessions> {
    RawProfessions::load_expect("common.professions")
}

lazy_static! {
    static ref LABOR: AssetHandle<RawProfessions> = default_professions();
    // used to define resources needed by every person
    static ref DUMMY_LABOR: Labor = Labor(
        LABOR
            .read()
            .0
            .iter()
            .position(|a| a.name == "_")
            .unwrap_or(0) as u8,
        PhantomData
    );
}

impl fmt::Debug for Labor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if (self.0 as usize) < LABOR.read().0.len() {
            f.write_str(&LABOR.read().0[self.0 as usize].name)
        } else {
            f.write_str("?")
        }
    }
}

#[derive(Debug)]
pub struct TradeOrder {
    pub customer: Id<Site>,
    pub amount: MapVec<Good, f32>, // positive for orders, negative for exchange
}

#[derive(Debug)]
pub struct TradeDelivery {
    pub supplier: Id<Site>,
    pub amount: MapVec<Good, f32>, // positive for orders, negative for exchange
    pub prices: MapVec<Good, f32>, // at the time of interaction
    pub supply: MapVec<Good, f32>, // maximum amount available, at the time of interaction
}

#[derive(Debug)]
pub struct TradeInformation {
    pub orders: DHashMap<Id<Site>, Vec<TradeOrder>>, // per provider
    pub deliveries: DHashMap<Id<Site>, Vec<TradeDelivery>>, // per receiver
}

impl Default for TradeInformation {
    fn default() -> Self {
        Self {
            orders: Default::default(),
            deliveries: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct NeighborInformation {
    pub id: Id<Site>,
    pub travel_distance: usize,

    // remembered from last interaction
    pub last_values: MapVec<Good, f32>,
    pub last_supplies: MapVec<Good, f32>,
}

#[derive(Debug)]
pub struct Economy {
    // Population
    pub pop: f32,

    /// Total available amount of each good
    pub stocks: MapVec<Good, f32>,
    /// Surplus stock compared to demand orders
    pub surplus: MapVec<Good, f32>,
    /// change rate (derivative) of stock in the current situation
    pub marginal_surplus: MapVec<Good, f32>,
    /// amount of wares not needed by the economy (helps with trade planning)
    pub unconsumed_stock: MapVec<Good, f32>,
    // For some goods, such a goods without any supply, it doesn't make sense to talk about value
    pub values: MapVec<Good, Option<f32>>,
    pub last_exports: MapVec<Good, f32>,
    pub active_exports: MapVec<Good, f32>, // unfinished trade (amount unconfirmed)
    //pub export_targets: MapVec<Good, f32>,
    pub labor_values: MapVec<Good, Option<f32>>,
    pub material_costs: MapVec<Good, f32>,

    // Proportion of individuals dedicated to an industry
    pub labors: MapVec<Labor, f32>,
    // Per worker, per year, of their output good
    pub yields: MapVec<Labor, f32>,
    pub productivity: MapVec<Labor, f32>,

    pub natural_resources: NaturalResources,
    // usize is distance
    pub neighbors: Vec<NeighborInformation>,
}

static INIT: Once = Once::new();

impl Default for Economy {
    fn default() -> Self {
        INIT.call_once(|| {
            LABOR.read();
        });
        Self {
            pop: 32.0,

            stocks: MapVec::from_list(&[(Coin, Economy::STARTING_COIN)], 100.0),
            surplus: Default::default(),
            marginal_surplus: Default::default(),
            values: MapVec::from_list(&[(Coin, Some(2.0))], None),
            last_exports: Default::default(),
            active_exports: Default::default(),

            labor_values: Default::default(),
            material_costs: Default::default(),

            labors: MapVec::from_default(0.01),
            yields: MapVec::from_default(1.0),
            productivity: MapVec::from_default(1.0),

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
        for &g in good_list() {
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
            if chunks != 0 {
                self.natural_resources.chunks_per_resource[g] = chunks;
                self.natural_resources.average_yield_per_chunk[g] = amount / (chunks as f32);
            }
        }
    }

    pub fn get_orders(&self) -> DHashMap<Option<Labor>, Vec<(Good, f32)>> {
        LABOR
            .read()
            .0
            .iter()
            .enumerate()
            .map(|(i, p)| {
                (
                    if p.name == "_" {
                        None
                    } else {
                        Some(Labor(i as u8, PhantomData))
                    },
                    p.orders.clone(),
                )
            })
            .collect()
    }

    pub fn get_productivity(&self) -> MapVec<Labor, Vec<(Good, f32)>> {
        let products: MapVec<Labor, Vec<(Good, f32)>> = MapVec::from_iter(
            LABOR
                .read()
                .0
                .iter()
                .enumerate()
                .filter(|(_, p)| !p.products.is_empty())
                .map(|(i, p)| (Labor(i as u8, PhantomData), p.products.clone())),
            vec![(Good::Terrain(BiomeKind::Void), 0.0)],
        );
        products.map(|l, vec| {
            let labor_ratio = self.labors[l];
            let total_workers = labor_ratio * self.pop;
            // apply economy of scale (workers get more productive in numbers)
            let relative_scale = 1.0 + labor_ratio;
            let absolute_scale = (1.0 + total_workers / 100.0).min(3.0);
            let scale = relative_scale * absolute_scale;
            vec.iter()
                .map(|(good, amount)| (*good, amount * scale))
                .collect()
        })
    }

    pub fn replenish(&mut self, _time: f32) {
        for (good, &ch) in self.natural_resources.chunks_per_resource.iter() {
            let per_year = self.natural_resources.average_yield_per_chunk[good] * (ch as f32);
            self.stocks[good] = self.stocks[good].max(per_year);
        }
        // info!("resources {:?}", self.stocks);
    }

    pub fn add_chunk(&mut self, ch: &SimChunk, distance_squared: i32) {
        let biome = ch.get_biome();
        // we don't scale by pi, although that would be correct
        let distance_bin = (distance_squared >> 16).min(64) as usize;
        if self.natural_resources.per_area.len() <= distance_bin {
            self.natural_resources
                .per_area
                .resize_with(distance_bin + 1, Default::default);
        }
        self.natural_resources.per_area[distance_bin].chunks += 1;
        self.natural_resources.per_area[distance_bin].resource_sum[Terrain(biome)] += 1.0;
        self.natural_resources.per_area[distance_bin].resource_chunks[Terrain(biome)] += 1;
        // TODO: Scale resources by rockiness or tree_density?
    }

    pub fn add_neighbor(&mut self, id: Id<Site>, distance: usize) {
        self.neighbors.push(NeighborInformation {
            id,
            travel_distance: distance,

            last_values: MapVec::from_default(Economy::MINIMUM_PRICE),
            last_supplies: Default::default(),
        });
    }

    pub fn get_site_prices(&self) -> SitePrices {
        SitePrices {
            values: self
                .values
                .iter()
                .map(|(g, v)| (g, v.unwrap_or(Economy::MINIMUM_PRICE)))
                .collect(),
        }
    }
}

pub fn good_list() -> &'static [Good] {
    static GOODS: [Good; 23] = [
        // controlled resources
        Territory(BiomeKind::Grassland),
        Territory(BiomeKind::Forest),
        Territory(BiomeKind::Lake),
        Territory(BiomeKind::Ocean),
        Territory(BiomeKind::Mountain),
        RoadSecurity,
        Ingredients,
        // produced goods
        Flour,
        Meat,
        Wood,
        Stone,
        Food,
        Tools,
        Armor,
        Potions,
        Transportation,
        // exchange currency
        Coin,
        // uncontrolled resources
        Terrain(BiomeKind::Lake),
        Terrain(BiomeKind::Mountain),
        Terrain(BiomeKind::Grassland),
        Terrain(BiomeKind::Forest),
        Terrain(BiomeKind::Desert),
        Terrain(BiomeKind::Ocean),
    ];

    &GOODS
}

pub fn transportation_effort(g: Good) -> f32 {
    match g {
        Terrain(_) | Territory(_) | RoadSecurity => 0.0,
        Coin => 0.01,
        Potions => 0.1,

        Armor => 2.0,
        Stone => 4.0,

        _ => 1.0,
    }
}

pub fn decay_rate(g: Good) -> f32 {
    match g {
        Food => 0.2,
        Flour => 0.1,
        Meat => 0.25,
        Ingredients => 0.1,
        _ => 0.0,
    }
}

/** you can't accumulate or save these options/resources for later */
pub fn direct_use_goods() -> &'static [Good] {
    static DIRECT_USE: [Good; 13] = [
        Transportation,
        Territory(BiomeKind::Grassland),
        Territory(BiomeKind::Forest),
        Territory(BiomeKind::Lake),
        Territory(BiomeKind::Ocean),
        Territory(BiomeKind::Mountain),
        RoadSecurity,
        Terrain(BiomeKind::Grassland),
        Terrain(BiomeKind::Forest),
        Terrain(BiomeKind::Lake),
        Terrain(BiomeKind::Ocean),
        Terrain(BiomeKind::Mountain),
        Terrain(BiomeKind::Desert),
    ];
    &DIRECT_USE
}

impl Labor {
    pub fn list() -> impl Iterator<Item = Self> {
        (0..LABOR.read().0.len())
            .filter(|&i| i != (DUMMY_LABOR.0 as usize))
            .map(|i| Self(i as u8, PhantomData))
    }
}
