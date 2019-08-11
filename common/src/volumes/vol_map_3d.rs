use crate::{
    vol::{BaseVol, ReadVol, SampleVol, VolSize, WriteVol},
    volumes::dyna::DynaErr,
};
use hashbrown::{hash_map, HashMap};
use std::{fmt::Debug, marker::PhantomData, sync::Arc};
use vek::*;

#[derive(Debug)]
pub enum VolMap3dErr<V: BaseVol> {
    NoSuchChunk,
    ChunkErr(V::Err),
    DynaErr(DynaErr),
    InvalidChunkSize,
}

// V = Voxel
// S = Size (replace with a const when const generics is a thing)
// M = Chunk metadata
#[derive(Clone)]
pub struct VolMap3d<V: BaseVol, S: VolSize> {
    chunks: HashMap<Vec3<i32>, Arc<V>>,
    phantom: PhantomData<S>,
}

impl<V: BaseVol, S: VolSize> VolMap3d<V, S> {
    #[inline(always)]
    pub fn chunk_key(pos: Vec3<i32>) -> Vec3<i32> {
        pos.map2(S::SIZE, |e, sz| {
            // Horrid, but it's faster than a cheetah with a red bull blood transfusion
            let log2 = (sz - 1).count_ones();
            ((((i64::from(e) + (1 << 32)) as u64) >> log2) - (1 << (32 - log2))) as i32
        })
    }

    #[inline(always)]
    pub fn chunk_offs(pos: Vec3<i32>) -> Vec3<i32> {
        pos.map2(S::SIZE, |e, sz| {
            // Horrid, but it's even faster than the aforementioned cheetah
            (((i64::from(e) + (1 << 32)) as u64) & u64::from(sz - 1)) as i32
        })
    }
}

impl<V: BaseVol + Debug, S: VolSize> BaseVol for VolMap3d<V, S> {
    type Vox = V::Vox;
    type Err = VolMap3dErr<V>;
}

impl<V: BaseVol + ReadVol + Debug, S: VolSize> ReadVol for VolMap3d<V, S> {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&V::Vox, VolMap3dErr<V>> {
        let ck = Self::chunk_key(pos);
        self.chunks
            .get(&ck)
            .ok_or(VolMap3dErr::NoSuchChunk)
            .and_then(|chunk| {
                let co = Self::chunk_offs(pos);
                chunk.get(co).map_err(VolMap3dErr::ChunkErr)
            })
    }
}

// TODO: This actually breaks the API: samples are supposed to have an offset of zero!
// TODO: Should this be changed, perhaps?
impl<I: Into<Aabb<i32>>, V: BaseVol + ReadVol + Debug, S: VolSize> SampleVol<I> for VolMap3d<V, S> {
    type Sample = VolMap3d<V, S>;

    /// Take a sample of the terrain by cloning the voxels within the provided range.
    ///
    /// Note that the resultant volume does not carry forward metadata from the original chunks.
    fn sample(&self, range: I) -> Result<Self::Sample, VolMap3dErr<V>> {
        let range = range.into();
        let mut sample = VolMap3d::new()?;
        let chunk_min = Self::chunk_key(range.min);
        let chunk_max = Self::chunk_key(range.max);
        for x in chunk_min.x..=chunk_max.x {
            for y in chunk_min.y..=chunk_max.y {
                for z in chunk_min.z..=chunk_max.z {
                    let chunk_key = Vec3::new(x, y, z);

                    let chunk = self.get_key_arc(chunk_key).cloned();

                    if let Some(chunk) = chunk {
                        sample.insert(chunk_key, chunk);
                    }
                }
            }
        }

        Ok(sample)
    }
}

impl<V: BaseVol + WriteVol + Clone + Debug, S: VolSize + Clone> WriteVol for VolMap3d<V, S> {
    #[inline(always)]
    fn set(&mut self, pos: Vec3<i32>, vox: V::Vox) -> Result<(), VolMap3dErr<V>> {
        let ck = Self::chunk_key(pos);
        self.chunks
            .get_mut(&ck)
            .ok_or(VolMap3dErr::NoSuchChunk)
            .and_then(|chunk| {
                let co = Self::chunk_offs(pos);
                Arc::make_mut(chunk)
                    .set(co, vox)
                    .map_err(VolMap3dErr::ChunkErr)
            })
    }
}

impl<V: BaseVol, S: VolSize> VolMap3d<V, S> {
    pub fn new() -> Result<Self, VolMap3dErr<V>> {
        if Self::chunk_size()
            .map(|e| e.is_power_of_two() && e > 0)
            .reduce_and()
        {
            Ok(Self {
                chunks: HashMap::new(),
                phantom: PhantomData,
            })
        } else {
            Err(VolMap3dErr::InvalidChunkSize)
        }
    }

    pub fn chunk_size() -> Vec3<u32> {
        S::SIZE
    }

    pub fn insert(&mut self, key: Vec3<i32>, chunk: Arc<V>) -> Option<Arc<V>> {
        self.chunks.insert(key, chunk)
    }

    pub fn get_key(&self, key: Vec3<i32>) -> Option<&V> {
        match self.chunks.get(&key) {
            Some(arc_chunk) => Some(arc_chunk.as_ref()),
            None => None,
        }
    }

    pub fn get_key_arc(&self, key: Vec3<i32>) -> Option<&Arc<V>> {
        self.chunks.get(&key)
    }

    pub fn remove(&mut self, key: Vec3<i32>) -> Option<Arc<V>> {
        self.chunks.remove(&key)
    }

    pub fn key_pos(&self, key: Vec3<i32>) -> Vec3<i32> {
        key * S::SIZE.map(|e| e as i32)
    }

    pub fn pos_key(&self, pos: Vec3<i32>) -> Vec3<i32> {
        Self::chunk_key(pos)
    }

    pub fn iter(&self) -> ChunkIter<V> {
        ChunkIter {
            iter: self.chunks.iter(),
        }
    }
}

pub struct ChunkIter<'a, V: BaseVol> {
    iter: hash_map::Iter<'a, Vec3<i32>, Arc<V>>,
}

impl<'a, V: BaseVol> Iterator for ChunkIter<'a, V> {
    type Item = (Vec3<i32>, &'a Arc<V>);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(k, c)| (*k, c))
    }
}
