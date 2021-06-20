use crate::{
    assets::{self, AssetExt},
    sim::SimChunk,
    site::Site,
    util::{
        map_array::{enum_from_index, index_from_enum, GenericIndex, NotFound},
        DHashMap,
    },
};
use common::{
    store::Id,
    terrain::BiomeKind,
    trade::{Good, SitePrices},
};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{
    convert::{TryFrom, TryInto},
    fmt::{self, Write},
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use Good::*;

// the opaque index type into the "map" of Goods
#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GoodIndex {
    idx: usize,
}

impl GenericIndex<Good, 23> for GoodIndex {
    // static list of all Goods traded
    const VALUES: [Good; GoodIndex::LENGTH] = [
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

    fn from_usize(idx: usize) -> Self { Self { idx } }

    fn into_usize(self) -> usize { self.idx }
}

impl TryFrom<Good> for GoodIndex {
    type Error = NotFound;

    fn try_from(e: Good) -> Result<Self, NotFound> { index_from_enum(e) }
}

impl From<GoodIndex> for Good {
    fn from(gi: GoodIndex) -> Good { enum_from_index(gi) }
}

impl std::fmt::Debug for GoodIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        GoodIndex::VALUES[self.idx].fmt(f)
    }
}

// the "map" itself
#[derive(Copy, Clone)]
pub struct GoodMap<V> {
    data: [V; GoodIndex::LENGTH],
}

impl<V: Default + Copy> Default for GoodMap<V> {
    fn default() -> Self {
        GoodMap {
            data: [V::default(); GoodIndex::LENGTH],
        }
    }
}

impl<V: Default + Copy + PartialEq + fmt::Debug> fmt::Debug for GoodMap<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("{ ")?;
        for i in self.iter() {
            if *i.1 != V::default() {
                Good::from(i.0).fmt(f)?;
                f.write_char(':')?;
                i.1.fmt(f)?;
                f.write_char(' ')?;
            }
        }
        f.write_char('}')
    }
}

impl<V> Index<GoodIndex> for GoodMap<V> {
    type Output = V;

    fn index(&self, index: GoodIndex) -> &Self::Output { &self.data[index.idx] }
}

impl<V> IndexMut<GoodIndex> for GoodMap<V> {
    fn index_mut(&mut self, index: GoodIndex) -> &mut Self::Output { &mut self.data[index.idx] }
}

impl<V> GoodMap<V> {
    pub fn iter(&self) -> impl Iterator<Item = (GoodIndex, &V)> + '_ {
        (&self.data)
            .iter()
            .enumerate()
            .map(|(idx, v)| (GoodIndex { idx }, v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (GoodIndex, &mut V)> + '_ {
        (&mut self.data)
            .iter_mut()
            .enumerate()
            .map(|(idx, v)| (GoodIndex { idx }, v))
    }
}

impl<V: Copy> GoodMap<V> {
    pub fn from_default(default: V) -> Self {
        GoodMap {
            data: [default; GoodIndex::LENGTH],
        }
    }

    pub fn from_iter(i: impl Iterator<Item = (GoodIndex, V)>, default: V) -> Self {
        let mut result = Self::from_default(default);
        for j in i {
            result.data[j.0.idx] = j.1;
        }
        result
    }

    pub fn map<U: Default + Copy>(self, mut f: impl FnMut(GoodIndex, V) -> U) -> GoodMap<U> {
        let mut result = GoodMap::<U>::from_default(U::default());
        for j in self.data.iter().enumerate() {
            result.data[j.0] = f(GoodIndex::from_usize(j.0), *j.1);
        }
        result
    }

    pub fn from_list<'a>(i: impl IntoIterator<Item = &'a (GoodIndex, V)>, default: V) -> Self
    where
        V: 'a,
    {
        let mut result = Self::from_default(default);
        for j in i {
            result.data[j.0.idx] = j.1;
        }
        result
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct RawProfession {
    pub name: String,
    pub orders: Vec<(Good, f32)>,
    pub products: Vec<(Good, f32)>,
}

#[derive(Debug)]
pub struct Profession {
    pub name: String,
    pub orders: Vec<(GoodIndex, f32)>,
    pub products: (GoodIndex, f32),
}

// reference to profession
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Labor(u8, PhantomData<Profession>);

// the opaque index type into the "map" of Labors (as Labor already contains a
// monotonous index we reuse it)
pub type LaborIndex = Labor;

impl LaborIndex {
    fn from_usize(idx: usize) -> Self { Self(idx as u8, PhantomData) }

    fn into_usize(self) -> usize { self.0 as usize }
}

// the "map" itself
#[derive(Clone)]
pub struct LaborMap<V> {
    data: Vec<V>,
}

impl<V: Default + Copy> Default for LaborMap<V> {
    fn default() -> Self {
        LaborMap {
            data: std::iter::repeat(V::default()).take(*LABOR_COUNT).collect(),
        }
    }
}

impl<V: Default + Copy + PartialEq + fmt::Debug> fmt::Debug for LaborMap<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("{ ")?;
        for i in self.iter() {
            if *i.1 != V::default() {
                i.0.fmt(f)?;
                f.write_char(':')?;
                (*i.1).fmt(f)?;
                f.write_char(' ')?;
            }
        }
        f.write_char('}')
    }
}

