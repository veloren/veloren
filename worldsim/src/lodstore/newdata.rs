use std::u32;
use std::collections::HashMap;
use vek::*;
use super::index::{
    self,
    LodIndex,
    AbsIndex,
    relative_to_1d,
    two_pow_u,
};
use super::area::LodArea;
use super::delta::LodDelta;
use serde::export::PhantomData;

/*
 traits and structs explained:
 - IndexStore: Every layer must implement this for either <usize, I> or <LodIndex, I>. depending on the store of the parent detail.
               It is accessed by parent layer to store the index when a detail is added or removed.
 - VecVecStore/VecHashStore/HashVecStore[HashHashStore]: Handles the different combination of layer and parent layer. Currently only 3 exist because for now there is no Hash in between the chain.
               We use this to half the number of structs we need. Also we can implement all algorithms which only requiere layer and parent layer on top of this trait.
               That reduced duplicate coding because it is used in layers as well as the leaf layers.
 - actual structs regarding of position in the chain. They represent the Layers and contain the details.
               Naming Scheme is <Own Detail Type><Parent Detail Type>[Nest]Layer
   1) VecVecLayer/VecHashLayer: Vec Leaf Layers that have a vec/hash index.
   2) VecVecNestLayer/VecHashNestLayer: Vec Layers that have a vec/hash index and are middle layers
   3) HashNoneNestLayer: Hash Layer that has no index and must be parent layer

*/

//K: Key is either usize or LodIndex
//I: Index stored, often u16 or u32
pub trait IndexStore<K, I: Copy> {
    fn load(&mut self, key: K) -> I;
    fn store(&mut self, key: K, index: I);
}
pub trait DetailStore<K2, T> {
    fn load(&mut self, key: K2) -> &T;
    fn load_mut(&mut self, key: K2) -> &mut T;
    fn store(&mut self, key: K2, detail: T);
}

pub trait Traversable<C> {
    fn get() -> C;
}
pub trait Materializeable<T> {
    fn mat() -> T;
}

struct LayerResult<'a, N: IndexStore<PK, I> + DetailStore<K, CT>, PK, I: Copy, K, CT> {
    child: &'a N,
    wanted: LodIndex,
    index: PK,
    pk_: PhantomData<K>,
    pct_: PhantomData<CT>,
    pi_: PhantomData<I>,
}

/*
impl LayerResult {
    fn get() -> LayerReslt<C::C>;
    fn mat() -> T;
}*/

pub trait Lod<T> {
    fn materialize(&self, i:LodIndex) -> &T;
}

//#######################################################

// Name <Own detail><Parent Index>
pub struct VecVecLayer<T, I: Copy, const L: u8> {
    pub detail: Vec<T>,
    pub index: Vec<I>,
}
pub struct VecHashLayer<T, I: Copy, const L: u8> {
    pub detail: Vec<T>,
    pub index: HashMap<LodIndex, I>,
}

//K: Child detail storage type usize or LodIndex
//T: own detail type
//PI: parents index type u16, u32
//CT: Child detail type
//I: own index type u16, u32
pub struct VecVecNestLayer<N: IndexStore<usize, I> + DetailStore<K, CT>, K, T, PI: Copy, CT, I: Copy, const L: u8> {
    pub detail: Vec<T>,
    pub index: Vec<PI>,
    pub nested: N,
    pk_: PhantomData<K>,
    pct_: PhantomData<CT>,
    pi_: PhantomData<I>,
}

pub struct VecHashNestLayer<N: IndexStore<usize, I> + DetailStore<K, CT>, K, T, PI: Copy, CT, I: Copy, const L: u8> {
    pub detail: Vec<T>,
    pub index: HashMap<LodIndex, PI>,
    pub nested: N,
    pk_: PhantomData<K>,
    pct_: PhantomData<CT>,
    pi_: PhantomData<I>,
}

pub struct HashNoneNestLayer<N: IndexStore<LodIndex, I> + DetailStore<K, CT>, K, T, CT, I: Copy, const L: u8> {
    pub detail: HashMap<LodIndex, T>,
    pub nested: N,
    pk_: PhantomData<K>,
    pct_: PhantomData<CT>,
    pi_: PhantomData<I>,
}

#[rustfmt::skip]
impl<T, I: Copy, const L: u8> IndexStore<usize, I> for VecVecLayer<T, I, {L}> {
    fn load(&mut self, key: usize) -> I {  *self.index.get(key).unwrap() }
    fn store(&mut self, key: usize, index: I) { self.index.insert(key, index); }
}
#[rustfmt::skip]
impl<N: IndexStore<usize, I> + DetailStore<K, CT>, K, T, PI: Copy, CT, I: Copy, const L: u8> IndexStore<usize, PI> for VecVecNestLayer<N, K, T, PI, CT, I, {L}>  {
    fn load(&mut self, key: usize) -> PI { *self.index.get(key).unwrap() }
    fn store(&mut self, key: usize, index: PI) { self.index.insert(key, index); }
}
#[rustfmt::skip]
impl<T, I: Copy, const L: u8> IndexStore<LodIndex, I> for VecHashLayer<T, I, {L}> {
    fn load(&mut self, key: LodIndex) -> I { *self.index.get(&key).unwrap() }
    fn store(&mut self, key: LodIndex, index: I) { self.index.insert(key, index); }
}
#[rustfmt::skip]
impl<N: IndexStore<usize, I> + DetailStore<K, CT>, K, T, PI: Copy, CT, I: Copy, const L: u8> IndexStore<LodIndex, PI> for VecHashNestLayer<N, K, T, PI, CT, I, {L}>  {
    fn load(&mut self, key: LodIndex) -> PI { *self.index.get(&key).unwrap() }
    fn store(&mut self, key: LodIndex, index: PI) { self.index.insert(key, index); }
}

