use crate::{
    vol::{BaseVol, ReadVol, SampleVol, VolSize, WriteVol},
    volumes::dyna::DynaErr,
};
use hashbrown::{hash_map, HashMap};
use std::{fmt::Debug, marker::PhantomData, sync::Arc};
use vek::*;

#[derive(Debug, Clone)]
pub enum VolMap2dErr<V: BaseVol> {
    NoSuchChunk,
    ChunkErr(V::Err),
    DynaErr(DynaErr),
    InvalidChunkSize,
}

// V = Voxel
// S = Size (replace with a const when const generics is a thing)
// M = Chunk metadata
#[derive(Clone)]
pub struct VolMap2d<V: BaseVol, S: VolSize> {
    chunks: HashMap<Vec2<i32>, Arc<V>>,
    phantom: PhantomData<S>,
}

impl<V: BaseVol, S: VolSize> VolMap2d<V, S> {
    #[inline(always)]
    pub fn chunk_key<P: Into<Vec2<i32>>>(pos: P) -> Vec2<i32> {
        pos.into()
            .map2(S::SIZE.into(), |e, sz: u32| e >> (sz - 1).count_ones())
    }

    #[inline(always)]
    pub fn chunk_offs(pos: Vec3<i32>) -> Vec3<i32> {
        let offs = pos.map2(S::SIZE, |e, sz| e & (sz - 1) as i32);
        Vec3::new(offs.x, offs.y, pos.z)
    }
}

impl<V: BaseVol + Debug, S: VolSize> BaseVol for VolMap2d<V, S> {
    type Vox = V::Vox;
    type Err = VolMap2dErr<V>;
}

impl<V: BaseVol + ReadVol + Debug, S: VolSize> ReadVol for VolMap2d<V, S> {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&V::Vox, VolMap2dErr<V>> {
        let ck = Self::chunk_key(pos);
        self.chunks
            .get(&ck)
            .ok_or(VolMap2dErr::NoSuchChunk)
            .and_then(|chunk| {
                let co = Self::chunk_offs(pos);
                chunk.get(co).map_err(VolMap2dErr::ChunkErr)
            })
    }
}

// TODO: This actually breaks the API: samples are supposed to have an offset of zero!
// TODO: Should this be changed, perhaps?
impl<I: Into<Aabr<i32>>, V: BaseVol + ReadVol + Debug, S: VolSize> SampleVol<I> for VolMap2d<V, S> {
    type Sample = VolMap2d<V, S>;

    /// Take a sample of the terrain by cloning the voxels within the provided range.
    ///
    /// Note that the resultant volume does not carry forward metadata from the original chunks.
    fn sample(&self, range: I) -> Result<Self::Sample, VolMap2dErr<V>> {
        let range = range.into();

        let mut sample = VolMap2d::new()?;
        let chunk_min = Self::chunk_key(range.min);
        let chunk_max = Self::chunk_key(range.max);
        for x in chunk_min.x..=chunk_max.x {
            for y in chunk_min.y..=chunk_max.y {
                let chunk_key = Vec2::new(x, y);

                let chunk = self.get_key_arc(chunk_key).cloned();

                if let Some(chunk) = chunk {
                    sample.insert(chunk_key, chunk);
                }
            }
        }

        Ok(sample)
    }
}

impl<V: BaseVol + WriteVol + Clone + Debug, S: VolSize + Clone> WriteVol for VolMap2d<V, S> {
    #[inline(always)]
    fn set(&mut self, pos: Vec3<i32>, vox: V::Vox) -> Result<(), VolMap2dErr<V>> {
        let ck = Self::chunk_key(pos);
        self.chunks
            .get_mut(&ck)
            .ok_or(VolMap2dErr::NoSuchChunk)
            .and_then(|chunk| {
                let co = Self::chunk_offs(pos);
                Arc::make_mut(chunk)
                    .set(co, vox)
                    .map_err(VolMap2dErr::ChunkErr)
            })
    }
}

impl<V: BaseVol, S: VolSize> VolMap2d<V, S> {
    pub fn new() -> Result<Self, VolMap2dErr<V>> {
        if Self::chunk_size()
            .map(|e| e.is_power_of_two() && e > 0)
            .reduce_and()
        {
            Ok(Self {
                chunks: HashMap::default(),
                phantom: PhantomData,
            })
        } else {
            Err(VolMap2dErr::InvalidChunkSize)
        }
    }

    pub fn chunk_size() -> Vec2<u32> {
        S::SIZE.into()
    }

    pub fn insert(&mut self, key: Vec2<i32>, chunk: Arc<V>) -> Option<Arc<V>> {
        self.chunks.insert(key, chunk)
    }

    pub fn get_key(&self, key: Vec2<i32>) -> Option<&V> {
        match self.chunks.get(&key) {
            Some(arc_chunk) => Some(arc_chunk.as_ref()),
            None => None,
        }
    }

    pub fn get_key_arc(&self, key: Vec2<i32>) -> Option<&Arc<V>> {
        self.chunks.get(&key)
    }

    pub fn clear(&mut self) {
        self.chunks.clear();
    }

    pub fn drain(&mut self) -> hash_map::Drain<Vec2<i32>, Arc<V>> {
        self.chunks.drain()
    }

    pub fn remove(&mut self, key: Vec2<i32>) -> Option<Arc<V>> {
        self.chunks.remove(&key)
    }

    pub fn key_pos(&self, key: Vec2<i32>) -> Vec2<i32> {
        key * Vec2::<u32>::from(S::SIZE).map(|e| e as i32)
    }

    pub fn pos_key(&self, pos: Vec3<i32>) -> Vec2<i32> {
        Self::chunk_key(pos)
    }

    pub fn iter(&self) -> ChunkIter<V> {
        ChunkIter {
            iter: self.chunks.iter(),
        }
    }
}

pub struct ChunkIter<'a, V: BaseVol> {
    iter: hash_map::Iter<'a, Vec2<i32>, Arc<V>>,
}

impl<'a, V: BaseVol> Iterator for ChunkIter<'a, V> {
    type Item = (Vec2<i32>, &'a Arc<V>);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(k, c)| (*k, c))
    }
}
