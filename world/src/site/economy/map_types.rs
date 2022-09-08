use crate::{
    assets::{self, AssetExt},
    util::map_array::{enum_from_index, index_from_enum, GenericIndex, NotFound},
};
use common::{terrain::BiomeKind, trade::Good};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{
    convert::{TryFrom, TryInto},
    fmt,
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

impl fmt::Debug for GoodIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { GoodIndex::VALUES[self.idx].fmt(f) }
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
        f.debug_map()
            .entries(
                self.iter()
                    .filter(|i| *i.1 != V::default())
                    .map(|i| (Good::from(i.0), i.1)),
            )
            .finish()
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
        self.data
            .iter()
            .enumerate()
            .map(|(idx, v)| (GoodIndex { idx }, v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (GoodIndex, &mut V)> + '_ {
        self.data
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

impl<V: Default + Clone> Default for LaborMap<V> {
    fn default() -> Self {
        LaborMap {
            data: std::iter::repeat(V::default()).take(*LABOR_COUNT).collect(),
        }
    }
}

impl<V: Default + Copy + PartialEq + fmt::Debug> fmt::Debug for LaborMap<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(
                self.iter()
                    .filter(|i| *i.1 != V::default())
                    .map(|i| (i.0, i.1)),
            )
            .finish()
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
        self.data
            .iter()
            .enumerate()
            .map(|(idx, v)| (LaborIndex::from_usize(idx), v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (LaborIndex, &mut V)> + '_ {
        self.data
            .iter_mut()
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

#[derive(Debug, Default)]
pub struct AreaResources {
    pub resource_sum: GoodMap<f32>,
    pub resource_chunks: GoodMap<f32>,
    pub chunks: u32,
}

#[derive(Debug, Default)]
pub struct NaturalResources {
    // resources per distance, we should increase labor cost for far resources
    pub per_area: Vec<AreaResources>,

    // computation simplifying cached values
    pub chunks_per_resource: GoodMap<f32>,
    pub average_yield_per_chunk: GoodMap<f32>,
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

impl Labor {
    pub fn list() -> impl Iterator<Item = Self> {
        (0..LABOR.len())
            .filter(|&i| i != (DUMMY_LABOR.0 as usize))
            .map(|i| Self(i as u8, PhantomData))
    }

    pub fn list_full() -> impl Iterator<Item = Self> {
        (0..LABOR.len()).map(|i| Self(i as u8, PhantomData))
    }

    pub fn is_everyone(&self) -> bool { self.0 == DUMMY_LABOR.0 }

    pub fn orders_everyone() -> impl Iterator<Item = &'static (GoodIndex, f32)> {
        LABOR
            .get(DUMMY_LABOR.0 as usize)
            .map_or([].iter(), |l| l.orders.iter())
    }

    pub fn orders(&self) -> impl Iterator<Item = &'static (GoodIndex, f32)> {
        LABOR
            .get(self.0 as usize)
            .map_or([].iter(), |l| l.orders.iter())
    }

    pub fn products(&self) -> (GoodIndex, f32) {
        LABOR
            .get(self.0 as usize)
            .map_or((GoodIndex::default(), 0.0), |l| l.products)
    }
}