#[rustfmt::skip]
impl<T, I: Copy, const L: u8> DetailStore<usize, T> for VecVecLayer<T, I, {L}> {
    fn load(&mut self, key: usize) -> &T {  self.detail.get(key).unwrap() }
    fn load_mut(&mut self, key: usize) -> &mut T {  self.detail.get_mut(key).unwrap() }
    fn store(&mut self, key: usize, detail: T) { self.detail.insert(key, detail); }
}
#[rustfmt::skip]
impl<N: IndexStore<usize, I> + DetailStore<K, CT>, K, T, PI: Copy, CT, I: Copy, const L: u8> DetailStore<usize, T> for VecVecNestLayer<N, K, T, PI, CT, I, {L}>  {
    fn load(&mut self, key: usize) -> &T { self.detail.get(key).unwrap() }
    fn load_mut(&mut self, key: usize) -> &mut T {  self.detail.get_mut(key).unwrap() }
    fn store(&mut self, key: usize, detail: T) { self.detail.insert(key, detail); }
}
#[rustfmt::skip]
impl<T, I: Copy, const L: u8> DetailStore<usize, T> for VecHashLayer<T, I, {L}> {
    fn load(&mut self, key: usize) -> &T { self.detail.get(key).unwrap() }
    fn load_mut(&mut self, key: usize) -> &mut T {  self.detail.get_mut(key).unwrap() }
    fn store(&mut self, key: usize, detail: T) { self.detail.insert(key, detail); }
}
#[rustfmt::skip]
impl<N: IndexStore<usize, I> + DetailStore<K, CT>, K, T, PI: Copy, CT, I: Copy, const L: u8> DetailStore<usize, T> for VecHashNestLayer<N, K, T, PI, CT, I, {L}>  {
    fn load(&mut self, key: usize) -> &T { self.detail.get(key).unwrap() }
    fn load_mut(&mut self, key: usize) -> &mut T {  self.detail.get_mut(key).unwrap() }
    fn store(&mut self, key: usize, detail: T) { self.detail.insert(key, detail); }
}
#[rustfmt::skip]
impl<N: IndexStore<LodIndex, I> + DetailStore<K, CT>, K, T, CT, I: Copy, const L: u8> DetailStore<LodIndex, T> for HashNoneNestLayer<N, K, T, CT, I, {L}>  {
    fn load(&mut self, key: LodIndex) -> &T { self.detail.get(&key).unwrap() }
    fn load_mut(&mut self, key: LodIndex) -> &mut T {  self.detail.get_mut(&key).unwrap() }
    fn store(&mut self, key: LodIndex, detail: T) { self.detail.insert(key, detail); }
}

//#######################################################

impl<N: IndexStore<usize, I> + DetailStore<K, CT>, K, T, PI: Copy, CT, I: Copy, const L: u8> VecVecNestLayer<N, K, T, PI, CT, I, {L}> {
    fn get<'a>(&'a self, index: LodIndex) -> LayerResult<'a, N, usize, I, K, CT> {
        LayerResult{
            child: &self.nested,
            wanted: index,
            index: 0,
            pk_: PhantomData,
            pct_: PhantomData,
            pi_: PhantomData,
        }
    }
}

/*
impl<T, I: Copy> Lod<T> for VecVecStore<T, I> {
    fn materialize(&self, i: LodIndex) -> &T {
        &self.detail()[0]
    }
}

impl<T, I: Copy> Lod<T> for VecHashStore<T, I> {
    fn materialize(&self, i: LodIndex) -> &T {
        &self.detail()[0]
    }
}

impl<T, I: Copy> Lod<T> for HashNoneStore<T, I> {
    fn materialize(&self, i: LodIndex) -> &T {
        &self.detail()[&i]
    }
}*/

pub type ExampleDelta =
    HashNoneNestLayer<
        VecHashNestLayer<
            VecVecNestLayer<
                VecVecLayer<
                    u16, (), 0
                > ,usize, (), u32, (), u16, 4
            > ,usize, Option<()> ,u16, (), u32, 9
        > ,usize, (), Option<()>, u16, 13
    >;

// TODO: instead of storing the absolute index in index, we store (index / number of entities), which means a u16 in Block can not only hold 2 full Subblocks (32^3 subblocks per block). but the full 2^16-1 ones.