impl<V> Index<LaborIndex> for LaborMap<V> {
    type Output = V;

    fn index(&self, index: LaborIndex) -> &Self::Output { &self.data[index.into_usize()] }
}

impl<V> IndexMut<LaborIndex> for LaborMap<V> {
    fn index_mut(&mut self, index: LaborIndex) -> &mut Self::Output {
        &mut self.data[index.into_usize()]
    }
}

impl<V> LaborMap<V> {
    pub fn iter(&self) -> impl Iterator<Item = (LaborIndex, &V)> + '_ {
        (&self.data)
            .iter()
            .enumerate()
            .map(|(idx, v)| (LaborIndex::from_usize(idx), v))
    }
}

impl<V: Copy + Default> LaborMap<V> {
    pub fn from_default(default: V) -> Self {
        LaborMap {
            data: std::iter::repeat(default).take(*LABOR_COUNT).collect(),
        }
    }
}

impl<V: Copy + Default> LaborMap<V> {
    pub fn from_iter(i: impl Iterator<Item = (LaborIndex, V)>, default: V) -> Self {
        let mut result = Self::from_default(default);
        for j in i {
            result.data[j.0.into_usize()] = j.1;
        }
        result
    }

    pub fn map<U: Default + Copy>(&self, f: impl Fn(LaborIndex, &V) -> U) -> LaborMap<U> {
        LaborMap {
            data: self.iter().map(|i| f(i.0, i.1)).collect(),
        }
    }
}

#[derive(Debug)]
pub struct AreaResources {
    pub resource_sum: GoodMap<f32>,
    pub resource_chunks: GoodMap<f32>,
    pub chunks: u32,
}

impl Default for AreaResources {
    fn default() -> Self {
        Self {
            resource_sum: GoodMap::default(),
            resource_chunks: GoodMap::default(),
            chunks: 0,
        }
    }
}

#[derive(Debug)]
pub struct NaturalResources {
    // resources per distance, we should increase labor cost for far resources
    pub per_area: Vec<AreaResources>,

    // computation simplifying cached values
    pub chunks_per_resource: GoodMap<f32>,
    pub average_yield_per_chunk: GoodMap<f32>,
}

impl Default for NaturalResources {
    fn default() -> Self {
        Self {
            per_area: Vec::new(),
            chunks_per_resource: GoodMap::default(),
            average_yield_per_chunk: GoodMap::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RawProfessions(Vec<RawProfession>);

impl assets::Asset for RawProfessions {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

pub fn default_professions() -> Vec<Profession> {
    RawProfessions::load_expect("common.professions")
        .read()
        .0
        .iter()
        .map(|r| Profession {
            name: r.name.clone(),
            orders: r
                .orders
                .iter()
                .map(|i| (i.0.try_into().unwrap_or_default(), i.1))
                .collect(),
            products: r
                .products
                .first()
                .map(|p| (p.0.try_into().unwrap_or_default(), p.1))
                .unwrap_or_default(),
        })
        .collect()
}

lazy_static! {
    static ref LABOR: Vec<Profession> = default_professions();
    // used to define resources needed by every person
    static ref DUMMY_LABOR: Labor = Labor(
        LABOR
            .iter()
            .position(|a| a.name == "_")
            .unwrap_or(0) as u8,
        PhantomData
    );
    // do not count the DUMMY_LABOR (has to be last entry)
    static ref LABOR_COUNT: usize = LABOR.len()-1;
}

impl fmt::Debug for Labor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if (self.0 as usize) < *LABOR_COUNT {
            f.write_str(&LABOR[self.0 as usize].name)
        } else {
            f.write_str("?")
        }
    }
}

impl Default for Labor {
    fn default() -> Self { *DUMMY_LABOR }
}

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
        LABOR
            .iter()
            .enumerate()
            .map(|(i, p)| {
                (
                    if i == DUMMY_LABOR.0 as usize {
                        None
                    } else {
                        Some(LaborIndex::from_usize(i))
                    },
                    p.orders.clone(),
                )
            })
            .collect()
    }

    pub fn get_productivity(&self) -> LaborMap<(GoodIndex, f32)> {
        // cache the site independent part of production
        lazy_static! {
            static ref PRODUCTS: LaborMap<(GoodIndex, f32)> = LaborMap::from_iter(
                LABOR
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.products.1 > 0.0)
                    .map(|(i, p)| { (LaborIndex::from_usize(i), p.products,) }),
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

impl Labor {
    pub fn list() -> impl Iterator<Item = Self> {
        (0..LABOR.len())
            .filter(|&i| i != (DUMMY_LABOR.0 as usize))
            .map(|i| Self(i as u8, PhantomData))
    }
}
