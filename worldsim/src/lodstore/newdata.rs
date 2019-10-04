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
 - IndexStore: Every detail must implement this for either <usize, I> or <LodIndex, I>. depending on the store of the parent detail.
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
pub trait DatailStore<K2, T> {
    fn load(&mut self, key: K2) -> &T;
    fn load_mut(&mut self, key: K2) -> &mut T;
    fn store(&mut self, key: K2, index: T);
}

// Algorithms for a Layer are impemented based on these traits
pub trait VecVecStore<T, I: Copy> {
    fn detail(&self) -> &Vec<T>;
    fn detail_mut(&mut self) -> &mut Vec<T>;
    fn index(&self) -> &Vec<I>;
    fn index_mut(&mut self) -> &mut Vec<I>;
}
pub trait VecHashStore<T, I: Copy> {
    fn detail(&self) -> &Vec<T>;
    fn detail_mut(&mut self) -> &mut Vec<T>;
    fn index(&self) -> &HashMap<LodIndex, I>;
    fn index_mut(&mut self) -> &mut HashMap<LodIndex, I>;
}
pub trait HashNoneStore<T, I: Copy> {
    fn detail(&self) -> &HashMap<LodIndex, T>;
    fn detail_mut(&mut self) -> &mut HashMap<LodIndex, T>;
   // fn get(&self, index: LodIndex) -> (&Child, index: LodIndex, usize/*shouldnt this be based on child ?*/)
}

pub trait Traversable<C> {
    fn get() -> C;
}
pub trait Materializeable<T> {
    fn mat() -> T;
}

/*
struct LayerResult<C> {
    child: &C,
    index: LodIndex,
    C::Type,
}

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

pub struct VecVecNestLayer<N: IndexStore<K, I>, T, K, I: Copy, const L: u8> {
    pub detail: Vec<T>,
    pub index: Vec<I>,
    pub nested: N,
    pk_: PhantomData<K>,
}

pub struct VecHashNestLayer<N: IndexStore<K, I>, T, K, I: Copy, const L: u8> {
    pub detail: Vec<T>,
    pub index: HashMap<LodIndex, I>,
    pub nested: N,
    pk_: PhantomData<K>,
}

pub struct HashNoneNestLayer<N: IndexStore<K, I>, T, K, I: Copy, const L: u8> {
    pub detail: HashMap<LodIndex, T>,
    pub nested: N,
    pk_: PhantomData<K>,
    pi_: PhantomData<I>,
}

impl<T, I: Copy, const L: u8> VecVecStore<T, I> for VecVecLayer<T, I, {L}> {
    fn detail(&self) -> &Vec<T> {&self.detail}
    fn detail_mut(&mut self) -> &mut Vec<T> {&mut self.detail}
    fn index(&self) -> &Vec<I> {&self.index}
    fn index_mut(&mut self) -> &mut Vec<I> {&mut self.index}
}

impl<N: IndexStore<K, I>, T, K, I: Copy, const L: u8> VecVecStore<T, I> for VecVecNestLayer<N, T, K, I, {L}> {
    fn detail(&self) -> &Vec<T> {&self.detail}
    fn detail_mut(&mut self) -> &mut Vec<T> {&mut self.detail}
    fn index(&self) -> &Vec<I> {&self.index}
    fn index_mut(&mut self) -> &mut Vec<I> {&mut self.index}
}

impl<T, I: Copy, const L: u8> VecHashStore<T, I> for VecHashLayer<T, I, {L}> {
    fn detail(&self) -> &Vec<T> {&self.detail}
    fn detail_mut(&mut self) -> &mut Vec<T> {&mut self.detail}
    fn index(&self) -> &HashMap<LodIndex, I> {&self.index}
    fn index_mut(&mut self) -> &mut HashMap<LodIndex, I> {&mut self.index}
}

impl<N: IndexStore<K, I>, T, K, I: Copy, const L: u8> VecHashStore<T, I> for VecHashNestLayer<N, T, K, I, {L}> {
    fn detail(&self) -> &Vec<T> {&self.detail}
    fn detail_mut(&mut self) -> &mut Vec<T> {&mut self.detail}
    fn index(&self) -> &HashMap<LodIndex, I> {&self.index}
    fn index_mut(&mut self) -> &mut HashMap<LodIndex, I> {&mut self.index}
}

impl<N: IndexStore<K, I>, T, K, I: Copy, const L: u8> HashNoneStore<T, I> for HashNoneNestLayer<N, T, K, I, {L}> {
    fn detail(&self) -> &HashMap<LodIndex, T> {&self.detail}
    fn detail_mut(&mut self) -> &mut HashMap<LodIndex, T> {&mut self.detail}
}

//#######################################################

impl<T, I: Copy> IndexStore<usize, I> for VecVecStore<T, I> {
    fn load(&mut self, key: usize) -> I {
        *self.index().get(key).unwrap()
    }
    fn store(&mut self, key: usize, index: I) {
        self.index_mut().insert(key, index);
    }
}

impl<T, I: Copy> IndexStore<LodIndex, I> for VecHashStore<T, I> {
    fn load(&mut self, key: LodIndex) -> I {
        *self.index().get(&key).unwrap()
    }
    fn store(&mut self, key: LodIndex, index: I) {
        self.index_mut().insert(key, index);
    }
}

impl<T, I: Copy> IndexStore<usize, I> for HashNoneStore<T, I> {
    fn load(&mut self, key: usize) -> I {
        unimplemented!()
    }
    fn store(&mut self, key: usize, index: I) {
        unimplemented!()
    }
}


//#######################################################

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
}

pub type ExampleDelta =
    HashNoneNestLayer<
        VecHashNestLayer<
            VecVecNestLayer<
                VecVecLayer<
                    () ,u16 , 0
                > ,() ,usize ,u32 ,4
            > ,() ,usize ,u16 ,9
        > ,() ,usize ,u16 ,13
    >;

// TODO: instead of storing the absolute index in index, we store (index / number of entities), which means a u16 in Block can not only hold 2 full Subblocks (32^3 subblocks per block). but the full 2^16-1 ones.